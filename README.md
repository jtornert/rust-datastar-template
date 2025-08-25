# rust-datastar-template

First time setup:

```bash
just setup
```

Create `.env` from `.env.example` and run:

```bash
just example keygen
```

Copy the key into the `KEY` environment variable.

Start development server (in memory):

```bash
just dev
```

Start tikv cluster and development server (persisted):

```bash
# Running the tikv cluster requires the tikv `DB_URL` environment variable.
just pd # terminal 1
just tikv # terminal 2
just db # terminal 3
just migrate # terminal 4, imports the database schema
just sql # terminal 4, connects to SurrealDB to run SQL statements
just dev # terminal 4
```

Run tests:

```bash
just test # all tests
just test <name> # run specific test matching <name>
```

Check for release:

```bash
just check
```

Build server:

```bash
just build
```

Clean up everything:

```bash
just clean
```
