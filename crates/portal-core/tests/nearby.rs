//! Every computer running SeamDrop nearby shows up (Mission 05 §1.8): three
//! cores on one machine, nothing paired, each finding the other two by the
//! unicast sweep (the same packets as the UDP broadcast, sent to each
//! other's ports, as on a network that passes no multicast).

mod common;

use std::time::{Duration, Instant};

use common::{free_udp_port, start};

#[tokio::test]
async fn three_computers_with_nothing_paired_all_see_each_other() {
    let ports = [free_udp_port(), free_udp_port(), free_udp_port()];
    let names = ["Den_PC", "macbook one", "Sam’s MacBook Air"];
    let mut nodes = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let (me, others) = (ports[i], ports.iter().filter(|p| **p != ports[i]).copied().collect::<Vec<_>>());
        nodes.push(
            start(name, move |c| {
                c.broadcast = true;
                c.udp_port = me;
                c.extra_targets = others.iter().map(|p| format!("127.0.0.1:{p}").parse().unwrap()).collect();
            })
            .await,
        );
    }
    let began = Instant::now();
    for (i, node) in nodes.iter_mut().enumerate() {
        let others: Vec<&str> = names.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, n)| *n).collect();
        let s = node
            .wait_for_within(Duration::from_secs(5), "both others nearby", |s| {
                others.iter().all(|n| s.peers.iter().any(|p| p.name == *n && !p.paired))
            })
            .await;
        assert!(s.peers.iter().all(|p| !p.paired), "nothing paired");
    }
    eprintln!("three computers saw each other within {:?}", began.elapsed());
}
