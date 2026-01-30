//! Crate illustrating usage of the `sqlite-wasm-uuid-rs` crate via `rusqlite`.
extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use rusqlite::Connection;
use uuid::Uuid;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Tests the `uuid` extension via `rusqlite`.
#[wasm_bindgen_test]
fn test_uuid_via_rusqlite() {
    unsafe {
        sqlite_wasm_uuid_rs::register();
    }
    let conn = Connection::open_in_memory().unwrap();

    let u1: String = conn.query_row("SELECT uuid()", [], |r| r.get(0)).unwrap();
    let u2: String = conn.query_row("SELECT uuid()", [], |r| r.get(0)).unwrap();
    assert_eq!(u1.len(), 36);
    assert_ne!(u1, u2);

    let blob_from_text: Vec<u8> = conn
        .query_row("SELECT uuid_blob('00000000-0000-0000-0000-000000000000')", [], |r| r.get(0))
        .unwrap();
    assert_eq!(blob_from_text.len(), 16);
    assert_eq!(blob_from_text, vec![0; 16]);

    let blob_from_blob: Vec<u8> =
        conn.query_row("SELECT uuid_blob(?1)", [&blob_from_text], |r| r.get(0)).unwrap();
    assert_eq!(blob_from_blob, blob_from_text);

    let str_from_blob: String =
        conn.query_row("SELECT uuid_str(?1)", [&blob_from_text], |r| r.get(0)).unwrap();
    assert_eq!(str_from_blob, "00000000-0000-0000-0000-000000000000");

    let input = "12345678-1234-1234-1234-123456789abc";
    let roundtrip: String =
        conn.query_row("SELECT uuid_str(uuid_blob(?1))", [input], |r| r.get(0)).unwrap();
    assert_eq!(roundtrip, input);
}

/// Tests usage of `uuid()` and `uuid_blob()` as a `DEFAULT` clause value.
#[wasm_bindgen_test]
fn test_uuid4_default() {
    unsafe {
        sqlite_wasm_uuid_rs::register();
    }
    let mut conn = Connection::open_in_memory().unwrap();

    conn.execute("CREATE TABLE t(id TEXT PRIMARY KEY DEFAULT (uuid()), val INTEGER)", []).unwrap();

    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx.prepare("INSERT INTO t(val) VALUES (?)").unwrap();
        for i in 0..100 {
            stmt.execute([i]).unwrap();
        }
    }
    tx.commit().unwrap();

    let count: i64 =
        conn.query_row("SELECT count(*) FROM t WHERE length(id) = 36", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 100);

    let mut ids: Vec<String> = conn
        .prepare("SELECT id FROM t")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    for id in &ids {
        let u = Uuid::parse_str(id).unwrap();
        assert_eq!(u.get_version_num(), 4);
    }

    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 100);

    conn.execute("DROP TABLE t", []).unwrap();
    conn.execute("CREATE TABLE t(id BLOB PRIMARY KEY DEFAULT (uuid_blob()), val INTEGER)", [])
        .unwrap();

    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx.prepare("INSERT INTO t(val) VALUES (?)").unwrap();
        for i in 0..100 {
            stmt.execute([i]).unwrap();
        }
    }
    tx.commit().unwrap();

    let mut blobs: Vec<Vec<u8>> = conn
        .prepare("SELECT id FROM t")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    for blob in &blobs {
        assert_eq!(blob.len(), 16);
    }
    blobs.sort();
    blobs.dedup();
    assert_eq!(blobs.len(), 100);
}

/// Tests the `uuid7` extension via `rusqlite`.
#[wasm_bindgen_test]
fn test_uuid7_via_rusqlite() {
    unsafe {
        sqlite_wasm_uuid_rs::register();
    }

    let conn = Connection::open_in_memory().unwrap();

    let mut results = Vec::new();
    for _ in 0..100 {
        let u: String = conn.query_row("SELECT uuid7()", [], |r| r.get(0)).unwrap();
        results.push(u);
    }
    assert_eq!(results[0].len(), 36);

    let blob: Vec<u8> = conn.query_row("SELECT uuid7_blob()", [], |r| r.get(0)).unwrap();
    assert_eq!(blob.len(), 16);
    let u_blob = Uuid::from_slice(&blob).unwrap();
    assert_eq!(u_blob.get_version_num(), 7);

    let input_text = u_blob.to_string();
    let blob_from_text: Vec<u8> =
        conn.query_row("SELECT uuid7_blob(?1)", [&input_text], |r| r.get(0)).unwrap();
    assert_eq!(blob_from_text, blob);

    let blob_from_blob: Vec<u8> =
        conn.query_row("SELECT uuid7_blob(?1)", [&blob], |r| r.get(0)).unwrap();
    assert_eq!(blob_from_blob, blob);

    let mut sorted = results.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(results.len(), sorted.len());

    for i in 0..results.len() - 1 {
        assert!(results[i] < results[i + 1], "UUIDv7 (Text) not sorted at index {}", i);
    }
}

/// Tests usage of `uuid7()` as a `DEFAULT` clause value.
#[wasm_bindgen_test]
fn test_uuid7_default() {
    unsafe {
        sqlite_wasm_uuid_rs::register();
    }
    let mut conn = Connection::open_in_memory().unwrap();

    conn.execute("CREATE TABLE t(id TEXT PRIMARY KEY DEFAULT (uuid7()), val INTEGER)", []).unwrap();

    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx.prepare("INSERT INTO t(val) VALUES (?)").unwrap();
        for i in 0..100 {
            stmt.execute([i]).unwrap();
        }
    }
    tx.commit().unwrap();

    let ids: Vec<String> = conn
        .prepare("SELECT id FROM t ORDER BY val")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    for i in 0..ids.len() - 1 {
        assert!(
            ids[i] < ids[i + 1],
            "UUIDv7 not sorted at index {}: {} >= {}",
            i,
            ids[i],
            ids[i + 1]
        );
    }

    for id in &ids {
        let u = Uuid::parse_str(id).unwrap();
        assert_eq!(u.get_version_num(), 7);
    }
}
