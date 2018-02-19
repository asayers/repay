extern crate mcmf;
extern crate itertools;

use mcmf::*;
use std::collections::BTreeMap;
use itertools::Itertools;

fn main() {
    let payments = vec![
        Transfer{ from: "akio",    to: "alex",    amt: 100, reason: "foo".into() },
        Transfer{ from: "alex",    to: "fumiaki", amt: 200, reason: "foo".into() },
        Transfer{ from: "fumiaki", to: "akio",    amt: 300, reason: "foo".into() },
    ];
    let repayments = repayments(payments);
    println!("{:?}", repayments);
}

#[derive(Debug)]
struct Transfer<T> {
    from: T,
    to: T,
    amt: usize,
    reason: String,
}

fn repayments<T: Ord + Clone>(ledger: Vec<Transfer<T>>) -> Vec<Transfer<T>> {
    // Step 1: Compute everyone's balances (starting from 0)
    let mut balances = BTreeMap::new();
    for transfer in ledger {
        {
        let from = balances.entry(transfer.from).or_insert(0);
        *from -= transfer.amt as isize;
        }
        let to = balances.entry(transfer.to).or_insert(0);
        *to += transfer.amt as isize;
    }

    // (Step 1.5: Set up a fully-connected graph with one node per person)
    let mut graph = GraphBuilder::new();
    let people = balances.keys();
    for (x, y) in people.clone().cartesian_product(people) {   // unnecessary clone
        if x != y {
            graph.add_edge(x.clone(), y.clone(), Capacity(1_000_000), Cost(1));
        }
    }

    // Step 2: Figure out how to shift money around to make all the balances go back to 0
    for (client, balance) in balances.iter() {
        if *balance > 0 {
            graph.add_edge(Vertex::Source, client.clone(), Capacity(balance.abs() as u32), Cost(0));
        }
        if *balance < 0 {
            graph.add_edge(client.clone(), Vertex::Sink, Capacity(balance.abs() as u32), Cost(0));
        }
    }
    let (_, paths) = graph.mcmf();

    // (Step 2.5: Wrangle these flows back into the shape of Tranfers)
    let mut repayments = vec![];
    for mut p in paths {
        let msg = "Graph is strongly connected => all flows are length 1";
        assert!(p.flows.pop().unwrap().b == Vertex::Sink, msg);
        let Flow { a, b, amount, .. } = p.flows.pop().unwrap();
        assert!(p.flows.pop().unwrap().a == Vertex::Source, msg);
        repayments.push(Transfer {
            from: a.as_option().unwrap(),
            to: b.as_option().unwrap(),
            amt: amount as usize,
            reason: "Settlement".into(),
        });
    }
    repayments
}
