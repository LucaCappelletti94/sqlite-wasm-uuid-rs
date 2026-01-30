# sqlite-wasm-uuid-rs

[![Crates.io](https://img.shields.io/crates/v/sqlite-wasm-uuid-rs.svg)](https://crates.io/crates/sqlite-wasm-uuid-rs)
[![Docs.rs](https://docs.rs/sqlite-wasm-uuid-rs/badge.svg)](https://docs.rs/sqlite-wasm-uuid-rs)
[![CI](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/workflows/Test/badge.svg)](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/actions)
[![License](https://img.shields.io/crates/l/sqlite-wasm-uuid-rs.svg)](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/blob/master/LICENSE)

Rust [SQLite-WASM](https://sqlite.org/wasm) extension for [UUIDv4 (Random)](https://en.wikipedia.org/wiki/Universally_unique_identifier#Version_4_(random)) & [UUIDv7 (Time-ordered)](https://uuid7.com/) generation.
Powered by the [uuid](https://crates.io/crates/uuid) crate and built for [sqlite-wasm-rs](https://crates.io/crates/sqlite-wasm-rs).
The crate is [`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html) and compiles to [`wasm32-unknown-unknown`](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).

## SQL Functions

- `uuid()`: Returns a new random Version 4 UUID as a 36-character string.
- `uuid_str(X)`: Parses X (blob or text) and returns a canonical 36-char string.
- `uuid_blob(X)`: Converts X to a 16-byte blob, or generates a new one if no X.
- `uuid7()`: Returns a new Version 7 UUID as a 36-character string.
- `uuid7_blob()`: Returns a new Version 7 UUID as a 16-byte BLOB. If called with 1 argument, converts the input UUID (TEXT or BLOB format) to a 16-byte BLOB.

For instance, you can now set the DEFAULT value of a TEXT column to `uuid()` and of a BLOB column to `uuid_blob()` to have UUIDs automatically generated upon insertion.

```sql
CREATE TABLE so_many_uuids (
    id_text TEXT PRIMARY KEY DEFAULT (uuid()),
    id_blob BLOB PRIMARY KEY DEFAULT (uuid_blob()),
    idv7_text TEXT DEFAULT (uuid7()),
    idv7_blob BLOB DEFAULT (uuid7_blob())
);
```

## Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
sqlite-wasm-uuid-rs = "0.1"
```

Then, depending on which library you are using to interface with SQLite-WASM, register the extension and use the functions as shown below. Please be mindful that the following examples are not executed as part of the CI tests because the different libraries have different sqlite dependencies which would conflict with each other. Instead, complete working examples are provided in the [test-rusqlite](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/tree/master/test-rusqlite) and [test-diesel](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/tree/master/test-diesel) directories.

Do note that if you are using [`diesel`](https://docs.diesel.rs/2.3.x/diesel/index.html) and only need to use these functions inside a query, and not in schema definitions (e.g., as default values), you can avoid using this extension altogether by simply using [`declare_sql_function`](https://docs.diesel.rs/2.3.x/diesel/expression/functions/attr.declare_sql_function.html#use-with-sqlite) to map the Rust functions you need.

### Rusqlite

```rust,ignore
use rusqlite::Connection;

// Register the extension (unsafe because it affects global SQLite state)
unsafe {
    sqlite_wasm_uuid_rs::register().expect("failed to register");
}

let conn = Connection::open_in_memory().unwrap();

// Generate a random UUIDv4 string
let uuid_str: String = conn.query_row("SELECT uuid()", [], |r| r.get(0)).unwrap();

// Generate a random UUIDv4 blob
let uuid_blob: Vec<u8> = conn.query_row("SELECT uuid_blob()", [], |r| r.get(0)).unwrap();
```

See [test-rusqlite](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/tree/master/test-rusqlite) for a complete CI-tested example.

### Diesel

```rust,ignore
// Register the extension
unsafe {
    sqlite_wasm_uuid_rs::register().expect("failed to register");
}

// Use raw SQL or `sql_query`
diesel::sql_query("SELECT uuid()").execute(&mut conn)?;

// Or use the functions in your schema definitions (requires custom SQL types)
// See test-diesel for the full boilerplate setup.
```

See [test-diesel](https://github.com/LucaCappelletti94/sqlite-wasm-uuid-rs/tree/master/test-diesel) for a complete CI-tested example.

## Testing

To run the tests (including the usage examples which are mirrored in the test suite), use [`wasm-pack`](https://rustwasm.github.io/wasm-pack/):

```bash
# Run tests in Headless Firefox
wasm-pack test --firefox --headless

# Or in Headless Chrome
wasm-pack test --chrome --headless
```

> **Note**: Standard `cargo test` does not work for this crate as it targets `wasm32-unknown-unknown` and requires a browser environment provided by `wasm-pack`.
