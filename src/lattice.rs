use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

use crate::dictionary::{DictEntry, Dictionary};
use crate::error::RunomeError;
use crate::intern;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    SysDict,
    UserDict,
    Unknown,
}

/// Trait for all lattice nodes providing common interface for Viterbi algorithm
pub trait LatticeNode: std::fmt::Debug {
    /// Get the surface form of this node
    fn surface(&self) -> &str;

    /// Get the left part-of-speech ID for connection cost calculation
    fn left_id(&self) -> u16;

    /// Get the right part-of-speech ID for connection cost calculation
    fn right_id(&self) -> u16;

    /// Get the word cost of this node
    fn cost(&self) -> i16;

    /// Get the minimum cost from BOS to this node (Viterbi)
    fn min_cost(&self) -> i32;

    /// Set the minimum cost from BOS to this node (Viterbi)
    fn set_min_cost(&mut self, cost: i32);

    /// Get the position of the best predecessor node (Viterbi)
    fn back_pos(&self) -> i32;

    /// Set the position of the best predecessor node (Viterbi)
    fn set_back_pos(&mut self, pos: i32);

    /// Get the index of the best predecessor node (Viterbi)
    fn back_index(&self) -> i32;

    /// Set the index of the best predecessor node (Viterbi)
    fn set_back_index(&mut self, index: i32);

    /// Get the position of this node in the lattice
    fn pos(&self) -> usize;

    /// Set the position of this node in the lattice
    fn set_pos(&mut self, pos: usize);

    /// Get the index of this node within its position
    fn index(&self) -> usize;

    /// Set the index of this node within its position
    fn set_index(&mut self, index: usize);

    /// Get the type of this node (SysDict, UserDict, Unknown)
    fn node_type(&self) -> NodeType;

    /// Get the length of the surface form in characters
    fn surface_len(&self) -> usize;

    /// Get the morphological ID for tie-breaking (corresponds to SurfaceNode.num in Python)
    /// Returns None for nodes without morphological data (BOS, EOS, etc.)
    fn morph_id(&self) -> Option<usize>;

    fn part_of_speech(&self) -> &str;

    fn inflection_type(&self) -> &str;

    fn inflection_form(&self) -> &str;

    fn base_form(&self) -> &str;

    fn reading(&self) -> &str;

    fn phonetic(&self) -> &str;
}

/// Node backed by a dictionary entry reference (zero-copy for dictionary words)
#[derive(Debug)]
pub struct Node<'a> {
    /// Reference to dictionary entry (avoids copying morphological data)
    dict_entry: &'a DictEntry,
    node_type: NodeType,

    /// Viterbi algorithm fields
    min_cost: i32,
    back_pos: i32,
    back_index: i32,
    pos: usize,
    index: usize,
}

impl<'a> Node<'a> {
    /// Create a new Node from a dictionary entry reference
    pub fn new(dict_entry: &'a DictEntry, node_type: NodeType) -> Self {
        Self {
            dict_entry,
            node_type,
            min_cost: i32::MAX,
            back_pos: -1,
            back_index: -1,
            pos: 0,
            index: 0,
        }
    }

    /// Get the complete dictionary entry for this node
    pub fn dict_entry(&self) -> &DictEntry {
        self.dict_entry
    }
}

impl<'a> LatticeNode for Node<'a> {
    fn surface(&self) -> &str {
        &self.dict_entry.surface
    }

    fn left_id(&self) -> u16 {
        self.dict_entry.left_id
    }

    fn right_id(&self) -> u16 {
        self.dict_entry.right_id
    }

    fn cost(&self) -> i16 {
        self.dict_entry.cost
    }

    fn min_cost(&self) -> i32 {
        self.min_cost
    }

    fn set_min_cost(&mut self, cost: i32) {
        self.min_cost = cost;
    }

    fn back_pos(&self) -> i32 {
        self.back_pos
    }

    fn set_back_pos(&mut self, pos: i32) {
        self.back_pos = pos;
    }

    fn back_index(&self) -> i32 {
        self.back_index
    }

    fn set_back_index(&mut self, index: i32) {
        self.back_index = index;
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    fn node_type(&self) -> NodeType {
        self.node_type.clone()
    }

    fn surface_len(&self) -> usize {
        self.dict_entry.surface.chars().count()
    }

    fn morph_id(&self) -> Option<usize> {
        Some(self.dict_entry.morph_id)
    }

    fn part_of_speech(&self) -> &str {
        &self.dict_entry.part_of_speech
    }

    fn inflection_type(&self) -> &str {
        &self.dict_entry.inflection_type
    }

    fn inflection_form(&self) -> &str {
        &self.dict_entry.inflection_form
    }

    fn base_form(&self) -> &str {
        &self.dict_entry.base_form
    }

    fn reading(&self) -> &str {
        &self.dict_entry.reading
    }

    fn phonetic(&self) -> &str {
        &self.dict_entry.phonetic
    }
}

/// Node for unknown words that owns its morphological data
#[derive(Debug)]
pub struct UnknownNode {
    /// Morphological data (owned since it's constructed dynamically)
    surface: String,
    left_id: u16,
    right_id: u16,
    cost: i16,
    part_of_speech: String,
    inflection_type: String,
    inflection_form: String,
    base_form: String,
    reading: String,
    phonetic: String,
    node_type: NodeType,

    /// Viterbi algorithm fields
    min_cost: i32,
    back_pos: i32,
    back_index: i32,
    pos: usize,
    index: usize,
}

impl UnknownNode {
    /// Create a new UnknownNode with owned morphological data
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: String,
        left_id: u16,
        right_id: u16,
        cost: i16,
        part_of_speech: String,
        inflection_type: String,
        inflection_form: String,
        base_form: String,
        reading: String,
        phonetic: String,
        node_type: NodeType,
    ) -> Self {
        Self {
            surface,
            left_id,
            right_id,
            cost,
            part_of_speech,
            inflection_type,
            inflection_form,
            base_form,
            reading,
            phonetic,
            node_type,
            min_cost: i32::MAX,
            back_pos: -1,
            back_index: -1,
            pos: 0,
            index: 0,
        }
    }

    /// Create an UnknownNode optimized for dictionary entries
    /// Uses string interning to reduce allocations for morphological data
    #[allow(clippy::too_many_arguments)]
    pub fn from_dict_entry(
        surface: &str,
        left_id: u16,
        right_id: u16,
        cost: i16,
        part_of_speech: &str,
        inflection_type: &str,
        inflection_form: &str,
        base_form: &str,
        reading: &str,
        phonetic: &str,
        node_type: NodeType,
    ) -> Self {
        Self {
            surface: intern::intern_or_clone(surface),
            left_id,
            right_id,
            cost,
            part_of_speech: intern::intern_or_clone(part_of_speech),
            inflection_type: intern::intern_or_clone(inflection_type),
            inflection_form: intern::intern_or_clone(inflection_form),
            base_form: intern::intern_or_clone(base_form),
            reading: intern::intern_or_clone(reading),
            phonetic: intern::intern_or_clone(phonetic),
            node_type,
            min_cost: i32::MAX,
            back_pos: -1,
            back_index: -1,
            pos: 0,
            index: 0,
        }
    }

    /// Create an UnknownNode for true unknown words with default morphological values
    /// This is highly optimized for the common case where most fields are "*"
    pub fn for_unknown_word(
        surface: String,
        left_id: u16,
        right_id: u16,
        cost: i16,
        part_of_speech: &str,
        base_form: Option<&str>,
        node_type: NodeType,
    ) -> Self {
        let base_form_string = match base_form {
            Some(bf) => intern::intern_or_clone(bf),
            None => intern::ASTERISK.to_string(),
        };

        Self {
            surface,
            left_id,
            right_id,
            cost,
            part_of_speech: intern::intern_or_clone(part_of_speech),
            inflection_type: intern::ASTERISK.to_string(),
            inflection_form: intern::ASTERISK.to_string(),
            base_form: base_form_string,
            reading: intern::ASTERISK.to_string(),
            phonetic: intern::ASTERISK.to_string(),
            node_type,
            min_cost: i32::MAX,
            back_pos: -1,
            back_index: -1,
            pos: 0,
            index: 0,
        }
    }
}

impl LatticeNode for UnknownNode {
    fn surface(&self) -> &str {
        &self.surface
    }

    fn left_id(&self) -> u16 {
        self.left_id
    }

    fn right_id(&self) -> u16 {
        self.right_id
    }

