# rust-ql

A small, persistent SQL-like database written in Rust. It is intended as an
educational project rather than a production database.

## Features

- Custom UTF-8-safe lexer and precedence-aware parser
- AST-based query representation
- `CREATE TABLE`, `INSERT`, and single-table `SELECT`
- Integer, floating-point, Boolean, string, and `NULL` values
- `WHERE` filtering, arithmetic, comparisons, and `AND`/`OR`/`NOT`
- Column aliases with `AS`
- Type, arity, `NOT NULL`, and primary-key validation
- Case-insensitive unquoted table and column lookup
- Hash-based primary-key index
- Validated JSON persistence using atomic replacement
- Structured query results for library callers

## Run

Rust 1.85 or newer is required because the workspace uses the Rust 2024
edition.

```console
cargo run
```

The REPL stores data in `db.json` in the current directory. Enter one statement
per line and use `exit` or `quit` to close it.

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name STRING NOT NULL, active BOOLEAN);
INSERT INTO users VALUES (1, 'Phil', TRUE);
INSERT INTO users (name, id) VALUES ('Alice', 2);
SELECT name AS user_name, active FROM users WHERE id >= 1;
```

Strings use SQL escaping, so an apostrophe is written twice:

```sql
INSERT INTO users VALUES (3, 'O''Reilly', FALSE);
```

For local performance experiments, `.seed <table> <count>` inserts
schema-compatible generated rows.

## Test and lint

```console
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

To enable the repository's pre-commit checks after cloning:

```console
git config core.hooksPath .githooks
```

The hook rejects staged whitespace errors or an out-of-date lockfile, then runs
formatting, Clippy, the complete test suite, and rustdoc with warnings denied.

## Project structure

- `interface` — shared tokens, AST nodes, statements, and values
- `lexer` — tokenization and source spans
- `parser` — SQL parsing and AST construction
- `backend` — validation, execution, indexing, and persistence
- root binary — interactive REPL and result formatting

## Current limitations

- Deliberately small SQL subset
- Single-table queries only; no joins, grouping, ordering, or aggregation
- No updates, deletes, transactions, or concurrent writers
- One non-floating-point primary key per table
- JSON storage rewrites the database for every mutating statement
- Statements in the REPL must fit on one line

## License

MIT. See [LICENSE](LICENSE).
