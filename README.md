# rust-datastar-template

First time setup:

```bash
just setup # installs SurrealDB, tiup, and cargo-watch
```

Create `.env` from `.env.example` and run:

```bash
just keygen
```

Copy the key into the `KEY` environment variable.

Start development server (in memory):

```bash
just dev
```

Running the tikv cluster requires the tikv `DB_URL` environment variable. Start tikv cluster and development server (persisted):

```bash
just pd # terminal 1
just tikv # terminal 2
just db # terminal 3
just migrate # terminal 4, imports the database schema
just data <name> # terminal 4, imports any .surql file in the sql/dev folder
just sql # terminal 4, connects to SurrealDB to run SQL statements
just dev # terminal 4
```

Run tests:

```bash
just test # run all tests
just test <name> # run specific test matching <name>
```

Check for release:

```bash
just check
```

Build server for release:

```bash
just build
```

Clean up everything:

```bash
just clean
```
