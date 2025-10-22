use crate::dictionary::types::{CharDefinitions, ConnectionMatrix, DictEntry, UnknownEntries};
use crate::error::RunomeError;
use std::fs;
use std::path::{Path, PathBuf};

/// Load dictionary entries from sysdic directory
pub fn load_entries(sysdic_dir: &Path) -> Result<Vec<DictEntry>, RunomeError> {
    let file_path = validate_file_exists(sysdic_dir, "entries.bin")?;
    let data = fs::read(&file_path)?;

    bincode::deserialize(&data).map_err(|e| RunomeError::DictDeserializationError {
        component: "entries".to_string(),
        source: e,
    })
}

/// Load connection matrix from sysdic directory
pub fn load_connections(sysdic_dir: &Path) -> Result<ConnectionMatrix, RunomeError> {
    let file_path = validate_file_exists(sysdic_dir, "connections.bin")?;
    let data = fs::read(&file_path)?;

    bincode::deserialize(&data).map_err(|e| RunomeError::DictDeserializationError {
        component: "connections".to_string(),
        source: e,
    })
}

/// Load character definitions from sysdic directory
pub fn load_char_definitions(sysdic_dir: &Path) -> Result<CharDefinitions, RunomeError> {
    let file_path = validate_file_exists(sysdic_dir, "char_defs.bin")?;
    let data = fs::read(&file_path)?;

    bincode::deserialize(&data).map_err(|e| RunomeError::DictDeserializationError {
        component: "char_defs".to_string(),
        source: e,
    })
}

/// Load unknown entries from sysdic directory
pub fn load_unknown_entries(sysdic_dir: &Path) -> Result<UnknownEntries, RunomeError> {
    let file_path = validate_file_exists(sysdic_dir, "unknowns.bin")?;
    let data = fs::read(&file_path)?;

    bincode::deserialize(&data).map_err(|e| RunomeError::DictDeserializationError {
        component: "unknowns".to_string(),
        source: e,
    })
}

/// Load morpheme index from sysdic directory
///
/// The morpheme index maps FST index IDs to vectors of morpheme IDs,
/// allowing storage of multiple morpheme IDs per surface form.
pub fn load_morpheme_index(sysdic_dir: &Path) -> Result<Vec<Vec<u32>>, RunomeError> {
    let file_path = validate_file_exists(sysdic_dir, "morpheme_index.bin")?;
    let data = fs::read(&file_path)?;

    bincode::deserialize(&data).map_err(|e| RunomeError::DictDeserializationError {
        component: "morpheme_index".to_string(),
        source: e,
    })
}

/// Load FST bytes from sysdic directory
pub fn load_fst_bytes(sysdic_dir: &Path) -> Result<Vec<u8>, RunomeError> {
    let file_path = fst_file_path(sysdic_dir)?;
    let data = fs::read(&file_path)?;
    Ok(data)
}

/// Get the path to the FST file in the sysdic directory
pub fn fst_file_path(sysdic_dir: &Path) -> Result<PathBuf, RunomeError> {
    validate_file_exists(sysdic_dir, "dic.fst")
}

/// Validate that sysdic directory exists and is accessible
pub fn validate_sysdic_directory(path: &Path) -> Result<(), RunomeError> {
    if !path.exists() {
        return Err(RunomeError::DictDirectoryNotFound {
            path: path.display().to_string(),
        });
    }

    if !path.is_dir() {
        return Err(RunomeError::DictValidationError {
            reason: format!("Path is not a directory: {}", path.display()),
        });
    }

    Ok(())
}

/// Validate that a required file exists in the sysdic directory
pub fn validate_file_exists(sysdic_dir: &Path, filename: &str) -> Result<PathBuf, RunomeError> {
    validate_sysdic_directory(sysdic_dir)?;

    let file_path = sysdic_dir.join(filename);
    if !file_path.exists() {
        return Err(RunomeError::DictFileMissing {
            filename: filename.to_string(),
        });
    }

    if !file_path.is_file() {
        return Err(RunomeError::DictValidationError {
            reason: format!("Path is not a file: {}", file_path.display()),
        });
    }

    Ok(file_path)
}