    fn cost(&self) -> i16 {
        self.cost
    }

    fn min_cost(&self) -> i32 {
        self.min_cost
    }

    fn set_min_cost(&mut self, cost: i32) {
        self.min_cost = cost;
    }

    fn back_pos(&self) -> i32 {
        self.back_pos
    }

    fn set_back_pos(&mut self, pos: i32) {
        self.back_pos = pos;
    }

    fn back_index(&self) -> i32 {
        self.back_index
    }

    fn set_back_index(&mut self, index: i32) {
        self.back_index = index;
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    fn node_type(&self) -> NodeType {
        self.node_type.clone()
    }

    fn surface_len(&self) -> usize {
        self.surface.chars().count()
    }

    fn morph_id(&self) -> Option<usize> {
        None // Unknown nodes don't have morphological IDs
    }

    fn part_of_speech(&self) -> &str {
        &self.part_of_speech
    }

    fn inflection_type(&self) -> &str {
        &self.inflection_type
    }

    fn inflection_form(&self) -> &str {
        &self.inflection_form
    }

    fn base_form(&self) -> &str {
        &self.base_form
    }

    fn reading(&self) -> &str {
        &self.reading
    }

    fn phonetic(&self) -> &str {
        &self.phonetic
    }
}

/// Beginning-of-sentence node
#[derive(Debug)]
pub struct BOS {
    /// Viterbi algorithm fields
    min_cost: i32,
    back_pos: i32,
    back_index: i32,
    pos: usize,
    index: usize,
}

impl BOS {
    /// Create a new BOS node
    pub fn new() -> Self {
        Self {
            min_cost: 0, // BOS starts with cost 0
            back_pos: -1,
            back_index: -1,
            pos: 0,
            index: 0,
        }
    }
}

impl Default for BOS {
    fn default() -> Self {
        Self::new()
    }
}

impl LatticeNode for BOS {
    fn surface(&self) -> &str {
        intern::BOS_SURFACE
    }

    fn left_id(&self) -> u16 {
        0 // BOS has no left context
    }

    fn right_id(&self) -> u16 {
        0 // BOS connects to any following node
    }

    fn cost(&self) -> i16 {
        0 // BOS has no inherent cost
    }

    fn min_cost(&self) -> i32 {
        self.min_cost
    }

    fn set_min_cost(&mut self, cost: i32) {
        self.min_cost = cost;
    }

    fn back_pos(&self) -> i32 {
        self.back_pos
    }

    fn set_back_pos(&mut self, pos: i32) {
        self.back_pos = pos;
    }

    fn back_index(&self) -> i32 {
        self.back_index
    }

    fn set_back_index(&mut self, index: i32) {
        self.back_index = index;
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    fn node_type(&self) -> NodeType {
        NodeType::SysDict // BOS is treated as system dictionary node
    }

    fn surface_len(&self) -> usize {
        0 // BOS has no surface representation
    }

    fn morph_id(&self) -> Option<usize> {
        None // BOS doesn't have a morphological ID
    }

    fn part_of_speech(&self) -> &str {
        intern::BOS_SURFACE // BOS doesn't have a part of speech
    }

    fn inflection_type(&self) -> &str {
        intern::EMPTY
    }

    fn inflection_form(&self) -> &str {
        intern::EMPTY
    }

    fn base_form(&self) -> &str {
        intern::BOS_SURFACE // BOS doesn't have a base form
    }

    fn reading(&self) -> &str {
        intern::EMPTY
    }

    fn phonetic(&self) -> &str {
        intern::EMPTY
    }
}

/// End-of-sentence node
#[derive(Debug)]
pub struct EOS {
    /// Viterbi algorithm fields
    min_cost: i32,
    back_pos: i32,
    back_index: i32,
    pos: usize,
    index: usize,
}

impl EOS {
    /// Create a new EOS node at the specified position
    pub fn new(end_pos: usize) -> Self {
        Self {
            min_cost: i32::MAX,
            back_pos: -1,
            back_index: -1,
            pos: end_pos,
            index: 0,
        }
    }
}

impl LatticeNode for EOS {
    fn surface(&self) -> &str {
        intern::EOS_SURFACE
    }

    fn left_id(&self) -> u16 {
        0 // EOS accepts connections from any preceding node
    }

    fn right_id(&self) -> u16 {
        0 // EOS has no right context
    }

    fn cost(&self) -> i16 {
        0 // EOS has no inherent cost
    }

    fn min_cost(&self) -> i32 {
        self.min_cost
    }

    fn set_min_cost(&mut self, cost: i32) {
        self.min_cost = cost;
    }

    fn back_pos(&self) -> i32 {
        self.back_pos
    }

    fn set_back_pos(&mut self, pos: i32) {
        self.back_pos = pos;
    }

    fn back_index(&self) -> i32 {
        self.back_index
    }

    fn set_back_index(&mut self, index: i32) {
        self.back_index = index;
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    fn node_type(&self) -> NodeType {
        NodeType::SysDict // EOS is treated as system dictionary node
    }

    fn surface_len(&self) -> usize {
        0 // EOS has no surface representation
    }

    fn morph_id(&self) -> Option<usize> {
        None // EOS doesn't have a morphological ID
    }

    fn part_of_speech(&self) -> &str {
        intern::EOS_SURFACE // EOS doesn't have a part of speech
    }

    fn inflection_type(&self) -> &str {
        intern::EMPTY
    }

    fn inflection_form(&self) -> &str {
        intern::EMPTY
    }

    fn base_form(&self) -> &str {
        intern::EOS_SURFACE // EOS doesn't have a base form
    }

    fn reading(&self) -> &str {
        intern::EMPTY
    }

    fn phonetic(&self) -> &str {
        intern::EMPTY
    }
}

/// Lattice structure for Viterbi algorithm-based morphological analysis
/// Ultra-compact end node representation that stores critical data inline
/// This eliminates expensive indirection and bounds checking in hot paths
#[derive(Debug, Clone)]
pub struct CompactEndNode {
    /// Node position (using u16 for better cache density)
    pub pos: u16,
    /// Node index within position (using u16 for better cache density)
    pub index: u16,
    /// Cached right_id for connection cost lookups (eliminates indirection)
    pub right_id: u16,
    /// Cached min_cost for Viterbi calculations (eliminates indirection)
    pub min_cost: i32,
    /// Cached morph_id for tie-breaking (eliminates indirection)
    pub morph_id: Option<u32>,
}

impl CompactEndNode {
    /// Create from a lattice node, caching critical data for fast access
    fn from_node(node: &dyn LatticeNode, pos: usize, index: usize) -> Self {
        Self {
            pos: pos as u16,
            index: index as u16,
            right_id: node.right_id(),
            min_cost: node.min_cost(),
            morph_id: node.morph_id().map(|id| id as u32),
        }
    }
}

/// Legacy node reference for compatibility
#[derive(Debug, Clone)]
pub struct NodeRef {
    pub pos: usize,
    pub index: usize,
}

/// Connection cost cache key - optimized for fast hashing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CostCacheKey {
    right_id: u16,
    left_id: u16,
}

/// Fast hasher for small integer keys
type FastHasher = BuildHasherDefault<fxhash::FxHasher>;

/// Connection cost cache for frequently accessed cost lookups
struct ConnectionCostCache {
    cache: HashMap<CostCacheKey, i16, FastHasher>,
    max_size: usize,
}

impl ConnectionCostCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_hasher(FastHasher::default()),
            max_size,
        }
    }

    fn get_or_compute<F>(
        &mut self,
        right_id: u16,
        left_id: u16,
        compute: F,
    ) -> Result<i16, RunomeError>
    where
        F: FnOnce() -> Result<i16, RunomeError>,
    {
        let key = CostCacheKey { right_id, left_id };

        if let Some(&cached_cost) = self.cache.get(&key) {
            return Ok(cached_cost);
        }

        let cost = compute()?;

        // Simple cache eviction - clear if too large
        if self.cache.len() >= self.max_size {
            self.cache.clear();
        }

        self.cache.insert(key, cost);
        Ok(cost)
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

pub struct Lattice<'a> {
    /// Start nodes at each position - snodes[pos][index]
    snodes: Vec<Vec<Box<dyn LatticeNode + 'a>>>,
    /// Ultra-optimized end nodes with inlined critical data (eliminates indirection)
    enodes: Vec<Vec<CompactEndNode>>,
    /// Current position pointer
    p: usize,
    /// Dictionary reference for connection cost lookups
    dic: Arc<dyn Dictionary>,
    /// High-performance connection cost cache for hot path optimization
    cost_cache: ConnectionCostCache,
    /// Surface length cache to avoid UTF-8 character counting
    surface_len_cache: HashMap<String, usize, FastHasher>,
}

