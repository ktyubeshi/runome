use crate::error::RunomeError;
use memmap2::Mmap;
use smallvec::SmallVec;
use std::path::Path;
use std::slice;
use std::sync::Arc;

use super::{loader, types::*};

type CategoryId = u16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CategoryMask(u128);

impl CategoryMask {
    const MAX_BITS: usize = 128;

    #[inline]
    pub(crate) fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub(crate) fn insert(&mut self, id: CategoryId) {
        debug_assert!((id as usize) < Self::MAX_BITS);
        self.0 |= 1u128 << id;
    }

    #[inline]
    pub(crate) fn contains(self, id: CategoryId) -> bool {
        debug_assert!((id as usize) < Self::MAX_BITS);
        (self.0 & (1u128 << id)) != 0
    }

    #[inline]
    pub(crate) fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub(crate) fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline]
    pub(crate) fn iter(self) -> impl Iterator<Item = CategoryId> {
        CategoryMaskIter { remaining: self.0 }
    }
}

struct CategoryMaskIter {
    remaining: u128,
}

impl Iterator for CategoryMaskIter {
    type Item = CategoryId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let bit = self.remaining.trailing_zeros() as u16;
        self.remaining &= self.remaining - 1;
        Some(bit)
    }
}

#[derive(Debug, Clone)]
enum ConnectionMatrixStorage {
    Owned(Arc<[i16]>),
    Mmap { mmap: Arc<Mmap>, data_offset: usize },
}

#[derive(Debug, Clone)]
struct PackedConnectionMatrix {
    rows: u32,
    cols: u32,
    len: usize,
    storage: ConnectionMatrixStorage,
}

#[derive(Debug, Clone)]
enum MorphemeIndexStorage {
    Owned {
        offsets: Vec<u32>,
        values: Vec<u32>,
    },
    Packed {
        mmap: Arc<Mmap>,
        entry_count: usize,
        value_count: usize,
        offsets_offset: usize,
        values_offset: usize,
    },
}

#[derive(Debug, Clone)]
struct MorphemeIndex {
    storage: MorphemeIndexStorage,
}

pub struct MorphemeIndexView<'a> {
    index: &'a MorphemeIndex,
}

#[derive(Clone, Debug)]
pub struct ConnectionMatrix {
    inner: Arc<PackedConnectionMatrix>,
}

pub struct ConnectionMatrixView<'a> {
    matrix: &'a PackedConnectionMatrix,
}

#[derive(Debug, Clone, Copy)]
struct CategoryFlags {
    invoke: bool,
    group: bool,
    length: u8,
}

#[derive(Debug)]
struct CodePointCategory {
    start: u32,
    end: u32,
    primary_id: CategoryId,
    compat_mask: CategoryMask,
}

#[derive(Debug)]
struct CategoryIndex {
    name_to_id: std::collections::HashMap<String, CategoryId>,
    names: Vec<String>,
    flags: Vec<CategoryFlags>,
    code_ranges: Vec<CodePointCategory>,
    default_id: Option<CategoryId>,
}

