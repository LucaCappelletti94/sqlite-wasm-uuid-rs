//! Crate illustrating usage of the `sqlite-wasm-uuid-rs` crate via `diesel`.

use diesel::{
    AsExpression, QueryId,
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    prelude::*,
    serialize::{Output, ToSql},
    sql_types::Binary,
    sqlite::{Sqlite, SqliteConnection},
};
use uuid::Uuid;
use wasm_bindgen_test::wasm_bindgen_test;

// Define a custom SQL type for UUIDs stored as BLOBs
#[derive(diesel::sql_types::SqlType, QueryId)]
#[diesel(sqlite_type(name = "Binary"))]
pub struct SqliteUuid;

// We must use a wrapper struct to implement AsExpression/ToSql/FromSql for a
// foreign type (Uuid) to avoid orphan rules if we are mapping to a LOCAL
// SqlType (SqliteUuid). OR we can implement it for a local struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = SqliteUuid)]
pub struct BlobUuid(pub Uuid);

// Conversion helpers
impl From<Uuid> for BlobUuid {
    fn from(u: Uuid) -> Self {
        BlobUuid(u)
    }
}

impl From<BlobUuid> for Uuid {
    fn from(b: BlobUuid) -> Self {
        b.0
    }
}

// Implement FromSql: Retrieve SqliteUuid (Blob) -> BlobUuid
impl FromSql<SqliteUuid, Sqlite> for BlobUuid {
    fn from_sql(value: <Sqlite as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        // Delegate to Vec<u8> (Binary)
        let bytes = <Vec<u8> as FromSql<Binary, Sqlite>>::from_sql(value)?;
        let u = Uuid::from_slice(&bytes)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(BlobUuid(u))
    }
}

// Implement ToSql: Send BlobUuid -> SqliteUuid (Blob)
impl ToSql<SqliteUuid, Sqlite> for BlobUuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> diesel::serialize::Result {
        // Delegate to [u8] (Binary)
        <[u8] as ToSql<Binary, Sqlite>>::to_sql(self.0.as_bytes(), out)
    }
}

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

table! {
    users (id) {
        id -> Text,
        name -> Text,
    }
}

table! {
    posts (id) {
        id -> crate::SqliteUuid, // Use our custom SQL type
        name -> Text,
    }
}

#[derive(Queryable, Insertable, Selectable, Debug, PartialEq)]
#[diesel(table_name = posts)]
struct Post {
    id: BlobUuid, // Use wrapper
    name: String,
}

#[wasm_bindgen_test]
fn test_uuid_with_diesel() {
    unsafe {
        // Register the UUID extension
        sqlite_wasm_uuid_rs::register().expect("Failed to register sqlite-wasm-uuid-rs");
    }

    let mut conn = SqliteConnection::establish(":memory:").unwrap();

    // Use raw SQL to create table with default uuid
    diesel::sql_query("CREATE TABLE users (id TEXT PRIMARY KEY DEFAULT (uuid7()), name TEXT)")
        .execute(&mut conn)
        .unwrap();

    // Create table for BLOB UUIDs.
    diesel::sql_query("CREATE TABLE posts (id BLOB PRIMARY KEY DEFAULT (uuid7_blob()), name TEXT)")
        .execute(&mut conn)
        .unwrap();

    // Test 1: Insert into 'users' using default uuid7() (Text)
    diesel::sql_query("INSERT INTO users (name) VALUES ('Alice')").execute(&mut conn).unwrap();

    // Test 2: Insert into 'posts' using specific UUID (as Blob via SqliteUuid)
    let my_uuid = Uuid::now_v7();
    let new_post = Post { id: BlobUuid(my_uuid), name: "My First Post".to_string() };

    diesel::insert_into(posts::table).values(&new_post).execute(&mut conn).unwrap();

    // Retrieve it back
    let saved_post: Post =
        posts::table.filter(posts::id.eq(BlobUuid(my_uuid))).first(&mut conn).unwrap();

    assert_eq!(saved_post.id.0, my_uuid);
    assert_eq!(saved_post.name, "My First Post");

    // Test 3: Select raw function output as Blob
    let generated_uuid: BlobUuid = diesel::select(diesel::dsl::sql::<SqliteUuid>("uuid7_blob()"))
        .get_result(&mut conn)
        .unwrap();

    assert_eq!(generated_uuid.0.get_version(), Some(uuid::Version::SortRand));
}
