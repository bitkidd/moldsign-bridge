//! Preserve the local HTTPS identity when the application identifier changes.
use std::{
    fs, io,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
};

/// Copy a complete legacy pair atomically, without modifying either existing identity.
pub fn migrate_connection(previous: &Path, current: &Path) -> io::Result<bool> {
    if current.exists()
        || !previous.join("localhost.pem").is_file()
        || !previous.join("localhost-key.pem").is_file()
    {
        return Ok(false);
    }
    let parent = current.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Connection directory has no parent",
        )
    })?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".connection-migration-{}", uuid::Uuid::new_v4()));
    fs::DirBuilder::new().mode(0o700).create(&staging)?;
    let result = (|| {
        for name in ["localhost.pem", "localhost-key.pem"] {
            let destination = staging.join(name);
            fs::copy(previous.join(name), &destination)?;
            fs::set_permissions(destination, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(&staging, current)?;
        Ok(true)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("bridge-migration-test-{}", uuid::Uuid::new_v4()));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            Self(path)
        }
        fn pair(&self, name: &str, content: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("localhost.pem"), format!("certificate:{content}")).unwrap();
            fs::write(path.join("localhost-key.pem"), format!("key:{content}")).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn migration_preserves_identity_and_restricts_permissions() {
        let fixture = Fixture::new();
        let previous = fixture.pair("old", "test");
        let current = fixture.0.join("new/connection");
        assert!(migrate_connection(&previous, &current).unwrap());
        for name in ["localhost.pem", "localhost-key.pem"] {
            assert_eq!(
                fs::read(previous.join(name)).unwrap(),
                fs::read(current.join(name)).unwrap()
            );
            assert_eq!(
                fs::metadata(current.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert_eq!(
            fs::metadata(&current).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(!migrate_connection(&previous, &current).unwrap());
    }
    #[test]
    fn existing_destination_is_not_overwritten() {
        let fixture = Fixture::new();
        let previous = fixture.pair("old", "old");
        let current = fixture.pair("new", "new");
        assert!(!migrate_connection(&previous, &current).unwrap());
        assert_eq!(
            fs::read_to_string(current.join("localhost-key.pem")).unwrap(),
            "key:new"
        );
    }
    #[test]
    fn incomplete_legacy_pair_is_not_migrated() {
        let fixture = Fixture::new();
        let previous = fixture.pair("old", "test");
        fs::remove_file(previous.join("localhost-key.pem")).unwrap();
        let current = fixture.0.join("new");
        assert!(!migrate_connection(&previous, &current).unwrap());
        assert!(!current.exists());
    }
}