impl CategoryIndex {
    fn new(defs: &CharDefinitions) -> Result<Self, RunomeError> {
        let mut names: Vec<String> = defs.categories.keys().cloned().collect();
        names.sort();

        if names.len() > CategoryMask::MAX_BITS {
            return Err(RunomeError::DictValidationError {
                reason: format!(
                    "Too many character categories: {} (max supported: {})",
                    names.len(),
                    CategoryMask::MAX_BITS
                ),
            });
        }

        let mut name_to_id = std::collections::HashMap::with_capacity(names.len());
        let mut flags = Vec::with_capacity(names.len());
        let mut default_id = None;

        for (idx, name) in names.iter().enumerate() {
            let Some(cat) = defs.categories.get(name) else {
                return Err(RunomeError::DictValidationError {
                    reason: format!("Missing category metadata for '{}'", name),
                });
            };
            if name == "DEFAULT" {
                default_id = Some(idx as CategoryId);
            }
            name_to_id.insert(name.clone(), idx as CategoryId);
            flags.push(CategoryFlags {
                invoke: cat.invoke,
                group: cat.group,
                length: cat.length,
            });
        }

        let mut code_ranges = Vec::with_capacity(defs.code_ranges.len());
        for range in &defs.code_ranges {
            let Some(&primary_id) = name_to_id.get(&range.category) else {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Code range references non-existent category: {}",
                        range.category
                    ),
                });
            };
            let mut compat_mask = CategoryMask::empty();
            for compat in &range.compat_categories {
                if let Some(&id) = name_to_id.get(compat) {
                    compat_mask.insert(id);
                }
            }
            code_ranges.push(CodePointCategory {
                start: range.from as u32,
                end: range.to as u32,
                primary_id,
                compat_mask,
            });
        }

        code_ranges.sort_by_key(|entry| entry.start);

        Ok(Self {
            name_to_id,
            names,
            flags,
            code_ranges,
            default_id,
        })
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    fn category_id(&self, name: &str) -> Option<CategoryId> {
        self.name_to_id.get(name).copied()
    }

    fn category_name(&self, id: CategoryId) -> &str {
        &self.names[id as usize]
    }

    fn flags(&self, id: CategoryId) -> CategoryFlags {
        self.flags[id as usize]
    }

    fn category_mask_for_char(&self, ch: char) -> CategoryMask {
        let cp = ch as u32;
        let mut mask = CategoryMask::empty();

        // Find the first entry where start > cp
        // All matches must be before this index
        let limit = self.code_ranges.partition_point(|entry| entry.start <= cp);

        // Scan backwards from limit
        // We use a heuristic limit to avoid scanning back to 0 for large codepoints
        // We assume matching ranges are clustered near the partition point (e.g. Kanji, Kanjinumeric)
        // 32 items should be sufficient for standard dictionaries
        let start_idx = limit.saturating_sub(32);
        
        for entry in self.code_ranges[start_idx..limit].iter().rev() {
            if entry.end >= cp {
                mask.insert(entry.primary_id);
                mask = mask.union(entry.compat_mask);
            }
        }

        if mask.is_empty() {
            if let Some(default_id) = self.default_id {
                mask.insert(default_id);
            }
        }
        mask
    }

    fn category_ids_for_char(&self, ch: char) -> SmallVec<[CategoryId; 8]> {
        self.category_mask_for_char(ch).iter().collect()
    }

    fn matches(&self, ch: char) -> Vec<(CategoryId, CategoryMask)> {
        let cp = ch as u32;
        let mut matches = Vec::new();
        
        let limit = self.code_ranges.partition_point(|entry| entry.start <= cp);
        let start_idx = limit.saturating_sub(32);

        for entry in self.code_ranges[start_idx..limit].iter().rev() {
            if entry.end >= cp {
                matches.push((entry.primary_id, entry.compat_mask));
            }
        }

        if matches.is_empty() {
            if let Some(default_id) = self.default_id {
                matches.push((default_id, CategoryMask::empty()));
            }
        }
        matches
    }
}

struct CategoryIterEntry {
    primary: String,
    compat: Vec<String>,
}

pub(crate) struct CharCategoryIter {
    entries: std::vec::IntoIter<CategoryIterEntry>,
}

impl Iterator for CharCategoryIter {
    type Item = (String, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| (entry.primary, entry.compat))
    }
}

