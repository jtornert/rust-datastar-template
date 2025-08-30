set dotenv-path := ".env"
set dotenv-load := true

dev:
    cargo watch -c -w src -w sql -x run

test name="":
    RUST_LOG=server=debug cargo watch -c -w src -w sql -w templates -x "test {{ name }}"

check:
    cargo clippy --release

build:
    cargo build --release

run: build
    ./target/release/web-server

example name:
    cargo watch -cq -w examples -x "run --example {{ name }}"

loc:
    cloc assets events locales packages/**/src sql src templates

format:
    find src -type f -name "*.rs" -exec rustfmt --edition 2024 {} \;

pd:
    pd-server --name=pd1 --data-dir=data/pd1 --client-urls="http://$PD_SERVER_URL"

tikv:
    tikv-server --pd-endpoints="$PD_SERVER_URL" --addr="$TIKV_SERVER_URL" --data-dir=data/tikv1

db log="info":
    RUST_LOG={{ log }} surreal start --user $DB_USERNAME --pass $DB_PASSWORD tikv://$PD_SERVER_URL

migrate:
    RUST_LOG=surreal::cli::import=info find sql -type f -name "*.surql" -not -path "sql/dev/*.surql" -exec surreal import -e $DB_URL --user $DB_USERNAME --pass $DB_PASSWORD --namespace $DB_NAMESPACE --database $DB_DATABASE {} \;

data name:
    RUST_LOG=surreal::cli::import=info surreal import -e $DB_URL --user $DB_USERNAME --pass $DB_PASSWORD --namespace $DB_NAMESPACE --database $DB_DATABASE "./sql/dev/{{ name }}.surql"

sql:
    RUST_LOG=info surreal sql -e $DB_URL --user $DB_USERNAME --pass $DB_PASSWORD --namespace $DB_NAMESPACE --database $DB_DATABASE --pretty

setup:
    curl -sSf https://install.surrealdb.com | sh
    curl -sSf https://tiup-mirrors.pingcap.com/install.sh | sh
    cargo install cargo-watch

clean:
    rm -rf data target history.txt
