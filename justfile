dev:
    cargo watch -c -w src -w build.rs -w .env -w Cargo.toml -i *.j2 -i *.css -i *.ts -x run

nats:
    nats-server --jetstream --store_dir=data --name=test_server

test name="":
    cargo watch -c -w src -w .env -w Cargo.toml -x "test {{ name }} -- --no-capture --color=always"

check:
    cargo clippy --release

build:
    cargo build --release

run:
    cargo run --release

loc:
    cloc src public build.rs