# nbt-rust

`nbt-rust` is a simple NBT (Named Binary Tag) library written in Rust.

## Features

- Big Endian, Little Endian, and Network Little Endian support
- Low-level `Tag`-based API
- Typed encode/decode with `serde`
- Helper functions for Bedrock-compatible NBT flows

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nbt-rust = "0.1.1"
```

## Quick Usage

```rust
use nbt_rust::{from_net_bytes, to_net_bytes_named, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PlayerData {
    username: String,
    hp: i32,
}

fn main() -> Result<()> {
    let input = PlayerData {
        username: "Steve".to_string(),
        hp: 20,
    };

    let bytes = to_net_bytes_named("", &input)?;
    let output: PlayerData = from_net_bytes(&bytes)?;

    assert_eq!(input, output);
    Ok(())
}
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -q
```