impl PackedConnectionMatrix {
    fn from_rows(rows: &[Vec<i16>]) -> Result<Self, RunomeError> {
        if rows.is_empty() {
            return Ok(Self {
                rows: 0,
                cols: 0,
                len: 0,
                storage: ConnectionMatrixStorage::Owned(Arc::from(
                    Vec::<i16>::new().into_boxed_slice(),
                )),
            });
        }

        let cols = rows[0].len();
        if cols == 0 {
            return Ok(Self {
                rows: rows.len() as u32,
                cols: 0,
                len: 0,
                storage: ConnectionMatrixStorage::Owned(Arc::from(
                    Vec::<i16>::new().into_boxed_slice(),
                )),
            });
        }

        if rows.len() > u32::MAX as usize || cols > u32::MAX as usize {
            return Err(RunomeError::DictValidationError {
                reason: format!(
                    "Connection matrix dimensions exceed supported range: {}x{}",
                    rows.len(),
                    cols
                ),
            });
        }

        let mut data = Vec::with_capacity(rows.len().saturating_mul(cols));
        for (idx, row) in rows.iter().enumerate() {
            if row.len() != cols {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Connection matrix row {} has inconsistent length: {} vs expected {}",
                        idx,
                        row.len(),
                        cols
                    ),
                });
            }
            data.extend_from_slice(row);
        }

        let data_arc: Arc<[i16]> = Arc::from(data.into_boxed_slice());

        Ok(Self {
            rows: rows.len() as u32,
            cols: cols as u32,
            len: data_arc.len(),
            storage: ConnectionMatrixStorage::Owned(data_arc),
        })
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn rows(&self) -> usize {
        self.rows as usize
    }

    #[inline]
    fn cols(&self) -> usize {
        self.cols as usize
    }

    #[inline]
    fn get(&self, left_id: u16, right_id: u16) -> Option<i16> {
        let left = left_id as usize;
        let right = right_id as usize;
        let cols = self.cols();
        if left < self.rows() && right < cols {
            let idx = left * cols + right;
            Some(self.as_slice()[idx])
        } else {
            None
        }
    }

    fn row_slice(&self, row: usize) -> Option<&[i16]> {
        if row >= self.rows() {
            return None;
        }
        let cols = self.cols();
        let start = row * cols;
        let end = start + cols;
        Some(&self.as_slice()[start..end])
    }

    fn as_slice(&self) -> &[i16] {
        match &self.storage {
            ConnectionMatrixStorage::Owned(data) => data,
            ConnectionMatrixStorage::Mmap { mmap, data_offset } => {
                let ptr = unsafe { mmap.as_ptr().add(*data_offset) as *const i16 };
                unsafe { std::slice::from_raw_parts(ptr, self.len) }
            }
        }
    }

    fn from_mmap(
        mmap: Arc<Mmap>,
        rows: u32,
        cols: u32,
        data_offset: usize,
    ) -> Result<Self, RunomeError> {
        let len = (rows as usize).checked_mul(cols as usize).ok_or_else(|| {
            RunomeError::DictValidationError {
                reason: format!(
                    "Invalid connection matrix dimensions: rows={}, cols={}",
                    rows, cols
                ),
            }
        })?;

        let total_bytes = len.checked_mul(std::mem::size_of::<i16>()).ok_or_else(|| {
            RunomeError::DictValidationError {
                reason: "Connection matrix size overflow".to_string(),
            }
        })?;

        let available = mmap.len().checked_sub(data_offset).ok_or_else(|| {
            RunomeError::DictValidationError {
                reason: "connections.pack data offset beyond file size".to_string(),
            }
        })?;

        if available != total_bytes {
            return Err(RunomeError::DictValidationError {
                reason: format!(
                    "connections.pack data size mismatch: available={} expected={}",
                    available, total_bytes
                ),
            });
        }

        if data_offset % std::mem::align_of::<i16>() != 0 {
            return Err(RunomeError::DictValidationError {
                reason: format!(
                    "connections.pack data offset {} is not aligned for i16",
                    data_offset
                ),
            });
        }

        Ok(Self {
            rows,
            cols,
            len,
            storage: ConnectionMatrixStorage::Mmap { mmap, data_offset },
        })
    }
}

const EMPTY_U32_SLICE: &[u32] = &[];

impl MorphemeIndexStorage {
    fn len(&self) -> usize {
        match self {
            MorphemeIndexStorage::Owned { offsets, .. } => offsets.len().saturating_sub(1),
            MorphemeIndexStorage::Packed { entry_count, .. } => *entry_count,
        }
    }

    fn get(&self, idx: usize) -> &[u32] {
        match self {
            MorphemeIndexStorage::Owned { offsets, values } => {
                if idx >= offsets.len().saturating_sub(1) {
                    return EMPTY_U32_SLICE;
                }

                let start = offsets[idx] as usize;
                let end = offsets[idx + 1] as usize;

                if start > end || end > values.len() {
                    return EMPTY_U32_SLICE;
                }

                &values[start..end]
            }
            MorphemeIndexStorage::Packed {
                mmap,
                entry_count,
                value_count,
                offsets_offset,
                values_offset,
            } => {
                if idx >= *entry_count {
                    return EMPTY_U32_SLICE;
                }

                let offsets_ptr = unsafe { mmap.as_ptr().add(*offsets_offset) as *const u32 };
                let offsets = unsafe { slice::from_raw_parts(offsets_ptr, entry_count + 1) };

                let start = offsets[idx] as usize;
                let end = offsets[idx + 1] as usize;

                if start > end || end > *value_count {
                    return EMPTY_U32_SLICE;
                }

                let values_ptr = unsafe { mmap.as_ptr().add(*values_offset) as *const u32 };
                let values = unsafe { slice::from_raw_parts(values_ptr, *value_count) };
                &values[start..end]
            }
        }
    }
}

