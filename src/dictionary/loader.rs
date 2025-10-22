use crate::dictionary::types::{CharDefinitions, ConnectionMatrix, DictEntry, UnknownEntries};
use crate::error::RunomeError;
use memmap2::{Mmap, MmapOptions};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub const CONNECTIONS_PACK_FILE: &str = "connections.pack";
pub const CONNECTIONS_PACK_MAGIC: &[u8; 4] = b"RNCM";
pub const CONNECTIONS_PACK_VERSION: u16 = 1;
pub const CONNECTIONS_PACK_HEADER_SIZE: usize = 4 + 2 + 4 + 4;

pub const MORPHEME_PACK_FILE: &str = "morpheme_index.pack";
pub const MORPHEME_PACK_MAGIC: &[u8; 4] = b"RNMI";
pub const MORPHEME_PACK_VERSION: u16 = 1;
pub const MORPHEME_PACK_FIXED_HEADER: usize = 4 + 2 + 4 + 4; // magic + version + entries + values

pub struct MorphemeIndexPack {
    pub mmap: Mmap,
    pub entry_count: u32,
    pub value_count: u32,
    pub offsets_offset: usize,
    pub values_offset: usize,
}

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

/// Try to load packed connection matrix for zero-copy access.
///
/// Returns None if the packed file is not present.
pub fn load_connections_packed(sysdic_dir: &Path) -> Result<Option<(Mmap, u32, u32)>, RunomeError> {
    let pack_path = sysdic_dir.join(CONNECTIONS_PACK_FILE);
    if !pack_path.exists() {
        return Ok(None);
    }

    let file = File::open(&pack_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    if mmap.len() < CONNECTIONS_PACK_HEADER_SIZE {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "connections.pack is too small ({} bytes, expected at least {})",
                mmap.len(),
                CONNECTIONS_PACK_HEADER_SIZE
            ),
        });
    }

    if &mmap[0..4] != CONNECTIONS_PACK_MAGIC {
        return Err(RunomeError::DictValidationError {
            reason: "connections.pack has invalid magic header".to_string(),
        });
    }

    let version = u16::from_le_bytes([mmap[4], mmap[5]]);
    if version != CONNECTIONS_PACK_VERSION {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "Unsupported connections.pack version {} (expected {})",
                version, CONNECTIONS_PACK_VERSION
            ),
        });
    }

    let rows = u32::from_le_bytes([mmap[6], mmap[7], mmap[8], mmap[9]]);
    let cols = u32::from_le_bytes([mmap[10], mmap[11], mmap[12], mmap[13]]);
    let expected_len = (rows as usize)
        .checked_mul(cols as usize)
        .and_then(|len| len.checked_mul(std::mem::size_of::<i16>()))
        .ok_or_else(|| RunomeError::DictValidationError {
            reason: format!(
                "Invalid connection matrix dimensions: rows={}, cols={}",
                rows, cols
            ),
        })?;

    if mmap.len() != CONNECTIONS_PACK_HEADER_SIZE + expected_len {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "connections.pack size mismatch: actual={} expected={}",
                mmap.len(),
                CONNECTIONS_PACK_HEADER_SIZE + expected_len
            ),
        });
    }

    Ok(Some((mmap, rows, cols)))
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

pub fn load_morpheme_index_packed(
    sysdic_dir: &Path,
) -> Result<Option<MorphemeIndexPack>, RunomeError> {
    let pack_path = sysdic_dir.join(MORPHEME_PACK_FILE);
    if !pack_path.exists() {
        return Ok(None);
    }

    let file = File::open(&pack_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    if mmap.len() < MORPHEME_PACK_FIXED_HEADER {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "morpheme_index.pack is too small ({} bytes, expected at least {})",
                mmap.len(),
                MORPHEME_PACK_FIXED_HEADER
            ),
        });
    }

    if &mmap[0..4] != MORPHEME_PACK_MAGIC {
        return Err(RunomeError::DictValidationError {
            reason: "morpheme_index.pack has invalid magic header".to_string(),
        });
    }

    let version = u16::from_le_bytes([mmap[4], mmap[5]]);
    if version != MORPHEME_PACK_VERSION {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "Unsupported morpheme_index.pack version {} (expected {})",
                version, MORPHEME_PACK_VERSION
            ),
        });
    }

    let entry_count = u32::from_le_bytes([mmap[6], mmap[7], mmap[8], mmap[9]]);
    let value_count = u32::from_le_bytes([mmap[10], mmap[11], mmap[12], mmap[13]]);

    let offsets_len = entry_count as usize + 1;
    let offsets_bytes = offsets_len
        .checked_mul(std::mem::size_of::<u32>())
        .ok_or_else(|| RunomeError::DictValidationError {
            reason: "morpheme_index.pack offsets size overflow".to_string(),
        })?;
    let values_bytes = (value_count as usize)
        .checked_mul(std::mem::size_of::<u32>())
        .ok_or_else(|| RunomeError::DictValidationError {
            reason: "morpheme_index.pack values size overflow".to_string(),
        })?;

    let expected_size = MORPHEME_PACK_FIXED_HEADER
        .checked_add(offsets_bytes)
        .and_then(|v| v.checked_add(values_bytes))
        .ok_or_else(|| RunomeError::DictValidationError {
            reason: "morpheme_index.pack total size overflow".to_string(),
        })?;

    if mmap.len() != expected_size {
        return Err(RunomeError::DictValidationError {
            reason: format!(
                "morpheme_index.pack size mismatch: actual={} expected={}",
                mmap.len(),
                expected_size
            ),
        });
    }

    Ok(Some(MorphemeIndexPack {
        mmap,
        entry_count,
        value_count,
        offsets_offset: MORPHEME_PACK_FIXED_HEADER,
        values_offset: MORPHEME_PACK_FIXED_HEADER + offsets_bytes,
    }))
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
