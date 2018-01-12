#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives;
#[macro_use]
extern crate error_chain;
extern crate openssl;
extern crate regex;

pub mod error;
mod migration;
mod schema;

pub use self::migration::Migration;

use diesel::expression::dsl::exists;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::select;
use diesel::insert_into;
use std::collections::HashMap;
use std::fs::read_dir;
use std::fs::File;
use std::io::BufReader;
use std::io::stdout;
use std::io::Write;
use std::io::prelude::*;
use std::path::Path;

use self::error::*;
use self::schema::schema_migrations::dsl::*;
use self::schema::*;

pub struct Migrations {
    migrations: Vec<Migration>,
}

impl Migrations {
    pub fn from_path(p: &Path) -> Result<Self> {
        let mut migrations = read_migrations_from_path(p)?;
        migrations.sort_unstable_by(|a, b| a.version.cmp(&b.version));
        try!(check_for_duplicates(&migrations));

        Ok(Migrations { migrations })
    }

    pub fn run(self, conn: &PgConnection) -> Result<()> {
        try!(schema::create_schema_table_if_necessary(conn));

        let runner = MigrationRunner {
            migrations: self,
            connection: conn,
        };
        try!(runner.check_migrations());
        try!(runner.run_all());

        Ok(())
    }
}

struct MigrationRunner<'a> {
    migrations: Migrations,
    connection: &'a PgConnection,
}

impl<'a> MigrationRunner<'a> {
    pub fn check_migrations(&self) -> Result<()> {
        let mut applied_migrations: HashMap<i32, MigrationEntry> = schema_migrations
            .load::<MigrationEntry>(self.connection)?
            .into_iter()
            .map(|entry| (entry.id, entry))
            .collect();

        for migration in &self.migrations.migrations {
            if let Some(entry) = applied_migrations.remove(&migration.version) {
                try!(self.check_name_and_checksum(migration.as_entry(), entry));
            }
        }

        if !applied_migrations.is_empty() {
            return Err(
                ErrorKind::MissingMigrations(
                    applied_migrations
                        .into_iter()
                        .map(|(_, v)| v.name)
                        .collect(),
                ).into(),
            );
        }

        Ok(())
    }

    pub fn run_all(&self) -> Result<()> {
        for migration in &self.migrations.migrations {
            try!(self.run_migration(migration))
        }

        Ok(())
    }

    fn check_name_and_checksum(
        &self,
        migration: MigrationEntry,
        entry: MigrationEntry,
    ) -> Result<()> {
        if migration.name != entry.name {
            return Err(
                ErrorKind::MigrationConflict(migration.name, entry.name).into(),
            );
        }
        if migration.checksum != entry.checksum {
            return Err(
                ErrorKind::MigrationIntegrityViolation(migration.name).into(),
            );
        }
        Ok(())
    }

    fn run_migration(&self, migration: &Migration) -> Result<()> {
        print!("[      ] {}", migration.name);
        stdout().flush().ok();

        if self.already_applied(&migration)? {
            println!("\r[ SKIP ]");
            return Ok(());
        }

        match migration.apply(self.connection).and_then(|_| {
            self.register_migration(&migration)
        }) {
            Ok(()) => {
            println!("\r[  OK  ]");
            Ok(())
        }
            Err(e) => {
                println!("\r[ FAIL ]");
                Err(e)
            }
        }
    }

    fn already_applied(&self, migration: &Migration) -> Result<bool> {
        select(exists(schema_migrations.filter(id.eq(migration.version))))
            .get_result(self.connection)
            .chain_err(|| ErrorKind::FailedMigration(migration.name.clone()))
    }

    fn register_migration(&self, migration: &Migration) -> Result<()> {
        try!(
            insert_into(schema_migrations)
                .values(&migration.as_entry())
                .execute(self.connection)
                .map(|_| ())
        );

        Ok(())
    }
}

fn check_for_duplicates(migrations: &Vec<Migration>) -> Result<()> {
    let mut prev_version = -1;
    let mut duplicates = vec![];
    for migration in migrations {
        if migration.version == prev_version {
            duplicates.push(migration.clone());
        }
        prev_version = migration.version;
    }
    if duplicates.len() > 0 {
        return Err(
            ErrorKind::DuplicateMigrations(
                duplicates
                    .into_iter()
                    .map(|migration| migration.version)
                    .collect(),
            ).into(),
        );
    }

    Ok(())
}

fn read_migrations_from_path(p: &Path) -> Result<Vec<Migration>> {
    let mut migrations = Vec::new();

    let dir_contents = read_dir(p).chain_err(|| ErrorKind::MigrationReadError)?;
    for f in dir_contents {
        let filepath: &Path = &f.chain_err(|| ErrorKind::MigrationReadError)?.path();
        let filename: &str = filepath.to_str().ok_or::<Error>(
            ErrorKind::MigrationReadError.into(),
        )?;
        let file = File::open(filepath).chain_err(|| {
            format!("unable to open file '{}'", filename)
        })?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader.read_to_string(&mut content).chain_err(|| {
            format!("unable to read file '{}'", filename)
        })?;
        let m = Migration::new(filename, &content)?;
        migrations.push(m);
    }

    Ok(migrations)
}
