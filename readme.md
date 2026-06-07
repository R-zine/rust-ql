# rust-ql

A small SQL-like database written in Rust.

## Features

- Custom lexer and parser
- AST-based query representation
- In-memory table storage
- Persistence to disk
- `CREATE TABLE`
- `INSERT`
- `SELECT`
- `WHERE` filtering
- Primary key constraints
- Hash-based primary key index

## Example

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name STRING
);

INSERT INTO users VALUES (1, 'Phil');

SELECT name
FROM users
WHERE id = 1;
```

## Project Structure

- `interface` – shared tokens, AST nodes, statements, and values
- `lexer` – tokenization
- `parser` – SQL parsing and AST construction
- `backend` – execution engine, storage, and persistence

## Current Limitations

- Very small SQL subset
- Single-table queries only
- No joins
- No updates or deletes
- Primary-key index only

## License

MIT
