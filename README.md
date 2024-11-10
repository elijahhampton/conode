# CoNode

CoNode is a library and p2p node designed to be used with libp2p and the Conode protocol that enables peer-to-peer work relationships without intermediaries. It provides a trustless platform where employers can broadcast work opportunities and interact directly with workers through a decentralized network.

Run a node to participate in the network and interact using the iced interface found in crates/conode-cli.

## Features

- **Decentralized Job Discovery**: Employers can broadcast work opportunities across the P2P network
- **Direct P2P Negotiation**: Workers and employers can negotiate terms directly through secure channels
- **Built for Scale**: Uses libp2p for robust peer discovery and communication

## Quick Start

### Prerequisites

- Rust 1.75+
- Cargo
- [Starknet CLI](https://docs.starknet.io/documentation/getting_started/)
- [Starknet-Devnet] (https://github.com/0xSpaceShard/starknet-devnet-rs)

### Installation

```bash
# Clone the repository
git clone https://github.com/elijahhampton/conode.git
cd conode

# Build the project
cargo build --release

# Run the CLI interface
CONODE_CONFIG=/path/to/config.toml cargo run -p conode-cli

# Run the CLI interface with less verbosity
RUST_LOG=info,iced=off,iced_native=off,iced_futures=off,winit=off CONODE_CONFIG=/path/to/config.toml cargo run -p conode-cli

```

## Usage

### As an Employer

```rust
use conode_protocol::LaborMarketNode;

#[tokio::main]
async fn main() {
   // Create a new node
   let node = LaborMarketNode::new().await?;

   // Broadcast a work opportunity
   node.broadcast_work(work_details).await?;

   // Listen for and handle worker proposals
   node.run().await?;
}
```

## Usage

### As a Worker

```rust
use conode_protocol::LaborMarketNode;

#[tokio::main]
async fn main() {
   // Create a new node
    let node = LaborMarketNode::new().await?;

    // Subscribe to work opportunities
    node.subscribe_to_work().await?;

    // Listen for work broadcasts and submit proposals
    node.run().await?;
}
```

## Development Setup

### Prerequisites

- Rust 1.75+
- Cargo
- [Starknet CLI](https://docs.starknet.io/documentation/getting_started/)

### Configuration

CoNode requires a configuration file specified via the `CONODE_CONFIG` environment variable. Here's a breakdown of the available settings:

Basic Configuration

````toml
# General Settings
data_dir = "/path/to/data"    # RocksDB directory
log_level = "info"            # Node logging level (debug, info, warn, error)

# RPC Settings
[rpc]
rpc = "http://127.0.0.1:5050"  # Starknet RPC endpoint
chain_id = "SN_GOERLI"         # Starknet chain ID

# Account Settings
[account]
mnemonic = ""  # Optional: Known mnemonic for account recovery

# Contract Settings
[contract]
payment_token = "0x123..."  # Payment token contract address
conode = "0x456..."        # CoNode protocol contract address
```

# Run all tests

cargo test

# Run specific crate tests

cargo test -p conode-protocol
cargo test -p conode-network

# Run with logging

RUST_LOG=debug cargo test
````
