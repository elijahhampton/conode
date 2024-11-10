use libp2p::PeerId;
use once_cell::sync::Lazy;

pub struct BootstrapNode {
    pub addr: String,
    pub peer_id: PeerId,
}

pub static BOOTSTRAP_NODES: Lazy<Vec<BootstrapNode>> = Lazy::new(|| {
    vec![BootstrapNode {
        addr: "/ip4/203.0.113.1/tcp/4001".to_string(),
        peer_id: "12D3KooWKEMc1UvVsyZ19cEr18V6AnFqoWyHKC8ikrCgzG83gZHw"
            .parse()
            .expect("Invalid peer id"),
    }]
});
