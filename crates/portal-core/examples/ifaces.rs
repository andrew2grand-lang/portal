//! Prints what netdev sees on this machine (handy when debugging link detection).
fn main() {
    for i in netdev::get_interfaces() {
        println!(
            "{:>3} {:<10} {:<22} type={:?} up={} default={} gw={} v4={:?} v6={:?} scopes={:?}",
            i.index,
            i.name,
            i.friendly_name.clone().unwrap_or_default(),
            i.if_type,
            i.is_up(),
            i.default,
            i.gateway.is_some(),
            i.ipv4.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
            i.ipv6.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
            i.ipv6_scope_ids,
        );
    }
}
