use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;

use super::error::*;

table! {
    schema_migrations {
        id -> Integer,
        name -> Text,
        checksum -> Binary,
    }
}

pub fn create_schema_table_if_necessary(conn: &PgConnection) -> Result<()> {
    conn.batch_execute(
        r"CREATE TABLE IF NOT EXISTS schema_migrations(
                id int PRIMARY KEY,
                name varchar(255) NOT NULL,
                checksum bytea NOT NULL
        )",
    ).chain_err(|| ErrorKind::MigrationTableInitializationFailure)
}

#[derive(Queryable, Insertable)]
#[table_name = "schema_migrations"]
pub struct MigrationEntry {
    pub id: i32,
    pub name: String,
    pub checksum: Vec<u8>,
}
