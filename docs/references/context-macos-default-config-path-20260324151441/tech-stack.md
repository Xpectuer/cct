# Tech Stack Snapshot

Detected tech stack files:

- `Cargo.toml`

```toml
[package]
name = "cct"
version = "0.1.0"
edition = "2021"

[lib]
name = "cct"
path = "src/lib.rs"

[[bin]]
name = "cct"
path = "src/main.rs"

[dependencies]
clap      = { version = "4", features = ["derive"] }
ratatui   = "0.29"
crossterm = "0.28"
serde     = { version = "1", features = ["derive"] }
toml      = "0.8"
toml_edit = "0.22"
dirs      = "5"
anyhow    = "1"

[dev-dependencies]
tempfile = "3"
serial_test = "3"
```