impl MorphemeIndex {
    fn from_vectors(vecs: Vec<Vec<u32>>) -> Self {
        let mut offsets = Vec::with_capacity(vecs.len() + 1);
        let mut values = Vec::new();
        offsets.push(0);

        for ids in vecs {
            values.extend(ids);
            let end = values.len();
            let end_u32 = u32::try_from(end).expect("morpheme index exceeds u32 range");
            offsets.push(end_u32);
        }

        Self {
            storage: MorphemeIndexStorage::Owned { offsets, values },
        }
    }

    fn from_pack(pack: loader::MorphemeIndexPack) -> Result<Self, RunomeError> {
        if pack.offsets_offset % std::mem::size_of::<u32>() != 0
            || pack.values_offset % std::mem::size_of::<u32>() != 0
        {
            return Err(RunomeError::DictValidationError {
                reason: "morpheme_index.pack data is not properly aligned".to_string(),
            });
        }

        let entry_count = pack.entry_count as usize;
        let value_count = pack.value_count as usize;
        let mmap_arc = Arc::new(pack.mmap);

        Ok(Self {
            storage: MorphemeIndexStorage::Packed {
                mmap: mmap_arc,
                entry_count,
                value_count,
                offsets_offset: pack.offsets_offset,
                values_offset: pack.values_offset,
            },
        })
    }

    fn len(&self) -> usize {
        self.storage.len()
    }

    fn get(&self, idx: usize) -> &[u32] {
        self.storage.get(idx)
    }
}

impl<'a> MorphemeIndexView<'a> {
    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn get(&self, idx: usize) -> &'a [u32] {
        self.index.get(idx)
    }
}

impl ConnectionMatrix {
    fn from_packed(matrix: &PackedConnectionMatrix) -> Self {
        Self {
            inner: Arc::new(matrix.clone()),
        }
    }

    pub fn from_rows(rows: &[Vec<i16>]) -> Result<Self, RunomeError> {
        let packed = PackedConnectionMatrix::from_rows(rows)?;
        Ok(Self {
            inner: Arc::new(packed),
        })
    }

    #[inline]
    pub fn rows(&self) -> usize {
        self.inner.rows()
    }

    #[inline]
    pub fn cols(&self) -> usize {
        self.inner.cols()
    }

    #[inline]
    pub fn get(&self, left_id: u16, right_id: u16) -> Option<i16> {
        self.inner.get(left_id, right_id)
    }

    #[inline]
    pub fn row(&self, row: usize) -> Option<&[i16]> {
        self.inner.row_slice(row)
    }
}

impl<'a> ConnectionMatrixView<'a> {
    #[inline]
    pub fn rows(&self) -> usize {
        self.matrix.rows()
    }

    #[inline]
    pub fn cols(&self) -> usize {
        self.matrix.cols()
    }

    #[inline]
    pub fn get(&self, left_id: u16, right_id: u16) -> Option<i16> {
        self.matrix.get(left_id, right_id)
    }

    #[inline]
    pub fn row(&self, row: usize) -> Option<&'a [i16]> {
        self.matrix.row_slice(row)
    }
}

/// Container for all dictionary resources
#[derive(Debug)]
pub struct DictionaryResource {
    entries: Vec<DictEntry>,
    connections: PackedConnectionMatrix,
    char_defs: CharDefinitions,
    unknowns: UnknownEntries,
    morpheme_index: MorphemeIndex,
    category_index: CategoryIndex,
    unknown_entries_by_id: Vec<Box<[UnknownEntry]>>,
}

impl DictionaryResource {
    /// Load all dictionary components from sysdic directory
    pub fn load(sysdic_dir: &Path) -> Result<Self, RunomeError> {
        loader::validate_sysdic_directory(sysdic_dir)?;

        let entries = loader::load_entries(sysdic_dir)?;
        let connections =
            if let Some((mmap, rows, cols)) = loader::load_connections_packed(sysdic_dir)? {
                let mmap_arc = Arc::new(mmap);
                PackedConnectionMatrix::from_mmap(
                    mmap_arc,
                    rows,
                    cols,
                    loader::CONNECTIONS_PACK_HEADER_SIZE,
                )?
            } else {
                let connections_rows = loader::load_connections(sysdic_dir)?;
                PackedConnectionMatrix::from_rows(&connections_rows)?
            };
        let char_defs = loader::load_char_definitions(sysdic_dir)?;
        let unknowns = loader::load_unknown_entries(sysdic_dir)?;
        let morpheme_index = if let Some(pack) = loader::load_morpheme_index_packed(sysdic_dir)? {
            MorphemeIndex::from_pack(pack)?
        } else {
            MorphemeIndex::from_vectors(loader::load_morpheme_index(sysdic_dir)?)
        };
        let category_index = CategoryIndex::new(&char_defs)?;

        let mut unknown_entries_by_id = Vec::with_capacity(category_index.len());
        unknown_entries_by_id.resize_with(category_index.len(), || Vec::new().into_boxed_slice());
        for (name, entries_vec) in unknowns.iter() {
            if let Some(id) = category_index.category_id(name) {
                unknown_entries_by_id[id as usize] = entries_vec.clone().into_boxed_slice();
            }
        }

        Ok(Self {
            entries,
            connections,
            char_defs,
            unknowns,
            morpheme_index,
            category_index,
            unknown_entries_by_id,
        })
    }

