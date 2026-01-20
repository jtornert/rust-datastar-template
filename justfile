dev:
    cargo watch -c -w src -w build.rs -w .env -w Cargo.toml -i *.j2 -i *.css -i *.ts -x run

nats:
    nats-server --jetstream --store_dir=data --name=test_server

test:
    cargo test

loc:
    cloc src public build.rs