error_chain!{
    foreign_links {
        Diesel(::diesel::result::Error);
    }

    errors {
        MigrationReadError {
            description("error while reading migrations from filesystem")
            display("error while reading migrations from filesystem")
        }
        DuplicateMigrations(migrations: Vec<i32>) {
            description("multiple migrations with the same version")
            display("multiple migrations with the same version: '{:?}'", migrations)
        }
        MissingMigrations(migrations: Vec<String>) {
            description("applied migrations are missing")
            display("applied migrations '{:?}' are missing", migrations)
        }
        MigrationConflict(migration: String, present: String) {
            description("a migration with the same version is already applied")
            display("migration '{}' conflicts with already applied migration '{}'", migration, present)
        }
        MigrationIntegrityViolation(migration: String) {
            description("migration is already applied but has different checksum")
            display("migration '{}' is already applied but the applied version has a different checksum", migration)
        }
        FailedMigration(migration: String) {
            description("migration execution failed")
            display("migraiton '{}' failed", migration)
        }
        MigrationTableInitializationFailure {
            description("initialization of the migration table failed")
            display("initialization of the migration table failed")
        }
    }
}
