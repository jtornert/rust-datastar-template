dev:
    watchexec -c -r -w src -w build.rs -w .env -w Cargo.toml --exts=rs,.toml,.env cargo run --features dev

nats:
    nats-server --jetstream --store_dir=data --name=test_server

test name="":
    watchexec -c -w src -w .env -w Cargo.toml cargo test {{ name }} -- --no-capture --color=always

check:
    cargo clippy --release

build:
    cargo build --release

run:
    cargo run --release

loc:
    tokei src public build.rs