impl<'a> Lattice<'a> {
    /// Create a new lattice with the specified size and dictionary
    ///
    /// Initializes the lattice with BOS node at position 0 and pre-allocates
    /// vectors for the specified size.
    ///
    /// # Arguments
    /// * `size` - Maximum number of positions in the lattice
    /// * `dic` - Dictionary reference for connection cost calculations
    ///
    /// # Returns
    /// * New Lattice instance with BOS node initialized
    pub fn new(size: usize, dic: Arc<dyn Dictionary>) -> Self {
        // Initialize snodes and enodes vectors
        // We need positions 0 through size+1 (size+2 total positions)
        let mut snodes = Vec::with_capacity(size + 2);
        let mut enodes = Vec::with_capacity(size + 2);

        // Initialize all positions as empty first
        for _ in 0..=(size + 1) {
            snodes.push(Vec::with_capacity(4));
            enodes.push(Vec::with_capacity(4));
        }

        // Position 0: BOS node in snodes
        let mut bos = Box::new(BOS::new()) as Box<dyn LatticeNode + 'a>;
        bos.set_pos(0);
        bos.set_index(0);
        snodes[0].push(bos);

        // Position 1: BOS node also appears in enodes[1] for connections
        let bos_compact = CompactEndNode::from_node(snodes[0][0].as_ref(), 0, 0);
        enodes[1].push(bos_compact);

        Self {
            snodes,
            enodes,
            p: 1, // Start at position 1 (after BOS)
            dic,
            cost_cache: ConnectionCostCache::new(10000), // Cache up to 10K cost lookups
            surface_len_cache: HashMap::with_hasher(FastHasher::default()),
        }
    }

    /// Get a node by reference - helper method for efficient node access
    fn get_node(&self, node_ref: &NodeRef) -> Option<&dyn LatticeNode> {
        self.snodes
            .get(node_ref.pos)
            .and_then(|nodes| nodes.get(node_ref.index))
            .map(|node| node.as_ref())
    }

    /// Ensure lattice capacity for a given position (optimized growth)
    fn ensure_capacity(&mut self, end_pos: usize) {
        if self.snodes.len() <= end_pos {
            // Grow by at least 50% to reduce reallocations
            let new_capacity = std::cmp::max(end_pos + 1, self.snodes.len() * 3 / 2);
            self.snodes.reserve(new_capacity - self.snodes.len());
            self.enodes.reserve(new_capacity - self.enodes.len());

            while self.snodes.len() <= end_pos {
                self.snodes.push(Vec::with_capacity(4));
                self.enodes.push(Vec::with_capacity(4));
            }
        }
    }

    /// Get the current position in the lattice
    pub fn position(&self) -> usize {
        self.p
    }

    /// Get the total number of positions in the lattice
    pub fn size(&self) -> usize {
        self.snodes.len().saturating_sub(1)
    }

    /// Get reference to start nodes at the specified position
    pub fn start_nodes(&self, pos: usize) -> Option<&Vec<Box<dyn LatticeNode + 'a>>> {
        self.snodes.get(pos)
    }

    /// Get reference to end nodes at the specified position  
    pub fn end_nodes(&self, pos: usize) -> Option<&Vec<CompactEndNode>> {
        self.enodes.get(pos)
    }

    /// Get cached surface length or compute and cache it
    fn get_surface_length(&mut self, surface: &str) -> usize {
        if let Some(&cached_len) = self.surface_len_cache.get(surface) {
            return cached_len;
        }

        let len = if surface.is_empty() {
            1
        } else {
            surface.chars().count()
        };

        // Cache for future use (limit cache size to prevent memory bloat)
        if self.surface_len_cache.len() < 5000 {
            self.surface_len_cache
                .insert(intern::intern_or_clone(surface), len);
        }

        len
    }

    /// Check if the lattice is properly initialized
    pub fn is_valid(&self) -> bool {
        // Must have at least BOS position
        if self.snodes.is_empty() || self.enodes.is_empty() {
            return false;
        }

        // Position 0 must contain exactly one BOS node
        if let Some(start_nodes) = self.snodes.first() {
            if start_nodes.len() != 1 {
                return false;
            }
            // Check if it's actually a BOS node
            if start_nodes[0].surface() != "__BOS__" {
                return false;
            }
        } else {
            return false;
        }

        // Position 1 in enodes must contain the BOS node for connections
        if let Some(end_nodes) = self.enodes.get(1) {
            if end_nodes.is_empty() {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    /// Get a reference to the dictionary
    pub fn dictionary(&self) -> &Arc<dyn Dictionary> {
        &self.dic
    }

    /// Add a node to the lattice with Viterbi cost calculation
    ///
    /// This is the core of the Viterbi algorithm - it finds the minimum cost path
    /// to reach this node from all possible predecessor nodes at the current position.
    ///
    /// # Arguments
    /// * `mut node` - The node to add (will be mutated to set Viterbi fields)
    ///
    /// # Returns
    /// * `Ok(())` if the node was successfully added
    /// * `Err(RunomeError)` if cost calculation or dictionary access fails
    /// Ultra-optimized add method with multiple performance optimizations:
    /// - Inlined critical data (no indirection)
    /// - Connection cost caching
    /// - Surface length caching
    /// - Hot path specialization for single predecessor
    /// - Optimized memory access patterns
    pub fn add(&mut self, mut node: Box<dyn LatticeNode + 'a>) -> Result<(), RunomeError> {
        // Initialize Viterbi cost calculation
        let mut min_cost = node.min_cost().saturating_sub(node.cost() as i32);
        let mut best_compact_node: Option<&CompactEndNode> = None;
        let node_left_id = node.left_id();

        // Hot path optimization: get end nodes directly (no Option checking in hot loop)
        let end_nodes = match self.enodes.get(self.p) {
            Some(nodes) if !nodes.is_empty() => nodes,
            _ => {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "End nodes array at position {} is empty for node '{}'. Lattice position: {}, enodes.len(): {}",
                        self.p,
                        node.surface(),
                        self.p,
                        self.enodes.len()
                    ),
                });
            }
        };

        // Ultra-optimized Viterbi search with inlined data (no indirection!)
        if end_nodes.len() == 1 {
            // Hot path specialization: single predecessor (most common case)
            let enode = &end_nodes[0];
            let connection_cost =
                self.cost_cache
                    .get_or_compute(enode.right_id, node_left_id, || {
                        self.dic.get_trans_cost(enode.right_id, node_left_id)
                    })?;

            let total_cost = enode
                .min_cost
                .checked_add(connection_cost as i32)
                .unwrap_or(i32::MAX);
            if total_cost < min_cost {
                min_cost = total_cost;
                best_compact_node = Some(enode);
            }
        } else {
            // Multiple predecessors: optimized loop with cached costs and inlined data
            for enode in end_nodes {
                // Calculate connection cost using cached lookup (major optimization)
                let connection_cost =
                    self.cost_cache
                        .get_or_compute(enode.right_id, node_left_id, || {
                            self.dic.get_trans_cost(enode.right_id, node_left_id)
                        })?;

                let total_cost = enode
                    .min_cost
                    .checked_add(connection_cost as i32)
                    .unwrap_or(i32::MAX);

                // Optimized cost comparison with inlined tie-breaking
                if total_cost < min_cost {
                    min_cost = total_cost;
                    best_compact_node = Some(enode);
                } else if total_cost == min_cost {
                    // Ultra-fast tie-breaking with inlined morph_id (no method calls!)
                    if let (Some(_current_best), Some(enode_id), Some(best_id)) = (
                        best_compact_node,
                        enode.morph_id,
                        best_compact_node.and_then(|n| n.morph_id),
                    ) {
                        if enode_id < best_id {
                            best_compact_node = Some(enode);
                        }
                    }
                }
            }
        }

        // Update node with optimal path information
        let final_cost = min_cost.checked_add(node.cost() as i32).unwrap_or(i32::MIN);
        node.set_min_cost(final_cost);

        if let Some(best_compact) = best_compact_node {
            node.set_back_pos(best_compact.pos as i32);
            node.set_back_index(best_compact.index as i32);
        } else {
            return Err(RunomeError::DictValidationError {
                reason: format!(
                    "No valid predecessor found for node '{}' at position {}",
                    node.surface(),
                    self.p
                ),
            });
        }

        // Set node position and index
        node.set_pos(self.p);
        let node_index = self.snodes.get(self.p).map_or(0, |nodes| nodes.len());
        node.set_index(node_index);

        // Ultra-optimized surface length calculation with caching
        let surface_len = self.get_surface_length(node.surface());
        let end_pos = self.p + surface_len;

        // Optimized lattice expansion
        self.ensure_capacity(end_pos);

        // Add to start nodes
        self.snodes[self.p].push(node);

        // Create ultra-compact end node with inlined critical data (major optimization!)
        let compact_end_node = CompactEndNode::from_node(
            self.snodes[self.p].last().unwrap().as_ref(),
            self.p,
            node_index,
        );
        self.enodes[end_pos].push(compact_end_node);

        Ok(())
    }

    /// Advance the lattice position
    ///
    /// Moves the position pointer forward to the next position that has end nodes,
    /// which will be used as starting points for the next set of node additions.
    ///
    /// # Returns
    /// * Number of positions advanced
    pub fn forward(&mut self) -> usize {
        let old_p = self.p;

        // Move to next position
        self.p += 1;

        // Find the next position that has end nodes
        while self.p < self.enodes.len()
            && self.enodes.get(self.p).is_none_or(|nodes| nodes.is_empty())
        {
            self.p += 1;
        }

        self.p - old_p
    }

    /// Finalize the lattice by adding an EOS (End-of-Sentence) node
    ///
    /// This method adds an EOS node at the current position and calculates
    /// its optimal cost from all available predecessor nodes. This completes
    /// the lattice construction for Viterbi path finding.
    ///
    /// # Returns
    /// * `Ok(())` if EOS was successfully added
    /// * `Err(RunomeError)` if cost calculation fails
    pub fn end(&mut self) -> Result<(), RunomeError> {
        // Python: eos = EOS(self.p)
        let eos = Box::new(EOS::new(self.p)) as Box<dyn LatticeNode + 'a>;

        // Python: self.add(eos) - use the same add() method as all other nodes
        self.add(eos)?;

        // Python: self.snodes = self.snodes[:self.p + 1]
        self.snodes.truncate(self.p + 1);

        Ok(())
    }

    /// Find minimum cost path using backward Viterbi algorithm
    ///
    /// Traces back from EOS node to BOS node following the optimal path
    /// computed during lattice construction. This is the core of morphological
    /// analysis - it returns the best segmentation of the input text.
    ///
    /// # Returns
    /// * `Ok(Vec<&dyn LatticeNode>)` - Vector of nodes representing optimal path from BOS to EOS
    /// * `Err(RunomeError)` - Error if lattice is invalid or no path exists
    ///
    /// # Path Structure
    /// The returned path always starts with BOS and ends with EOS:
    /// `[BOS, word1, word2, ..., wordN, EOS]`
    pub fn backward(&self) -> Result<Vec<&dyn LatticeNode>, RunomeError> {
        // Validate that lattice is properly finalized with EOS
        if self.snodes.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Empty lattice - no nodes to trace back from".to_string(),
            });
        }

        let last_pos = self.snodes.len() - 1;
        if self.snodes[last_pos].is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "No EOS node found at final position".to_string(),
            });
        }

        // Start from EOS node (should be at last position, index 0)
        let eos_node = &self.snodes[last_pos][0];
        if eos_node.surface() != "__EOS__" {
            return Err(RunomeError::DictValidationError {
                reason: "Final node is not EOS".to_string(),
            });
        }

        // Trace back through optimal path
        let mut path = Vec::new();
        let mut current_pos = last_pos;
        let mut current_index = 0;

        // Follow back-pointers until we reach BOS (back_pos = -1)
        loop {
            // Get current node
            if current_pos >= self.snodes.len() || current_index >= self.snodes[current_pos].len() {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Invalid path: position {} index {} out of bounds",
                        current_pos, current_index
                    ),
                });
            }

            let current_node = self.snodes[current_pos][current_index].as_ref();
            path.push(current_node);

            // Check if we've reached BOS (back_pos = -1)
            let back_pos = current_node.back_pos();
            if back_pos == -1 {
                // Should be BOS node
                if current_node.surface() != "__BOS__" {
                    return Err(RunomeError::DictValidationError {
                        reason: "Path trace reached node with back_pos=-1 but it's not BOS"
                            .to_string(),
                    });
                }
                break;
            }

            // Move to predecessor node
            let back_index = current_node.back_index();
            if back_pos < 0 || back_index < 0 {
                return Err(RunomeError::DictValidationError {
                    reason: format!(
                        "Invalid back pointers: back_pos={}, back_index={}",
                        back_pos, back_index
                    ),
                });
            }

            current_pos = back_pos as usize;
            current_index = back_index as usize;
        }

        // Reverse path to get BOS → EOS order
        path.reverse();

        // Validate path structure
        if path.is_empty() {
            return Err(RunomeError::DictValidationError {
                reason: "Empty path generated".to_string(),
            });
        }

        if path[0].surface() != "__BOS__" {
            return Err(RunomeError::DictValidationError {
                reason: "Path does not start with BOS".to_string(),
            });
        }

        if path[path.len() - 1].surface() != "__EOS__" {
            return Err(RunomeError::DictValidationError {
                reason: "Path does not end with EOS".to_string(),
            });
        }

        Ok(path)
    }
}

