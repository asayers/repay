extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate mcmf;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

mod stack;

use mcmf::*;
use std::collections::BTreeMap;
use std::fs::File;
use stack::Stack;

fn main() {
    // Parse the command-line arguments
    let opts = clap::App::new("debtor").version("1.0")
        .args_from_usage(
            "<PATH>         'The ledger containing historical transactions'
             -x             'Guarantee an exact solution (may be slow)'
             -v...          'Increase the level of verbosity'")
        .get_matches();

    // Initialise the logger (prints to stderr)
    let log_level = match opts.occurrences_of("v") {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 | _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::new().filter(None, log_level).init();

    // Step 1: Parse the ledger (JSON)
    let ledger_path = opts.value_of("PATH").unwrap();
    let ledger_file = File::open(ledger_path).unwrap();
    let ledger_iter = serde_json::Deserializer::from_reader(ledger_file)
        .into_iter().map(|x| x.expect("Deserialise line"));

    // Step 2: Compute everyone's balances (starting from 0)
    info!("Reading ledger...");
    let mut n = 0;
    let mut balances = BTreeMap::new();
    for transfer in ledger_iter {
        let transfer: Transfer<String> = transfer;  // FIXME: dumb
        {
        let from = balances.entry(transfer.from).or_insert(0);
        *from -= transfer.amt;
        }
        let to = balances.entry(transfer.to).or_insert(0);
        *to += transfer.amt;
        n += 1;
    }
    info!("Done! Read {} entries", n);
    debug!("{:?}", balances);

    if opts.is_present("x") {
        for p in compute_repayments_exact(balances) {
            println!("{}", serde_json::to_string(&p).unwrap());
        }
    } else {
        for p in compute_repayments_approx(balances) {
            println!("{}", serde_json::to_string(&p).unwrap());
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Transfer<T> {
    from: T,
    to: T,
    amt: isize,  // TODO: Change to f64, multiply by 100 for approx
}

impl<T> Transfer<T> {
    fn normalise(&mut self) {
        if self.amt < 0 {
            ::std::mem::swap(&mut self.from, &mut self.to);
            self.amt = -self.amt;
        }
    }
}

fn compute_repayments_exact(balances: BTreeMap<String, isize>) -> Vec<Transfer<String>> {
    // Step 3: Order people from smallest to largest absolute balance
    let mut best = (usize::max_value(), Stack::new());

    let mut balance_refs: Vec<(&str, isize)> = Vec::new();
    for (ref s, x) in balances.iter() {
        balance_refs.push((&s, *x));
    }
    balance_refs.sort_unstable_by_key(|&(_, x)| x.abs());

    search_tree(&mut best, Stack::new(), &balance_refs);

    let mut ret = vec![];
    for rc in best.1.iter() {
        let mut transfer = Transfer {
            from: (*rc).1.from.to_owned(),
            to: (*rc).1.to.to_owned(),
            amt: (*rc).1.amt,
        };
        transfer.normalise();
        ret.push(transfer);
    }
    ret
}

// TODO: Incremental deepening, starting at max{m,n}
fn search_tree<'a, 'b>(best: &'b mut (usize, Stack<Transfer<&'a str>>), stack: Stack<Transfer<&'a str>>, remaining: &[(&'a str, isize)]) {
    match remaining.split_first() {
        None => {
            debug!("LEAF!");
            if stack.len() < best.0 {
                debug!("it's good!");
                *best = (stack.len(), stack);
            }
        }
        Some((head, tail)) => {
            debug!(">> {}: smallest node: {:?}, ({:?})", stack.len(), head, tail);
            let mut matches = false;
            // TODO: check for exact matches, skip other branches if there are any
            for (x, i) in tail.iter().zip(0..) {
                debug!("try eliminating with {:?}?", x);
                if x.1.signum() == head.1.signum() { continue; }
                matches = true;
                let mut next = Vec::from(tail);
                next[i].1 += head.1;
                if next[i].1 == 0 {
                    next.remove(i);
                }
                next.sort_unstable_by_key(|&(_, x)| x.abs());
                let t = Transfer {
                    from: head.0,
                    to: x.0,
                    amt: head.1,
                };
                search_tree(best, stack.push(t), &next);
            }
            assert!(matches);
        }
    }
}

fn compute_repayments_approx(balances: BTreeMap<String, isize>) -> Vec<Transfer<String>> {
    // (Step 1.5: Set up a fully-connected graph with one node per person)
    info!("Setting up graph...");
    let mut graph = GraphBuilder::new();
    for x in balances.keys() {
        for y in balances.keys() {
            if x != y {
                graph.add_edge(x.clone(), y.clone(), Capacity(1_000_000_000), Cost(1));
            }
        }
    }

    // Step 2: Figure out how to shift money around to make all the balances go back to 0
    info!("Computing minimum-cost flow...");
    for (client, balance) in balances.iter() {
        if *balance > 0 {
            graph.add_edge(Vertex::Source, client.clone(), Capacity(balance.abs() as u32), Cost(0));
        }
        if *balance < 0 {
            graph.add_edge(client.clone(), Vertex::Sink, Capacity(balance.abs() as u32), Cost(0));
        }
    }
    let (cost, paths) = graph.mcmf();
    info!("Done! Total repayable: {}", cost);

    // (Step 2.5: Wrangle these flows back into the shape of Tranfers)
    info!("Assembling repayments...");
    let mut repayments = vec![];
    for mut p in paths {
        if p.flows.len() != 3 {
            // Graph is strongly connected => all flows should have length 1
            warn!("Maximum transfer amount exceeded.  Repaying via a different route...");
        }
        for Flow { a, b, amount, .. } in p.flows {
            if let Vertex::Node(a) = a {
                if let Vertex::Node(b) = b {
                    repayments.push(Transfer {
                        from: a,
                        to: b,
                        amt: amount as isize,
                    });
                }
            }
        }
    }
    repayments
}
