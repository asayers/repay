extern crate bitset64;
extern crate clap;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate mcmf;
extern crate mzsp;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use mcmf::*;
use mzsp::MZSP;
use std::collections::BTreeMap;
use std::fs::File;

fn main() {
    // Parse the command-line arguments
    let opts = clap::App::new("debtor").version("1.0")
        .args_from_usage(
            "<PATH>         'The ledger containing historical transactions'
             -a, --approx   'Guarantee a fast solution (may be suboptimal)'
             -x, --exact    'Guarantee an exact solution (may be slow)'
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
    let mut n = 0;
    let mut balances = BTreeMap::new();
    let ts = ::std::time::Instant::now();
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
    let balances: Vec<(String, isize)> = balances.into_iter().filter(|&(_,x)| x != 0).collect();
    let ts = ts.elapsed();
    info!("Read {} entries from {} in {}.{:0>3}s", n, ledger_path, ts.as_secs(), ts.subsec_nanos()/1_000_000);
    info!("{} unresolved balances, {} to repay", balances.len(), balances.iter().map(|&(_,x)|x.abs()).sum::<isize>());

    let ts = ::std::time::Instant::now();
    let plan = match (opts.is_present("x"), opts.is_present("a"), balances.len() <= 20) {
        (true, true, _) => panic!("User specified exact mode *and* approximate mode!"),
        (true, false, _) => compute_repayments_exact(balances),      // -x was specified
        (false, true, _) => compute_repayments_approx(balances),     // -a was specified
        (false, false, true) => compute_repayments_exact(balances),  // n is small
        (false, false, false) => {                                   // n is big
            warn!("The following solution may be approximate.  (Use '-x' to force exact mode)");
            compute_repayments_approx(balances)
        }
    };
    let ts = ts.elapsed();
    info!("Computed repayment plan in {}.{:0>3}s", ts.as_secs(), ts.subsec_nanos()/1_000_000);
    info!("{} repayments required", plan.len());
    for mut p in plan {
        p.normalise();
        println!("{}", serde_json::to_string(&p).unwrap());
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

fn compute_repayments_exact(balances: Vec<(String, isize)>) -> Vec<Transfer<String>> {
    if balances.len() >= 64 {
        error!("Exact mode doesn't support ledgers with more than 64 unsettled \
            balances.  Please use approximate mode instead.");
        ::std::process::exit(1);
    }
    // Get the data into the right form (TODO: eliminate this)
    let values: Vec<isize> = balances.iter().map(|x|x.1).collect();

    // Compute the largest set of zero-sum paritions
    let parts = MZSP::compute(&values);
    info!("Divided into {} partitions", parts.len());
    parts.flat_map(|partition| {
        let balances: Vec<(String,isize)> = partition.elements()
            .map(|idx| balances[idx as usize].clone())
            .collect();
        // For each partition, construct a plan.  We know that these partitions contain no zero-sum
        // subsets, so `construct_plan` is optimal.
        construct_plan(balances)
    }).collect()
}

/// Given a zero-sum set of nodes, construct a graph which moves all the value from the positive
/// nodes to the negative nodes.  This function is *O(n)*, but the graph will be maximally
/// inefficient, in the sense that it will always contain exactly *n* edges.  If the given set of
/// nodes contains zero-sum subsets then we can do better.
// TODO: Use a priority search queue
fn construct_plan<T: Clone>(mut balances: Vec<(T, isize)>) -> Vec<Transfer<T>> {
    assert_eq!(balances.iter().map(|x|x.1).sum::<isize>(), 0, "balances must be zero-sum");
    let mut ret = vec![];
    loop {
        // Take the node with the smallest absolute value;  this will be our "from" node.
        balances.sort_unstable_by_key(|&(_, x)| -x.abs());
        let (from_tag, from_val) = match balances.pop() { None => break, Some(x) => x };
        if from_val == 0 { continue; }
        // Find a node with the opposite sign (any will do);  this will be our "to" node.
        let to = balances.iter_mut().find(|x| x.1.signum() != from_val.signum())
            .expect("a node with opposite sign");  // The partition is zero-sum => it must exist
        let to_tag = to.0.clone();
        to.1 += from_val;  // Eliminate the "from" node with the "to" node.
        // There's no need to remove zero-balance "to" nodes;  this will only occur for the very
        // last node.
        ret.push(Transfer { from: from_tag, to: to_tag, amt: from_val });
    }
    ret
}

fn compute_repayments_approx(balances: Vec<(String, isize)>) -> Vec<Transfer<String>> {
    // (Step 1.5: Set up a fully-connected graph with one node per person)
    let mut graph = GraphBuilder::new();
    for &(ref x,_) in balances.iter() {
        for &(ref y,_) in balances.iter() {
            if x != y {
                graph.add_edge(x.clone(), y.clone(), Capacity(1_000_000_000), Cost(1));
            }
        }
    }

    // Step 2: Figure out how to shift money around to make all the balances go back to 0
    for (client, balance) in balances {
        if balance > 0 {
            graph.add_edge(Vertex::Source, client, Capacity(balance.abs() as u32), Cost(0));
        } else if balance < 0 {
            graph.add_edge(client, Vertex::Sink, Capacity(balance.abs() as u32), Cost(0));
        } else {
            error!("Got a zero node");
        }
    }
    let (cost, paths) = graph.mcmf();
    info!("Total flow: {}", cost);

    // (Step 2.5: Wrangle these flows back into the shape of Tranfers)
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
