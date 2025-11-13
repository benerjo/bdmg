///the purpose of this example is to show how to use the build scripts to build everything needed
/// to have an inmemory database, without any external element apart from the object store.

#[macro_use]
extern crate serde_derive;

include!(concat!(env!("OUT_DIR"), "/mod.rs"));

use diesel::Connection;
use diesel_migrations::MigrationHarness;

const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    diesel_migrations::EmbeddedMigrations::new(&[diesel_migrations::EmbeddedMigration::new(
        include_str!(concat!(env!("OUT_DIR"), "/up.sql")),
        None,
        diesel_migrations::EmbeddedName::new("install"),
        diesel_migrations::TomlMetadataWrapper::new(true),
    )]);

#[derive(Debug)]
pub enum InMemoryCreationError {
    ConnectionProblem(diesel::ConnectionError),
    MigrationProblem,
}

impl std::fmt::Display for InMemoryCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InMemoryCreationError::ConnectionProblem(e) => {
                write!(f, "Unable to connect to the in-memory database: {e}")
            }
            InMemoryCreationError::MigrationProblem => {
                write!(f, "Unable to perform all migrations")
            }
        }
    }
}

impl From<diesel::ConnectionError> for InMemoryCreationError {
    fn from(value: diesel::ConnectionError) -> Self {
        InMemoryCreationError::ConnectionProblem(value)
    }
}

///Retrieve a connection to a in-memory sqlite, with the datamodel created
pub fn get_inmemory_db() -> Result<diesel::SqliteConnection, InMemoryCreationError> {
    let mut connection = diesel::SqliteConnection::establish(":memory:")?;

    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_v) => Ok(connection),
        Err(_e) => Err(InMemoryCreationError::MigrationProblem),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_inmemory_db() {
        let mut db = match get_inmemory_db() {
            Ok(db) => db,
            Err(e) => panic!("{e}"),
        };

        let user = match User::create(
            &mut db,
            String::from("123456"),
            String::from("root"),
            String::from("root@localhost"),
            None,
        ) {
            Ok(u) => u,
            Err(e) => panic!("{e}"),
        };

        match user.get_user_group_memberships(&mut db) {
            Ok(v) => debug_assert!(v.is_empty()),
            Err(e) => panic!("{e}"),
        }
    }
}