    /// Load and validate all dictionary components from sysdic directory
    pub fn load_and_validate(sysdic_dir: &Path) -> Result<Self, RunomeError> {
        let resource = Self::load(sysdic_dir)?;
        resource.validate()?;
        Ok(resource)
    }

    /// Validate the integrity of loaded dictionary data
    pub fn validate(&self) -> Result<(), RunomeError> {
        // Validate entries have reasonable values
        if self.entries.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Dictionary entries are empty".to_string(),
            });
        }

        // Validate connection matrix dimensions
        if self.connections.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Connection matrix is empty".to_string(),
            });
        }

        // Validate character definitions
        if self.char_defs.categories.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Character categories are empty".to_string(),
            });
        }

        if self.char_defs.code_ranges.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Character code ranges are empty".to_string(),
            });
        }

        // Validate that all code ranges reference existing categories
        for range in &self.char_defs.code_ranges {
            if !self.char_defs.categories.contains_key(&range.category) {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Code range references non-existent category: {}",
                        range.category
                    ),
                });
            }
        }

        // Validate entry IDs are within reasonable bounds for connection matrix
        let max_id = self.connections.rows().checked_sub(1).ok_or_else(|| {
            RunomeError::DictValidationError {
                reason: "Connection matrix must have at least one row".to_string(),
            }
        })? as u16;
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.left_id > max_id {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Entry {} has left_id {} exceeding connection matrix bounds (max: {})",
                        i, entry.left_id, max_id
                    ),
                });
            }
            if entry.right_id > max_id {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Entry {} has right_id {} exceeding connection matrix bounds (max: {})",
                        i, entry.right_id, max_id
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get all dictionary entries
    pub fn get_entries(&self) -> &[DictEntry] {
        &self.entries
    }

    /// Get connection cost between left and right part-of-speech IDs
    pub fn get_connection_cost(&self, left_id: u16, right_id: u16) -> Result<i16, RunomeError> {
        self.connections
            .get(left_id, right_id)
            .ok_or(RunomeError::InvalidConnectionId { left_id, right_id })
    }

    /// Get connection matrix for user dictionary use.
    ///
    /// Returns a shared handle that allows zero-copy access to connection costs.
    pub fn connection_matrix(&self) -> ConnectionMatrix {
        ConnectionMatrix::from_packed(&self.connections)
    }

    /// Borrowed view of the connection matrix for lightweight read-only access.
    pub fn connection_matrix_view(&self) -> ConnectionMatrixView<'_> {
        ConnectionMatrixView {
            matrix: &self.connections,
        }
    }

    /// Get character category for a given character (returns first match)
    pub fn get_char_category(&self, ch: char) -> Option<&CharCategory> {
        let ids = self.category_index.category_ids_for_char(ch);
        ids.into_iter().find_map(|id| {
            let name = self.category_index.category_name(id);
            self.char_defs.categories.get(name)
        })
    }

    #[inline]
    pub(crate) fn iter_char_categories(&self, ch: char) -> CharCategoryIter {
        let matches = self
            .category_index
            .matches(ch)
            .into_iter()
            .map(|(primary, compat_mask)| {
                let primary_name = self.category_index.category_name(primary).to_string();
                let compat_names = compat_mask
                    .iter()
                    .map(|id| self.category_index.category_name(id).to_string())
                    .collect();
                CategoryIterEntry {
                    primary: primary_name,
                    compat: compat_names,
                }
            })
            .collect::<Vec<_>>();

        CharCategoryIter {
            entries: matches.into_iter(),
        }
    }

    /// Get all character categories for a given character
    /// Returns a HashMap where keys are category names and values are compatible categories
    /// This matches the Python SystemDictionary.get_char_categories() behavior
    pub fn get_char_categories(&self, ch: char) -> std::collections::HashMap<String, Vec<String>> {
        let mut result = std::collections::HashMap::new();

        for (category, compat) in self.iter_char_categories(ch) {
            result.insert(category, compat);
        }

        // Default category if no matches found
        if result.is_empty() {
            result.insert("DEFAULT".to_string(), Vec::new());
        }

        result
    }

    /// Get unknown entries for a specific category
    pub fn get_unknown_entries(&self, category: &str) -> Option<&[UnknownEntry]> {
        self.unknowns.get(category).map(|v| v.as_slice())
    }

    pub fn get_unknown_entries_by_id(&self, category: CategoryId) -> Option<&[UnknownEntry]> {
        self.unknown_entries_by_id
            .get(category as usize)
            .and_then(|entries| {
                if entries.is_empty() {
                    None
                } else {
                    Some(entries.as_ref())
                }
            })
    }

    pub(crate) fn get_char_category_mask(&self, ch: char) -> CategoryMask {
        self.category_index.category_mask_for_char(ch)
    }

    pub fn get_char_category_ids(&self, ch: char) -> Vec<CategoryId> {
        self.category_index
            .category_mask_for_char(ch)
            .iter()
            .collect()
    }

    pub fn category_name_by_id(&self, category: CategoryId) -> &str {
        self.category_index.category_name(category)
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.category_index.category_id(name)
    }

    /// Get morpheme index for mapping FST index IDs to vectors of morpheme IDs
    ///
    /// This method now returns a zero-copy view over the morpheme index data.
    /// Callers that require an owned representation can collect from the view.
    pub fn get_morpheme_index(&self) -> MorphemeIndexView<'_> {
        self.morpheme_index_view()
    }

    pub fn morpheme_index_view(&self) -> MorphemeIndexView<'_> {
        MorphemeIndexView {
            index: &self.morpheme_index,
        }
    }

    /// Check if unknown word processing should always be invoked for category
    pub fn unknown_invoked_always(&self, category: &str) -> bool {
        self.category_index
            .category_id(category)
            .map(|id| self.category_index.flags(id).invoke)
            .unwrap_or(false)
    }

    pub fn unknown_invoked_always_by_id(&self, category: CategoryId) -> bool {
        self.category_index.flags(category).invoke
    }

    /// Check if characters of this category should be grouped together
    pub fn unknown_grouping(&self, category: &str) -> bool {
        self.category_index
            .category_id(category)
            .map(|id| self.category_index.flags(id).group)
            .unwrap_or(false)
    }

    pub fn unknown_grouping_by_id(&self, category: CategoryId) -> bool {
        self.category_index.flags(category).group
    }

    /// Get length constraint for unknown words of this category
    pub fn unknown_length(&self, category: &str) -> i32 {
        self.category_index
            .category_id(category)
            .map(|id| i32::from(self.category_index.flags(id).length))
            .unwrap_or(-1)
    }

    pub fn unknown_length_by_id(&self, category: CategoryId) -> i32 {
        i32::from(self.category_index.flags(category).length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_sysdic_path() -> PathBuf {
        // Assuming tests are run from the project root
        PathBuf::from("sysdic")
    }

    #[test]
    fn test_load_dictionary_success() {
        let sysdic_path = get_test_sysdic_path();

        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let result = DictionaryResource::load(&sysdic_path);
        assert!(result.is_ok(), "Failed to load dictionary");

        let dict = result.unwrap();

        // Verify all components were loaded and are non-empty
        assert!(
            !dict.entries.is_empty(),
            "Dictionary entries should not be empty"
        );
        assert!(
            !dict.connections.is_empty(),
            "Connection matrix should not be empty"
        );
        assert!(
            !dict.char_defs.categories.is_empty(),
            "Character categories should not be empty"
        );
        assert!(
            !dict.char_defs.code_ranges.is_empty(),
            "Character code ranges should not be empty"
        );
        // Verify reasonable data sizes
        assert!(
            dict.entries.len() > 1000,
            "Should have substantial number of entries"
        );
        assert!(
            dict.connections.rows() > 100,
            "Should have substantial connection matrix"
        );
        assert!(
            dict.char_defs.categories.len() > 5,
            "Should have multiple character categories"
        );
        assert!(
            dict.char_defs.code_ranges.len() > 10,
            "Should have multiple code ranges"
        );
    }

    #[test]
    fn test_load_and_validate_success() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let result = DictionaryResource::load_and_validate(&sysdic_path);
        assert!(result.is_ok(), "Failed to load and validate dictionary.");

        // Dictionary loaded and validated successfully
    }

    #[test]
    fn test_validate_data() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");
        let validation_result = dict.validate();

        assert!(
            validation_result.is_ok(),
            "Dictionary validation failed: {:?}",
            validation_result
        );
        // Dictionary validation passed
    }

    #[test]
    fn test_get_entries() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");
        let entries = dict.get_entries();

        assert!(!entries.is_empty(), "Should have dictionary entries");

        // Verify entries have required fields
        for entry in entries.iter().take(5) {
            assert!(
                !entry.part_of_speech.is_empty(),
                "Entry should have part of speech"
            );
            assert!(!entry.reading.is_empty(), "Entry should have reading");
        }
    }

    #[test]
    fn test_connection_costs() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Test some valid connection costs
        let cost_result = dict.get_connection_cost(0, 0);
        assert!(
            cost_result.is_ok(),
            "Should be able to get connection cost for valid indices"
        );

        let cost = cost_result.unwrap();
        assert!(
            (-10000..=10000).contains(&cost),
            "Connection cost should be reasonable: {}",
            cost
        );

        // Test boundary cases
        let max_id = (dict.connections.rows() - 1) as u16;
        let boundary_cost = dict.get_connection_cost(max_id, max_id);
        assert!(
            boundary_cost.is_ok(),
            "Should be able to get connection cost for boundary indices"
        );

        // Test invalid indices
        let invalid_cost = dict.get_connection_cost(max_id + 1, 0);
        assert!(invalid_cost.is_err(), "Should fail for invalid indices");
    }

    #[test]
    fn test_char_categories() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Test some common characters have categories
        let test_chars = ['あ', 'ア', '漢', 'A', '1'];

        for ch in test_chars {
            let category = dict.get_char_category(ch);
            assert!(
                category.is_some(),
                "Character '{}' should have a category",
                ch
            );

            let cat = category.unwrap();
            assert!(
                cat.length <= 10,
                "Character '{}' category length should be reasonable: {}",
                ch,
                cat.length
            );
        }
    }

    #[test]
    fn test_char_category_mask_matches_ids() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        let sample_chars = ['は', '漢', 'A', '0', '🙂'];
        for ch in sample_chars {
            let ids_from_api = dict.get_char_category_ids(ch);
            let mut ids_from_mask: Vec<_> = dict.get_char_category_mask(ch).iter().collect();
            ids_from_mask.sort_unstable();
            let mut ids_api_sorted = ids_from_api.clone();
            ids_api_sorted.sort_unstable();
            assert_eq!(
                ids_from_mask, ids_api_sorted,
                "Category mask mismatch for character '{}'",
                ch
            );
        }
    }

    #[test]
    fn test_get_char_categories_multiple() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Test specific characters and their expected categories
        let categories_ha = dict.get_char_categories('は');
        assert!(
            categories_ha.contains_key("HIRAGANA"),
            "Character 'は' should have HIRAGANA category"
        );
        assert_eq!(
            categories_ha.get("HIRAGANA").unwrap(),
            &Vec::<String>::new(),
            "HIRAGANA category should have empty compatible categories"
        );

        let categories_ka = dict.get_char_categories('ハ');
        assert!(
            categories_ka.contains_key("KATAKANA"),
            "Character 'ハ' should have KATAKANA category"
        );

        // Test character that should have multiple categories (五 = KANJI + KANJINUMERIC)
        let categories_go = dict.get_char_categories('五');
        assert!(
            categories_go.contains_key("KANJI"),
            "Character '五' should have KANJI category"
        );
        assert!(
            categories_go.contains_key("KANJINUMERIC"),
            "Character '五' should have KANJINUMERIC category"
        );

        // KANJINUMERIC should have KANJI as compatible category
        let kanjinumeric_compat = categories_go.get("KANJINUMERIC").unwrap();
        assert!(
            kanjinumeric_compat.contains(&"KANJI".to_string()),
            "KANJINUMERIC should have KANJI as compatible category"
        );

        // Test DEFAULT category for unknown character
        let categories_unknown = dict.get_char_categories('𠮷'); // Rare kanji
        if categories_unknown.len() == 1 && categories_unknown.contains_key("DEFAULT") {
            assert_eq!(
                categories_unknown.get("DEFAULT").unwrap(),
                &Vec::<String>::new(),
                "DEFAULT category should have empty compatible categories"
            );
        }
    }

    #[test]
    fn test_unknown_word_properties() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Test known category properties based on char.def
        assert!(!dict.unknown_invoked_always("HIRAGANA"));
        assert!(dict.unknown_grouping("HIRAGANA"));
        assert_eq!(dict.unknown_length("HIRAGANA"), 2);

        assert!(dict.unknown_invoked_always("KATAKANA"));
        assert!(dict.unknown_grouping("KATAKANA"));
        assert_eq!(dict.unknown_length("KATAKANA"), 2);

        assert!(!dict.unknown_invoked_always("KANJI"));
        assert!(!dict.unknown_grouping("KANJI"));
        assert_eq!(dict.unknown_length("KANJI"), 2);

        assert!(dict.unknown_invoked_always("ALPHA"));
        assert!(dict.unknown_grouping("ALPHA"));
        assert_eq!(dict.unknown_length("ALPHA"), 0);

        assert!(dict.unknown_invoked_always("NUMERIC"));
        assert!(dict.unknown_grouping("NUMERIC"));
        assert_eq!(dict.unknown_length("NUMERIC"), 0);

        // Test non-existent category
        assert!(!dict.unknown_invoked_always("NONEXISTENT"));
        assert!(!dict.unknown_grouping("NONEXISTENT"));
        assert_eq!(dict.unknown_length("NONEXISTENT"), -1);
    }

    #[test]
    fn test_unknown_entries() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Verify unknown entry categories exist and have entries
        assert!(
            !dict.unknowns.is_empty(),
            "Should have unknown entry categories"
        );

        for category in dict.unknowns.keys() {
            let entries = dict.get_unknown_entries(category).unwrap();
            assert!(
                !entries.is_empty(),
                "Category '{}' should have entries",
                category
            );

            // Verify entry structure
            for entry in entries {
                assert!(
                    !entry.part_of_speech.is_empty(),
                    "Unknown entry should have part of speech"
                );
            }
        }

        // Test a non-existent category
        let nonexistent = dict.get_unknown_entries("NONEXISTENT_CATEGORY");
        assert!(
            nonexistent.is_none(),
            "Should return None for non-existent category"
        );
    }

    #[test]
    fn test_load_missing_directory() {
        let nonexistent_dir = PathBuf::from("/definitely/nonexistent/directory");
        let result = DictionaryResource::load(&nonexistent_dir);
        assert!(
            result.is_err(),
            "Should fail when loading non-existent directory"
        );

        if let Err(error) = result {
            match error {
                RunomeError::DictDirectoryNotFound { .. } => {
                    // Correctly detected missing directory
                }
                _ => panic!("Expected DictDirectoryNotFound error, got: {:?}", error),
            }
        }
    }

    #[test]
    fn test_data_consistency() {
        let sysdic_path = get_test_sysdic_path();

        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let dict = DictionaryResource::load(&sysdic_path).expect("Failed to load dictionary");

        // Verify connection matrix is square
        let matrix_view = dict.connection_matrix_view();
        for row_index in 0..matrix_view.rows() {
            let row = matrix_view
                .row(row_index)
                .expect("Connection matrix row missing");
            assert_eq!(
                row.len(),
                matrix_view.cols(),
                "Connection matrix row {} has inconsistent length",
                row_index
            );
        }

        // Verify all entries have valid connection IDs
        let max_id = if matrix_view.rows() > 0 {
            (matrix_view.rows() - 1) as u16
        } else {
            0
        };
        for (i, entry) in dict.entries.iter().enumerate() {
            assert!(
                entry.left_id <= max_id,
                "Entry {} has left_id {} exceeding matrix bounds (max: {})",
                i,
                entry.left_id,
                max_id
            );
            assert!(
                entry.right_id <= max_id,
                "Entry {} has right_id {} exceeding matrix bounds (max: {})",
                i,
                entry.right_id,
                max_id
            );
        }

        // Verify character code ranges reference existing categories
        for range in &dict.char_defs.code_ranges {
            assert!(
                dict.char_defs.categories.contains_key(&range.category),
                "Code range references non-existent category: {}",
                range.category
            );
        }

        // Data consistency checks completed successfully
    }
}
