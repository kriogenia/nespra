//! Files of the data kept between runs, like the query history, stored as JSON.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

/// Loads the value stored in the file, or the default one if the file does not exist.
///
/// # Errors
///
/// Fails if the file exists but cannot be read or does not hold a valid value.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> Result<T, StoreError> {
    let json = match fs::read(path) {
        Ok(json) => json,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
        Err(source) => {
            return Err(StoreError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    serde_json::from_slice(&json).map_err(|source| StoreError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Stores the value in the file, creating its directory if needed.
///
/// The file is replaced at once, so it is never left half written.
///
/// # Errors
///
/// Fails if the directory or the file cannot be written.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let write_error = |source| StoreError::Write {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(write_error)?;
    }
    let json = serde_json::to_vec_pretty(value).map_err(|e| write_error(e.into()))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json).map_err(write_error)?;
    fs::rename(&temporary, path).map_err(write_error)
}

/// Errors loading or saving a file.
#[derive(Error, Debug)]
pub enum StoreError {
    /// The file could not be read.
    #[error("Unable to read {}: {source}", path.display())]
    Read {
        /// Path of the file.
        path: PathBuf,
        /// Cause of the failure.
        source: io::Error,
    },
    /// The file does not hold a valid value.
    #[error("Invalid content in {}: {source}", path.display())]
    Parse {
        /// Path of the file.
        path: PathBuf,
        /// Cause of the failure.
        source: serde_json::Error,
    },
    /// The file could not be written.
    #[error("Unable to save {}: {source}", path.display())]
    Write {
        /// Path of the file.
        path: PathBuf,
        /// Cause of the failure.
        source: io::Error,
    },
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use super::*;

    type Value = BTreeMap<String, Vec<u32>>;

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("data.json");
        let value = Value::from([("a".to_string(), vec![1, 2])]);

        save(&path, &value).unwrap();

        assert_eq!(load::<Value>(&path).unwrap(), value);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn test_load_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let value: Value = load(&dir.path().join("data.json")).unwrap();
        assert!(value.is_empty());
    }

    #[test]
    fn test_load_an_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, "not json").unwrap();
        assert!(matches!(
            load::<Value>(&path),
            Err(StoreError::Parse { .. })
        ));
    }

    #[test]
    fn test_save_where_a_directory_cannot_be_created() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let result = save(&file.path().join("data.json"), &Value::new());
        assert!(matches!(result, Err(StoreError::Write { .. })));
    }
}
