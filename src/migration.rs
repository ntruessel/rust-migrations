use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use openssl::sha::sha256;
use regex::Regex;

use super::error::*;
use super::schema::MigrationEntry;

#[derive(Clone, Debug)]
pub struct Migration {
    pub version: i32,
    pub name: String,
    sql: String,
}

impl Migration {
    pub fn new(filename: &str, sql: &str) -> Result<Self> {
        let re = Regex::new(r"(?P<version>\d+)-(?P<name>.*)\.sql").unwrap();
        let cap = re.captures(filename).chain_err(|| {
            format!(
                "file '{}' does not match expected format '<version>-<name>.sql'",
                filename
            )
        })?;

        Ok(Migration {
            version: cap["version"].parse().unwrap(),
            name: cap["name"].to_string(),
            sql: sql.to_string(),
        })
    }

    pub fn apply(&self, conn: &PgConnection) -> Result<()> {
        conn.batch_execute(&self.sql).chain_err(|| {
            ErrorKind::FailedMigration(self.name.clone())
        })
    }

    pub fn as_entry(&self) -> MigrationEntry {
        MigrationEntry {
            id: self.version,
            name: self.name.clone(),
            checksum: sha256(self.sql.as_bytes()).to_vec(),
        }
    }
}
