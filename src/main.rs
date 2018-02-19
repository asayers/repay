extern crate clap;
extern crate env_logger;
extern crate itertools;
#[macro_use] extern crate log;
extern crate mcmf;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use itertools::Itertools;
use mcmf::*;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufRead};

fn main() {
    // Parse the command-line arguments
    let opts = clap::App::new("debtor").version("1.0")
        .args_from_usage(
            "<PATH>         'The ledger containing historical transactions'
             -q --quiet     'Don't print anything unless there's a problem'
             -v...          'Increase the level of verbosity'")
        .get_matches();

    // Initialise the logger (prints to stderr)
    let log_level = match opts.occurrences_of("v") {
        _ if opts.is_present("q") => log::LevelFilter::Error,
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        2 | _ => log::LevelFilter::Trace,
    };
    env_logger::Builder::new().filter(None, log_level).init();

    let ledger_path = opts.value_of("PATH").unwrap();
    let ledger_file = File::open(ledger_path).unwrap();
    let ledger_iter = BufReader::new(ledger_file)
        .lines()
        .map(|l| serde_json::from_str::<Transfer>(&l.unwrap()).unwrap());

    for p in compute_repayments(ledger_iter) {
        println!("{}", serde_json::to_string(&p).unwrap());
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Transfer {
    from: String,
    to: String,
    amt: usize,
}

fn compute_repayments<I: IntoIterator<Item = Transfer>>(ledger: I) -> Vec<Transfer> {
    // Step 1: Compute everyone's balances (starting from 0)
    info!("Reading ledger...");
    let mut n = 0;
    let mut balances = BTreeMap::new();
    for transfer in ledger {
        {
        let from = balances.entry(transfer.from).or_insert(0);
        *from -= transfer.amt as isize;
        }
        let to = balances.entry(transfer.to).or_insert(0);
        *to += transfer.amt as isize;
        n += 1;
    }
    info!("Read {} entries.", n);

    // (Step 1.5: Set up a fully-connected graph with one node per person)
    info!("Setting up graph...");
    let mut graph = GraphBuilder::new();
    let people = balances.keys();
    for (x, y) in people.clone().cartesian_product(people) {   // unnecessary clone
        if x != y {
            graph.add_edge(x.clone(), y.clone(), Capacity(1_000_000), Cost(1));
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
    info!("Total repayable: {}", cost);

    // (Step 2.5: Wrangle these flows back into the shape of Tranfers)
    info!("Assembling repayments...");
    let mut repayments = vec![];
    for mut p in paths {
        // let msg = "Graph is strongly connected => all flows are length 1";
        for Flow { a, b, amount, .. } in p.flows {
            if let Vertex::Node(a) = a {
                if let Vertex::Node(b) = b {
                    repayments.push(Transfer {
                        from: a,
                        to: b,
                        amt: amount as usize,
                    });
                }
            }
        }
    }
    repayments
}