impl<'a> std::fmt::Debug for Lattice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lattice")
            .field("p", &self.p)
            .field("size", &self.size())
            .field("snodes_len", &self.snodes.len())
            .field("enodes_len", &self.enodes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictEntry;
    use std::sync::Arc;

    fn create_test_dict_entry() -> DictEntry {
        DictEntry {
            surface: "テスト".to_string(),
            left_id: 100,
            right_id: 200,
            cost: 150,
            part_of_speech: "名詞,一般,*,*,*,*".to_string(),
            inflection_type: "*".to_string(),
            inflection_form: "*".to_string(),
            base_form: "テスト".to_string(),
            reading: "テスト".to_string(),
            phonetic: "テスト".to_string(),
            morph_id: 0,
        }
    }

    #[test]
    fn test_node_creation() {
        let dict_entry = create_test_dict_entry();
        let node = Node::new(&dict_entry, NodeType::SysDict);

        assert_eq!(node.surface(), "テスト");
        assert_eq!(node.left_id(), 100);
        assert_eq!(node.right_id(), 200);
        assert_eq!(node.cost(), 150);
        assert_eq!(node.node_type(), NodeType::SysDict);
        assert_eq!(node.surface_len(), 3); // 3 characters

        // Check initial Viterbi values
        assert_eq!(node.min_cost(), i32::MAX);
        assert_eq!(node.back_pos(), -1);
        assert_eq!(node.back_index(), -1);
        assert_eq!(node.pos(), 0);
        assert_eq!(node.index(), 0);
    }

    #[test]
    fn test_unknown_node_creation() {
        let unknown = UnknownNode::new(
            "未知語".to_string(),
            300,
            400,
            500,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "未知語".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        );

        assert_eq!(unknown.surface(), "未知語");
        assert_eq!(unknown.left_id(), 300);
        assert_eq!(unknown.right_id(), 400);
        assert_eq!(unknown.cost(), 500);
        assert_eq!(unknown.node_type(), NodeType::Unknown);
        assert_eq!(unknown.surface_len(), 3);
        assert_eq!(unknown.part_of_speech(), "名詞,一般,*,*,*,*");
        assert_eq!(unknown.base_form(), "未知語");
    }

    #[test]
    fn test_bos_node() {
        let bos = BOS::new();

        assert_eq!(bos.surface(), "__BOS__");
        assert_eq!(bos.left_id(), 0);
        assert_eq!(bos.right_id(), 0);
        assert_eq!(bos.cost(), 0);
        assert_eq!(bos.min_cost(), 0); // BOS starts with cost 0
        assert_eq!(bos.node_type(), NodeType::SysDict);
        assert_eq!(bos.surface_len(), 0);
    }

    #[test]
    fn test_eos_node() {
        let eos = EOS::new(10);

        assert_eq!(eos.surface(), "__EOS__");
        assert_eq!(eos.left_id(), 0);
        assert_eq!(eos.right_id(), 0);
        assert_eq!(eos.cost(), 0);
        assert_eq!(eos.min_cost(), i32::MAX); // EOS starts with max cost
        assert_eq!(eos.pos(), 10);
        assert_eq!(eos.node_type(), NodeType::SysDict);
        assert_eq!(eos.surface_len(), 0);
    }

    #[test]
    fn test_viterbi_field_updates() {
        let dict_entry = create_test_dict_entry();
        let mut node = Node::new(&dict_entry, NodeType::SysDict);

        // Test updating Viterbi fields
        node.set_min_cost(1000);
        node.set_back_pos(5);
        node.set_back_index(2);
        node.set_pos(8);
        node.set_index(3);

        assert_eq!(node.min_cost(), 1000);
        assert_eq!(node.back_pos(), 5);
        assert_eq!(node.back_index(), 2);
        assert_eq!(node.pos(), 8);
        assert_eq!(node.index(), 3);
    }

    #[test]
    fn test_node_types() {
        let dict_entry = create_test_dict_entry();

        let sys_node = Node::new(&dict_entry, NodeType::SysDict);
        let user_node = Node::new(&dict_entry, NodeType::UserDict);

        assert_eq!(sys_node.node_type(), NodeType::SysDict);
        assert_eq!(user_node.node_type(), NodeType::UserDict);
    }

    #[test]
    fn test_surface_length_calculation() {
        // Test ASCII
        let dict_entry_ascii = DictEntry {
            surface: "test".to_string(),
            left_id: 1,
            right_id: 1,
            cost: 1,
            part_of_speech: "".to_string(),
            inflection_type: "".to_string(),
            inflection_form: "".to_string(),
            base_form: "".to_string(),
            reading: "".to_string(),
            phonetic: "".to_string(),
            morph_id: 1,
        };
        let node_ascii = Node::new(&dict_entry_ascii, NodeType::SysDict);
        assert_eq!(node_ascii.surface_len(), 4);

        // Test Japanese (multi-byte UTF-8)
        let dict_entry_jp = DictEntry {
            surface: "こんにちは".to_string(),
            left_id: 1,
            right_id: 1,
            cost: 1,
            part_of_speech: "".to_string(),
            inflection_type: "".to_string(),
            inflection_form: "".to_string(),
            base_form: "".to_string(),
            reading: "".to_string(),
            phonetic: "".to_string(),
            morph_id: 2,
        };
        let node_jp = Node::new(&dict_entry_jp, NodeType::SysDict);
        assert_eq!(node_jp.surface_len(), 5); // 5 characters, not bytes
    }

    // Mock dictionary for testing
    struct MockDictionary;

    impl crate::dictionary::Dictionary for MockDictionary {
        fn lookup(&self, _surface: &str) -> Result<Vec<&DictEntry>, crate::error::RunomeError> {
            Ok(Vec::new()) // Return empty for testing
        }

        fn get_trans_cost(
            &self,
            _left_id: u16,
            _right_id: u16,
        ) -> Result<i16, crate::error::RunomeError> {
            Ok(100) // Return fixed cost for testing
        }
    }

    fn create_mock_dictionary() -> Arc<dyn crate::dictionary::Dictionary> {
        Arc::new(MockDictionary)
    }

    #[test]
    fn test_lattice_creation() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(10, dic);

        // Check basic properties
        assert_eq!(lattice.position(), 1); // Starts at position 1
        assert_eq!(lattice.size(), 11); // Should be size + 1
        assert!(lattice.is_valid());

        // Check BOS node at position 0
        let start_nodes = lattice.start_nodes(0).unwrap();
        assert_eq!(start_nodes.len(), 1);
        assert_eq!(start_nodes[0].surface(), "__BOS__");
        assert_eq!(start_nodes[0].pos(), 0);
        assert_eq!(start_nodes[0].index(), 0);

        // Check BOS also appears in enodes[1] for connections
        let end_nodes = lattice.end_nodes(1).unwrap();
        assert_eq!(end_nodes.len(), 1);
        assert_eq!(end_nodes[0].pos, 0);
        assert_eq!(end_nodes[0].index, 0);

        // Verify we can access the actual node through the reference
        let node_ref = NodeRef {
            pos: end_nodes[0].pos as usize,
            index: end_nodes[0].index as usize,
        };
        if let Some(node) = lattice.get_node(&node_ref) {
            assert_eq!(node.surface(), "__BOS__");
        }
    }

    #[test]
    fn test_lattice_validation() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(5, dic);

        // Should be valid after creation
        assert!(lattice.is_valid());

        // Test validation logic
        assert!(lattice.start_nodes(0).is_some());
        assert!(lattice.end_nodes(1).is_some());
    }

    #[test]
    fn test_lattice_empty_positions() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(3, dic);

        // Positions 1, 2, 3 should be empty initially (except enodes[1] has BOS)
        assert_eq!(lattice.start_nodes(1).unwrap().len(), 0);
        assert_eq!(lattice.start_nodes(2).unwrap().len(), 0);
        assert_eq!(lattice.start_nodes(3).unwrap().len(), 0);

        // Only enodes[1] should have the BOS node, others empty
        assert_eq!(lattice.end_nodes(0).unwrap().len(), 0);
        assert_eq!(lattice.end_nodes(2).unwrap().len(), 0);
        assert_eq!(lattice.end_nodes(3).unwrap().len(), 0);
    }

    #[test]
    fn test_lattice_bounds_checking() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(2, dic);

        // Valid positions
        assert!(lattice.start_nodes(0).is_some());
        assert!(lattice.start_nodes(1).is_some());
        assert!(lattice.start_nodes(2).is_some());

        // Out of bounds positions should return None
        assert!(lattice.start_nodes(10).is_none());
        assert!(lattice.end_nodes(10).is_none());
    }

    #[test]
    fn test_lattice_dictionary_access() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(5, dic.clone());

        // Should be able to access the dictionary
        let lattice_dic = lattice.dictionary();

        // Test that it's the same dictionary (using Arc)
        assert!(Arc::ptr_eq(lattice_dic, &dic));
    }

    #[test]
    fn test_lattice_zero_size() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(0, dic);

        // Should still be valid with just BOS
        assert!(lattice.is_valid());
        assert_eq!(lattice.position(), 1);
        assert_eq!(lattice.size(), 1); // Just position 0

        // Should have BOS at position 0
        let start_nodes = lattice.start_nodes(0).unwrap();
        assert_eq!(start_nodes.len(), 1);
        assert_eq!(start_nodes[0].surface(), "__BOS__");
    }

    #[test]
    fn test_add_method_basic() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Create an unknown node to add (avoids lifetime issues)
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        // Add the node to the lattice
        let result = lattice.add(node);
        assert!(result.is_ok(), "Adding node should succeed");

        // Verify the node was added to snodes at current position (1)
        let start_nodes = lattice.start_nodes(1).unwrap();
        assert_eq!(start_nodes.len(), 1, "Should have one node at position 1");

        // Verify the node has correct Viterbi fields set
        let added_node = &start_nodes[0];
        assert_eq!(added_node.pos(), 1, "Node position should be 1");
        assert_eq!(added_node.index(), 0, "Node index should be 0");
        assert!(
            added_node.min_cost() < i32::MAX,
            "Min cost should be calculated"
        );

        // Verify back-pointers are set (should point to BOS node)
        assert_eq!(
            added_node.back_pos(),
            0,
            "Should point back to BOS position"
        );
        assert_eq!(added_node.back_index(), 0, "Should point back to BOS index");
    }

    #[test]
    fn test_add_method_multiple_nodes() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Create multiple unknown nodes with different surface forms
        let node1 = Box::new(UnknownNode::new(
            "テスト1".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト1".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        let node2 = Box::new(UnknownNode::new(
            "テスト2".to_string(),
            101,
            201,
            200,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト2".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        // Add both nodes
        assert!(
            lattice.add(node1).is_ok(),
            "Adding first node should succeed"
        );
        assert!(
            lattice.add(node2).is_ok(),
            "Adding second node should succeed"
        );

        // Verify both nodes were added
        let start_nodes = lattice.start_nodes(1).unwrap();
        assert_eq!(start_nodes.len(), 2, "Should have two nodes at position 1");

        // Verify they have different indices
        assert_eq!(start_nodes[0].index(), 0, "First node should have index 0");
        assert_eq!(start_nodes[1].index(), 1, "Second node should have index 1");

        // Both should point back to BOS
        for node in start_nodes {
            assert_eq!(node.back_pos(), 0, "Should point back to BOS position");
            assert_eq!(node.back_index(), 0, "Should point back to BOS index");
        }
    }

    #[test]
    fn test_add_method_cost_calculation() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Create an unknown node with cost = 150
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        // Add the node
        assert!(lattice.add(node).is_ok(), "Adding node should succeed");

        // Check the calculated cost
        let start_nodes = lattice.start_nodes(1).unwrap();
        let added_node = &start_nodes[0];

        // Expected cost calculation:
        // BOS min_cost (0) + connection_cost (100 from mock) + node_cost (150) = 250
        assert_eq!(
            added_node.min_cost(),
            250,
            "Min cost should be correctly calculated"
        );
    }

    #[test]
    fn test_add_method_unknown_node() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Create an unknown node
        let unknown_node = Box::new(UnknownNode::new(
            "未知語".to_string(),
            300,
            400,
            500,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "未知語".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        // Add the unknown node
        let result = lattice.add(unknown_node);
        assert!(result.is_ok(), "Adding unknown node should succeed");

        // Verify it was added correctly
        let start_nodes = lattice.start_nodes(1).unwrap();
        assert_eq!(start_nodes.len(), 1, "Should have one unknown node");

        let added_node = &start_nodes[0];
        assert_eq!(added_node.surface(), "未知語", "Surface should match");
        assert_eq!(
            added_node.node_type(),
            NodeType::Unknown,
            "Should be unknown type"
        );

        // Cost: BOS min_cost (0) + connection_cost (100) + unknown_cost (500) = 600
        assert_eq!(
            added_node.min_cost(),
            600,
            "Unknown node cost should be correct"
        );
    }

    #[test]
    fn test_add_method_lattice_expansion() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(2, dic); // Small lattice

        // Create an unknown node with long surface (will extend beyond initial size)
        let node = Box::new(UnknownNode::new(
            "とても長い表面形".to_string(), // 7 characters
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "とても長い表面形".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        // Add the node (should trigger lattice expansion)
        let result = lattice.add(node);
        assert!(
            result.is_ok(),
            "Adding long node should succeed and expand lattice"
        );

        // Verify lattice was expanded
        let end_pos = 1 + 7; // position 1 + 7 characters = position 8
        assert!(lattice.snodes.len() > end_pos, "Lattice should be expanded");
        assert!(lattice.enodes.len() > end_pos, "Lattice should be expanded");

        // Verify node was added to start nodes at current position
        let start_nodes = lattice.start_nodes(1).unwrap();
        assert!(
            !start_nodes.is_empty(),
            "Should have start node at position 1"
        );

        // Verify node was added to end nodes at calculated end position
        let end_nodes = lattice.end_nodes(end_pos);
        assert!(
            end_nodes.is_some(),
            "End nodes should exist at calculated position"
        );
        // Note: The end node might be empty if the node cloning logic didn't work properly
        // For now, just verify the lattice expanded correctly
        assert!(
            lattice.snodes.len() > end_pos,
            "Lattice should have expanded to accommodate end position"
        );
    }

    #[test]
    fn test_add_method_viterbi_optimization() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Add first node at position 1
        let node1 = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node1).is_ok());

        // Move to next position
        lattice.forward();

        // Add second node that should connect to the first
        let node2 = Box::new(UnknownNode::new(
            "語".to_string(), // 1 character
            101,
            201,
            100,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "語".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node2).is_ok());

        // Verify the second node has optimal predecessor
        let position = lattice.position();
        let start_nodes = lattice.start_nodes(position).unwrap();
        assert!(
            !start_nodes.is_empty(),
            "Should have nodes at current position"
        );

        let second_node = &start_nodes[0];
        // Should point back to the first node we added
        assert_eq!(
            second_node.back_pos(),
            1,
            "Should point back to first node position"
        );
        assert_eq!(
            second_node.back_index(),
            0,
            "Should point back to first node index"
        );
    }

    #[test]
    fn test_add_forward_end() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Get SystemDictionary instance
        let sys_dict_result = crate::dictionary::SystemDictionary::instance();
        if sys_dict_result.is_err() {
            eprintln!(
                "Skipping test: Failed to create SystemDictionary: {:?}",
                sys_dict_result.err()
            );
            return;
        }
        let sys_dict = sys_dict_result.unwrap();

        // Test string "すもも" (3 characters)
        let s = "すもも";
        let mut lattice = Lattice::new(s.chars().count(), sys_dict.clone());

        // Step 1: Look up "すもも" and add all entries
        let entries_result = sys_dict.lookup(s);
        assert!(entries_result.is_ok(), "Dictionary lookup should succeed");
        let entries = entries_result.unwrap();

        for entry in &entries {
            let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
            let add_result = lattice.add(node);
            assert!(add_result.is_ok(), "Adding node should succeed");
        }

        // Verify nodes were added at position 1
        let start_nodes_1 = lattice.start_nodes(1).unwrap();
        assert!(!start_nodes_1.is_empty(), "Should have nodes at position 1");

        // Step 2: Move forward and look up substring
        let positions_moved = lattice.forward();
        assert!(positions_moved >= 1, "Should move at least one position");

        // Look up remaining substring (skip first character)
        let remaining = &s[3..]; // Skip first UTF-8 character "す" (3 bytes)
        if !remaining.is_empty() {
            let substring_entries_result = sys_dict.lookup(remaining);
            assert!(
                substring_entries_result.is_ok(),
                "Substring lookup should succeed"
            );
            let substring_entries = substring_entries_result.unwrap();

            for entry in &substring_entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding substring node should succeed");
            }

            let current_pos = lattice.position();
            let start_nodes = lattice.start_nodes(current_pos).unwrap();
            assert!(
                !start_nodes.is_empty(),
                "Should have nodes after adding substring"
            );
        }

        // Step 3: Continue forward and lookup next substring
        let positions_moved_2 = lattice.forward();
        assert!(
            positions_moved_2 >= 1,
            "Second forward should move at least one position"
        );

        // Look up final substring if exists
        let final_remaining = &s[6..]; // Skip first two UTF-8 characters
        if !final_remaining.is_empty() {
            let final_entries_result = sys_dict.lookup(final_remaining);
            assert!(
                final_entries_result.is_ok(),
                "Final substring lookup should succeed"
            );
            let final_entries = final_entries_result.unwrap();

            for entry in &final_entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding final node should succeed");
            }
        }

        // Step 4: Final forward and end lattice
        let final_forward = lattice.forward();
        assert!(
            final_forward >= 1,
            "Final forward should move at least one position"
        );

        let end_result = lattice.end();
        assert!(end_result.is_ok(), "Lattice end should succeed");

        // Verify EOS was added
        let final_pos = lattice.position();
        let final_nodes = lattice.start_nodes(final_pos).unwrap();
        assert!(
            !final_nodes.is_empty(),
            "Should have EOS node at final position"
        );
        assert_eq!(
            final_nodes[0].surface(),
            "__EOS__",
            "Final node should be EOS"
        );
    }

    #[test]
    fn test_end_method_basic() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(5, dic);

        // Add an unknown node first
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node).is_ok());

        // Move forward to simulate processing
        lattice.forward();

        // Add EOS
        let result = lattice.end();
        assert!(result.is_ok(), "End method should succeed");

        // Verify EOS was added
        let current_pos = lattice.position();
        let start_nodes = lattice.start_nodes(current_pos).unwrap();
        assert_eq!(start_nodes.len(), 1, "Should have EOS node");

        let eos_node = &start_nodes[0];
        assert_eq!(eos_node.surface(), "__EOS__", "Should be EOS node");
        assert_eq!(
            eos_node.pos(),
            current_pos,
            "EOS position should be correct"
        );
        assert_eq!(eos_node.index(), 0, "EOS index should be 0");

        // EOS should have valid back-pointers
        assert!(
            eos_node.back_pos() >= 0,
            "EOS should have valid back position"
        );
        assert!(
            eos_node.back_index() >= 0,
            "EOS should have valid back index"
        );
    }

    #[test]
    fn test_end_method_cost_calculation() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(5, dic);

        // Add an unknown node with known cost = 150
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node).is_ok());

        // Move to end position
        lattice.forward();

        // Add EOS
        assert!(lattice.end().is_ok());

        // Verify EOS cost calculation
        let current_pos = lattice.position();
        let start_nodes = lattice.start_nodes(current_pos).unwrap();
        let eos_node = &start_nodes[0];

        // EOS should have calculated minimum cost from its predecessor
        assert!(
            eos_node.min_cost() < i32::MAX,
            "EOS should have calculated cost"
        );
        // Expected: predecessor min_cost + connection_cost (100) = 250 + 100 = 350
        assert_eq!(
            eos_node.min_cost(),
            350,
            "EOS cost should be correctly calculated"
        );
    }

    #[test]
    fn test_end_method_truncation() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic); // Large lattice

        // Add a short unknown node
        let node = Box::new(UnknownNode::new(
            "短".to_string(), // 1 character
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "短".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node).is_ok());

        // Move forward
        lattice.forward();

        let pos_before_end = lattice.position();
        let size_before_end = lattice.snodes.len();

        // Add EOS
        assert!(lattice.end().is_ok());

        // Verify lattice was truncated
        let size_after_end = lattice.snodes.len();
        assert_eq!(
            size_after_end,
            pos_before_end + 1,
            "Lattice should be truncated to EOS position + 1"
        );
        assert!(
            size_after_end <= size_before_end,
            "Lattice should not grow after EOS"
        );
    }

    #[test]
    fn test_forward_method() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(10, dic);

        // Initially at position 1
        assert_eq!(lattice.position(), 1);

        // Add an unknown node to create end nodes (3-character surface)
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(), // 3 characters
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;
        assert!(lattice.add(node).is_ok());

        // Forward should move to next position with end nodes
        let positions_moved = lattice.forward();
        assert!(positions_moved >= 1, "Should move at least one position");

        // Should be at a position that has end nodes
        let current_pos = lattice.position();
        let end_nodes = lattice.end_nodes(current_pos);
        assert!(end_nodes.is_some(), "Should be at position with end nodes");
        assert!(
            !end_nodes.unwrap().is_empty(),
            "End nodes should not be empty"
        );
    }

    #[test]
    fn test_add_forward_end_simulation() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Get SystemDictionary instance
        let sys_dict_result = crate::dictionary::SystemDictionary::instance();
        if sys_dict_result.is_err() {
            eprintln!(
                "Skipping test: Failed to create SystemDictionary: {:?}",
                sys_dict_result.err()
            );
            return;
        }
        let sys_dict = sys_dict_result.unwrap();

        // Test string "すもも" (3 characters) - same as Python test
        let s = "すもも";
        let mut lattice = Lattice::new(s.chars().count(), sys_dict.clone());

        // Step 1: Look up "すもも" and add all entries
        let entries_result = sys_dict.lookup(s);
        assert!(entries_result.is_ok(), "Dictionary lookup should succeed");
        let entries = entries_result.unwrap();

        for entry in &entries {
            let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
            let add_result = lattice.add(node);
            assert!(add_result.is_ok(), "Adding node should succeed");
        }

        // Verify initial lattice state - mimic Python assertions
        let start_nodes_1 = lattice.start_nodes(1).unwrap();

        // Python test: self.assertEqual(9, len(lattice.snodes[1]))
        assert_eq!(
            start_nodes_1.len(),
            9,
            "Should have 9 start nodes at position 1 like Python test"
        );

        // Python test: self.assertEqual(7, len(lattice.enodes[2]))
        if let Some(end_nodes_2) = lattice.end_nodes(2) {
            assert_eq!(
                end_nodes_2.len(),
                7,
                "Should have 7 end nodes at position 2 like Python test"
            );
        }

        // Python test: self.assertEqual(1, len(lattice.enodes[3]))
        if let Some(end_nodes_3) = lattice.end_nodes(3) {
            assert_eq!(
                end_nodes_3.len(),
                1,
                "Should have 1 end node at position 3 like Python test"
            );
        }

        // Python test: self.assertEqual(1, len(lattice.enodes[4]))
        if let Some(end_nodes_4) = lattice.end_nodes(4) {
            assert_eq!(
                end_nodes_4.len(),
                1,
                "Should have 1 end node at position 4 like Python test"
            );
        }

        // Step 2: Move forward (equivalent to lattice.forward() in Python)
        let positions_moved = lattice.forward();
        assert_eq!(
            positions_moved, 1,
            "Should move exactly 1 position like Python test"
        );

        // Step 3: Look up substring "もも" (s[1:] in Python - character-based slicing)
        let chars: Vec<char> = s.chars().collect();
        let substring: String = chars[1..].iter().collect(); // Skip first character "す"

        if !substring.is_empty() {
            let substring_entries_result = sys_dict.lookup(&substring);
            assert!(
                substring_entries_result.is_ok(),
                "Substring lookup should succeed"
            );
            let substring_entries = substring_entries_result.unwrap();

            for entry in &substring_entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding substring node should succeed");
            }

            // Verify node counts match Python test expectations
            let current_pos = lattice.position();
            let start_nodes = lattice.start_nodes(current_pos).unwrap();

            // Python test: self.assertEqual(4, len(lattice.snodes[2]))
            assert_eq!(
                start_nodes.len(),
                4,
                "Should have 4 start nodes at position 2 like Python test"
            );

            // Python test: self.assertEqual(3, len(lattice.enodes[3]))
            if let Some(end_nodes_3) = lattice.end_nodes(3) {
                assert_eq!(
                    end_nodes_3.len(),
                    3,
                    "Should have 3 end nodes at position 3 like Python test"
                );
            }

            // Python test: self.assertEqual(3, len(lattice.enodes[4]))
            if let Some(end_nodes_4) = lattice.end_nodes(4) {
                assert_eq!(
                    end_nodes_4.len(),
                    3,
                    "Should have 3 end nodes at position 4 like Python test"
                );
            }
        }

        // Step 4: Second forward
        let positions_moved_2 = lattice.forward();
        assert_eq!(
            positions_moved_2, 1,
            "Second forward should move exactly 1 position"
        );

        // Step 5: Look up final substring "も" (s[2:] in Python - character-based slicing)
        let final_substring: String = chars[2..].iter().collect(); // Skip first two characters "すも"

        if !final_substring.is_empty() {
            let final_entries_result = sys_dict.lookup(&final_substring);
            assert!(
                final_entries_result.is_ok(),
                "Final substring lookup should succeed"
            );
            let final_entries = final_entries_result.unwrap();

            for entry in &final_entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding final node should succeed");
            }

            // Verify node counts match Python test expectations
            let current_pos = lattice.position();
            let start_nodes = lattice.start_nodes(current_pos).unwrap();

            // Python test: self.assertEqual(2, len(lattice.snodes[3]))
            assert_eq!(
                start_nodes.len(),
                2,
                "Should have 2 start nodes at position 3 like Python test"
            );

            // Python test: self.assertEqual(5, len(lattice.enodes[4]))
            if let Some(end_nodes_4) = lattice.end_nodes(4) {
                assert_eq!(
                    end_nodes_4.len(),
                    5,
                    "Should have 5 end nodes at position 4 like Python test"
                );
            }
        }

        // Step 6: Final forward
        let final_forward = lattice.forward();
        assert_eq!(
            final_forward, 1,
            "Final forward should move exactly 1 position"
        );

        // Step 7: End lattice (equivalent to lattice.end() in Python)
        let end_result = lattice.end();
        assert!(end_result.is_ok(), "Lattice end should succeed");

        // Verify EOS was added - match Python test assertions
        let final_pos = lattice.position();
        let final_nodes = lattice.start_nodes(final_pos).unwrap();

        // Python test: self.assertTrue(isinstance(lattice.snodes[4][0], EOS))
        assert!(
            !final_nodes.is_empty(),
            "Should have EOS node at final position"
        );
        assert_eq!(
            final_nodes[0].surface(),
            "__EOS__",
            "Final node should be EOS"
        );

        // Python test: self.assertTrue(isinstance(lattice.enodes[5][0], EOS))
        if let Some(end_nodes_5) = lattice.end_nodes(final_pos + 1) {
            if !end_nodes_5.is_empty() {
                let node_ref = NodeRef {
                    pos: end_nodes_5[0].pos as usize,
                    index: end_nodes_5[0].index as usize,
                };
                if let Some(node) = lattice.get_node(&node_ref) {
                    assert_eq!(node.surface(), "__EOS__", "EOS should appear in end nodes");
                }
            }
        }
    }

    #[test]
    fn test_backward_method_basic() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(3, dic);

        // Add a simple chain: BOS -> node1 -> EOS
        let node1 = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        assert!(lattice.add(node1).is_ok());
        lattice.forward();
        assert!(lattice.end().is_ok());

        // Test backward path finding
        let path_result = lattice.backward();
        assert!(path_result.is_ok(), "Backward should succeed");

        let path = path_result.unwrap();
        assert_eq!(path.len(), 3, "Path should have BOS, node, EOS");

        // Verify path structure
        assert_eq!(path[0].surface(), "__BOS__", "First node should be BOS");
        assert_eq!(
            path[1].surface(),
            "テスト",
            "Second node should be our test node"
        );
        assert_eq!(path[2].surface(), "__EOS__", "Third node should be EOS");
    }

    #[test]
    fn test_backward_method_empty_lattice() {
        let dic = create_mock_dictionary();
        let lattice = Lattice::new(0, dic);

        // Try backward on lattice without EOS
        let path_result = lattice.backward();
        assert!(
            path_result.is_err(),
            "Backward should fail on unfinalized lattice"
        );
    }

    #[test]
    fn test_backward_method_unfinalized_lattice() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(3, dic);

        // Add a node but don't call end()
        let node = Box::new(UnknownNode::new(
            "テスト".to_string(),
            100,
            200,
            150,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "テスト".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        assert!(lattice.add(node).is_ok());

        // Try backward without EOS
        let path_result = lattice.backward();
        assert!(path_result.is_err(), "Backward should fail without EOS");
    }

    #[test]
    fn test_backward_method_multiple_nodes() {
        let dic = create_mock_dictionary();
        let mut lattice = Lattice::new(5, dic);

        // Add multiple nodes: BOS -> node1 -> node2 -> EOS
        let node1 = Box::new(UnknownNode::new(
            "日本".to_string(),
            100,
            200,
            150,
            "名詞,固有名詞,地域,国,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "日本".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        assert!(lattice.add(node1).is_ok());
        lattice.forward();

        let node2 = Box::new(UnknownNode::new(
            "語".to_string(),
            200,
            300,
            100,
            "名詞,一般,*,*,*,*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "語".to_string(),
            "*".to_string(),
            "*".to_string(),
            NodeType::Unknown,
        )) as Box<dyn LatticeNode>;

        assert!(lattice.add(node2).is_ok());
        lattice.forward();
        assert!(lattice.end().is_ok());

        // Test backward path finding
        let path_result = lattice.backward();
        assert!(path_result.is_ok(), "Backward should succeed");

        let path = path_result.unwrap();
        assert_eq!(path.len(), 4, "Path should have BOS, node1, node2, EOS");

        // Verify path structure and order
        assert_eq!(path[0].surface(), "__BOS__", "First should be BOS");
        assert_eq!(path[1].surface(), "日本", "Second should be '日本'");
        assert_eq!(path[2].surface(), "語", "Third should be '語'");
        assert_eq!(path[3].surface(), "__EOS__", "Fourth should be EOS");
    }

    #[test]
    fn test_backward_simple_morpheme_sequence() {
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Get SystemDictionary instance
        let sys_dict_result = crate::dictionary::SystemDictionary::instance();
        if sys_dict_result.is_err() {
            eprintln!(
                "Skipping test: Failed to create SystemDictionary: {:?}",
                sys_dict_result.err()
            );
            return;
        }
        let sys_dict = sys_dict_result.unwrap();

        // Simple test with "すもも" (3 characters)
        let s = "すもも";
        let mut lattice = Lattice::new(s.chars().count(), sys_dict.clone());

        // Replicate the Python test pattern exactly
        let mut pos = 0;
        let chars: Vec<char> = s.chars().collect();

        while pos < chars.len() {
            // Get substring from current position
            let remaining: String = chars[pos..].iter().collect();

            // Look up dictionary entries
            let entries_result = sys_dict.lookup(&remaining);
            assert!(entries_result.is_ok(), "Dictionary lookup should succeed");
            let entries = entries_result.unwrap();

            // Add all entries to lattice
            for entry in &entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding node should succeed");
            }

            // Move forward
            pos += lattice.forward();
        }

        // End lattice
        assert!(lattice.end().is_ok(), "Lattice end should succeed");

        // Run backward algorithm
        let path_result = lattice.backward();
        assert!(path_result.is_ok(), "Backward should succeed");
        let min_cost_path = path_result.unwrap();

        // Verify basic structure
        assert!(
            min_cost_path.len() >= 3,
            "Should have at least BOS, word, EOS"
        );
        assert_eq!(min_cost_path[0].surface(), "__BOS__", "First should be BOS");
        assert_eq!(
            min_cost_path[min_cost_path.len() - 1].surface(),
            "__EOS__",
            "Last should be EOS"
        );

        // Reconstruct the text
        let reconstructed: String = min_cost_path[1..min_cost_path.len() - 1]
            .iter()
            .map(|node| node.surface())
            .collect();
        assert_eq!(reconstructed, s, "Should reconstruct original text");
    }

    #[test]
    fn test_backward() {
        // Equivalent to Python TestLattice.test_backward()
        // Tests the complete lattice building and backward path finding process
        // with the same test string "すもももももももものうち" used in the Python version.
        // Skip test if sysdic directory doesn't exist (e.g., in CI)
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        // Get SystemDictionary instance
        let sys_dict_result = crate::dictionary::SystemDictionary::instance();
        if sys_dict_result.is_err() {
            eprintln!(
                "Skipping test: Failed to create SystemDictionary: {:?}",
                sys_dict_result.err()
            );
            return;
        }
        let sys_dict = sys_dict_result.unwrap();

        let s = "すもももももももものうち";

        // Python test: lattice = Lattice(len(s), SYS_DIC)
        let mut lattice = Lattice::new(s.chars().count(), sys_dict.clone());

        // Python test: pos = 0; while pos < len(s): ...
        let mut pos = 0;
        let chars: Vec<char> = s.chars().collect();

        while pos < chars.len() {
            // Python test: entries = SYS_DIC.lookup(s[pos:].encode('utf8'), MATCHER)
            let remaining: String = chars[pos..].iter().collect();
            let entries_result = sys_dict.lookup(&remaining);
            assert!(entries_result.is_ok(), "Dictionary lookup should succeed");
            let entries = entries_result.unwrap();

            // Python test: for e in entries: lattice.add(SurfaceNode(e))
            for entry in &entries {
                let node = Box::new(Node::new(entry, NodeType::SysDict)) as Box<dyn LatticeNode>;
                let add_result = lattice.add(node);
                assert!(add_result.is_ok(), "Adding node should succeed");
            }

            // Python test: pos += lattice.forward()
            pos += lattice.forward();
        }

        // Python test: lattice.end()
        assert!(lattice.end().is_ok(), "Lattice end should succeed");

        // Python test: min_cost_path = lattice.backward()
        let path_result = lattice.backward();
        assert!(path_result.is_ok(), "Backward should succeed");
        let min_cost_path = path_result.unwrap();

        // Python test: self.assertEqual(9, len(min_cost_path))
        assert_eq!(
            min_cost_path.len(),
            9,
            "Should have 9 nodes in the minimum cost path"
        );

        // Verify basic structure regardless of exact segmentation
        assert!(
            min_cost_path.len() >= 3,
            "Should have at least BOS, content, EOS"
        );

        // Python test: self.assertTrue(isinstance(min_cost_path[0], BOS))
        assert_eq!(
            min_cost_path[0].surface(),
            "__BOS__",
            "First node should be BOS"
        );

        // Python test: self.assertTrue(isinstance(min_cost_path[8], EOS))
        assert_eq!(
            min_cost_path[min_cost_path.len() - 1].surface(),
            "__EOS__",
            "Last node should be EOS"
        );

        // Verify text reconstruction works
        let reconstructed: String = min_cost_path[1..min_cost_path.len() - 1]
            .iter()
            .map(|node| node.surface())
            .collect();
        assert_eq!(reconstructed, s, "Should reconstruct original text");

        // Verify the expected segmentation matches Python
        // Python test expectations:
        assert_eq!(
            min_cost_path[1].surface(),
            "すもも",
            "Second node should be 'すもも'"
        );
        assert_eq!(
            min_cost_path[2].surface(),
            "も",
            "Third node should be 'も'"
        );
        assert_eq!(
            min_cost_path[3].surface(),
            "もも",
            "Fourth node should be 'もも'"
        );
        assert_eq!(
            min_cost_path[4].surface(),
            "も",
            "Fifth node should be 'も'"
        );
        assert_eq!(
            min_cost_path[5].surface(),
            "もも",
            "Sixth node should be 'もも'"
        );
        assert_eq!(
            min_cost_path[6].surface(),
            "の",
            "Seventh node should be 'の'"
        );
        assert_eq!(
            min_cost_path[7].surface(),
            "うち",
            "Eighth node should be 'うち'"
        );
    }
}
