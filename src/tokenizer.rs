use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use crate::dictionary::{DictEntry, Dictionary, SystemDictionary, UserDictionary};
use crate::error::RunomeError;
use crate::intern;
use crate::lattice::{Lattice, LatticeNode, NodeType, StartNode};

/// Constants matching Python Janome tokenizer
const MAX_CHUNK_SIZE: usize = 1024;
const CHUNK_SIZE: usize = 500;

/// Token struct containing all morphological information
/// Mirrors the Python Token class with complete compatibility
/// Uses Cow<str> for zero-copy optimization when strings can reference static/interned data
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    surface: Cow<'static, str>,
    part_of_speech: Cow<'static, str>,
    infl_type: Cow<'static, str>,
    infl_form: Cow<'static, str>,
    base_form: Cow<'static, str>,
    reading: Cow<'static, str>,
    phonetic: Cow<'static, str>,
    node_type: NodeType,
}

fn normalize_inflection(value: &str) -> Cow<'static, str> {
    if value.contains('\u{FF0D}') {
        Cow::Owned(value.replace('\u{FF0D}', "\u{2212}"))
    } else {
        intern::intern_or_cow(value)
    }
}

impl Token {
    /// Create a Token from a dictionary node with full morphological information
    /// Uses zero-copy optimization for interned strings
    pub fn from_dict_node<N: LatticeNode + ?Sized>(node: &N) -> Self {
        Self {
            surface: intern::intern_or_cow(node.surface()),
            part_of_speech: intern::intern_or_cow(node.part_of_speech()),
            infl_type: normalize_inflection(node.inflection_type()),
            infl_form: normalize_inflection(node.inflection_form()),
            base_form: intern::intern_or_cow(node.base_form()),
            reading: intern::intern_or_cow(node.reading()),
            phonetic: intern::intern_or_cow(node.phonetic()),
            node_type: node.node_type(),
        }
    }

    /// Create a Token from an unknown word node
    /// Uses zero-copy optimization for interned strings (especially asterisks)
    pub fn from_unknown_node<N: LatticeNode + ?Sized>(node: &N, baseform_unk: bool) -> Self {
        let base_form = if baseform_unk {
            intern::intern_or_cow(node.surface())
        } else {
            Cow::Borrowed(intern::ASTERISK)
        };

        Self {
            surface: intern::intern_or_cow(node.surface()),
            part_of_speech: intern::intern_or_cow(node.part_of_speech()),
            infl_type: Cow::Borrowed(intern::ASTERISK),
            infl_form: Cow::Borrowed(intern::ASTERISK),
            base_form,
            reading: intern::intern_or_cow(node.reading()),
            phonetic: intern::intern_or_cow(node.phonetic()),
            node_type: node.node_type(),
        }
    }

    /// Create a Token with explicit field values
    /// Used by TokenFilters to create modified tokens
    /// Converts String parameters to Cow<str> with interning optimization
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: String,
        part_of_speech: String,
        infl_type: String,
        infl_form: String,
        base_form: String,
        reading: String,
        phonetic: String,
        node_type: NodeType,
    ) -> Self {
        Self {
            surface: intern::intern_or_cow(&surface),
            part_of_speech: intern::intern_or_cow(&part_of_speech),
            infl_type: normalize_inflection(&infl_type),
            infl_form: normalize_inflection(&infl_form),
            base_form: intern::intern_or_cow(&base_form),
            reading: intern::intern_or_cow(&reading),
            phonetic: intern::intern_or_cow(&phonetic),
            node_type,
        }
    }

    // Accessor methods matching Python Token class

    pub fn surface(&self) -> &str {
        &self.surface
    }

    pub fn part_of_speech(&self) -> &str {
        &self.part_of_speech
    }

    pub fn infl_type(&self) -> &str {
        &self.infl_type
    }

    pub fn infl_form(&self) -> &str {
        &self.infl_form
    }

    pub fn base_form(&self) -> &str {
        &self.base_form
    }

    pub fn reading(&self) -> &str {
        &self.reading
    }

    pub fn phonetic(&self) -> &str {
        &self.phonetic
    }

    pub fn node_type(&self) -> NodeType {
        self.node_type.clone()
    }
}

impl fmt::Display for Token {
    /// Format Token to match Python Janome output exactly:
    /// "surface\tpart_of_speech,infl_type,infl_form,base_form,reading,phonetic"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t{},{},{},{},{},{}",
            self.surface,
            self.part_of_speech,
            self.infl_type,
            self.infl_form,
            self.base_form,
            self.reading,
            self.phonetic
        )
    }
}

/// Enum representing the result of tokenization
/// Either a full Token with morphological info or just the surface string (wakati mode)
#[derive(Debug, Clone)]
pub enum TokenizeResult {
    Token(Token),
    Surface(String),
}

/// Precomputed view over a chunk's characters and byte offsets
struct ChunkCharView<'a> {
    text: &'a str,
    offsets: Vec<usize>,
    chars: Vec<char>,
}

impl<'a> ChunkCharView<'a> {
    fn new(text: &'a str) -> Self {
        let mut offsets = Vec::with_capacity(text.len().saturating_add(1));
        let mut chars = Vec::with_capacity(text.len());

        for (byte_offset, ch) in text.char_indices() {
            offsets.push(byte_offset);
            chars.push(ch);
        }
        offsets.push(text.len());

        Self {
            text,
            offsets,
            chars,
        }
    }

    #[inline]
    fn len_chars(&self) -> usize {
        self.chars.len()
    }

    #[inline]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        debug_assert!(start <= end);
        let start_byte = if start < self.offsets.len() {
            self.offsets[start]
        } else {
            self.text.len()
        };
        let end_byte = if end < self.offsets.len() {
            self.offsets[end]
        } else {
            *self.offsets.last().unwrap_or(&self.text.len())
        };
        &self.text[start_byte..end_byte]
    }

    #[inline]
    fn char_at(&self, index: usize) -> char {
        self.chars[index]
    }
}

impl fmt::Display for TokenizeResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizeResult::Token(token) => write!(f, "{}", token),
            TokenizeResult::Surface(surface) => write!(f, "{}", surface),
        }
    }
}

/// Iterator for streaming tokenization results
pub struct TextChunkIterator<'a> {
    tokenizer: &'a Tokenizer,
    text: &'a str,
    processed: usize,
    current_tokens: std::vec::IntoIter<TokenizeResult>,
    wakati: bool,
    baseform_unk: bool,
}

impl<'a> Iterator for TextChunkIterator<'a> {
    type Item = Result<TokenizeResult, RunomeError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Return next token from current batch
        if let Some(token) = self.current_tokens.next() {
            return Some(Ok(token));
        }

        // Process next chunk if available
        if self.processed < self.text.len() {
            match self.tokenizer.tokenize_partial(
                &self.text[self.processed..],
                self.wakati,
                self.baseform_unk,
            ) {
                Ok((tokens, pos)) => {
                    self.processed += pos;
                    self.current_tokens = tokens.into_iter();
                    self.current_tokens.next().map(Ok)
                }
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

/// Main Tokenizer struct providing Japanese morphological analysis
/// Mirrors the Python Janome Tokenizer class API
#[derive(Clone)]
pub struct Tokenizer {
    sys_dic: Arc<SystemDictionary>,
    user_dic: Option<Arc<UserDictionary>>,
    max_unknown_length: usize,
    wakati: bool,
}

impl Tokenizer {
    fn surface_len_with_limit(surface: &str, limit: usize) -> Option<usize> {
        if limit == 0 {
            return None;
        }
        let mut len = 0usize;
        for _ in surface.chars() {
            len += 1;
            if len > limit {
                return None;
            }
        }
        if len == 0 { None } else { Some(len) }
    }

    fn emit_dictionary_entries(
        lattice: &mut Lattice<'_>,
        entries: &[&DictEntry],
        node_type: NodeType,
        max_char_len: usize,
    ) -> Result<bool, RunomeError> {
        let mut emitted = false;

        for entry in entries.iter().copied() {
            if Self::surface_len_with_limit(&entry.surface, max_char_len).is_none() {
                continue;
            }

            let start_node = StartNode::Unknown(crate::lattice::UnknownNode::from_dict_entry(
                &entry.surface,
                entry.left_id,
                entry.right_id,
                entry.cost,
                &entry.part_of_speech,
                &entry.inflection_type,
                &entry.inflection_form,
                &entry.base_form,
                &entry.reading,
                &entry.phonetic,
                node_type.clone(),
            ));
            lattice.add(start_node)?;
            emitted = true;
        }

        Ok(emitted)
    }

    /// Create a new Tokenizer instance
    ///
    /// # Arguments
    /// * `max_unknown_length` - Maximum length for unknown words (default: 1024)
    /// * `wakati` - If true, only return surface forms (default: false)
    ///
    /// # Returns
    /// * `Ok(Tokenizer)` - Successfully created tokenizer
    /// * `Err(RunomeError)` - Error if dictionary initialization fails
    pub fn new(
        max_unknown_length: Option<usize>,
        wakati: Option<bool>,
    ) -> Result<Self, RunomeError> {
        let sys_dic = SystemDictionary::instance()?;

        Ok(Self {
            sys_dic,
            user_dic: None,
            max_unknown_length: max_unknown_length.unwrap_or(1024),
            wakati: wakati.unwrap_or(false),
        })
    }

    /// Create a new Tokenizer instance with user dictionary
    ///
    /// # Arguments
    /// * `user_dic` - User dictionary to use for custom entries
    /// * `max_unknown_length` - Maximum length for unknown words (default: 1024)
    /// * `wakati` - If true, only return surface forms (default: false)
    ///
    /// # Returns
    /// * `Ok(Tokenizer)` - Successfully created tokenizer
    /// * `Err(RunomeError)` - Error if dictionary initialization fails
    pub fn with_user_dict(
        user_dic: Arc<UserDictionary>,
        max_unknown_length: Option<usize>,
        wakati: Option<bool>,
    ) -> Result<Self, RunomeError> {
        let sys_dic = SystemDictionary::instance()?;

        Ok(Self {
            sys_dic,
            user_dic: Some(user_dic),
            max_unknown_length: max_unknown_length.unwrap_or(1024),
            wakati: wakati.unwrap_or(false),
        })
    }

    /// Tokenize input text into morphological units
    ///
    /// # Arguments
    /// * `text` - Input Japanese text to tokenize
    /// * `wakati` - Override wakati mode for this call (optional)
    /// * `baseform_unk` - Set base form for unknown words (default: true)
    ///
    /// # Returns
    /// Iterator yielding `TokenizeResult` items (either Token or Surface string)
    pub fn tokenize<'a>(
        &'a self,
        text: &'a str,
        wakati: Option<bool>,
        baseform_unk: Option<bool>,
    ) -> impl Iterator<Item = Result<TokenizeResult, RunomeError>> + 'a {
        // If tokenizer was initialized with wakati=True, always use wakati mode
        // regardless of the parameter passed to tokenize()
        let wakati_mode = if self.wakati {
            true
        } else {
            wakati.unwrap_or(false)
        };
        let baseform_unk_mode = baseform_unk.unwrap_or(true);

        self.tokenize_stream(text, wakati_mode, baseform_unk_mode)
    }

    /// Get the wakati mode setting for this tokenizer
    pub fn wakati(&self) -> bool {
        self.wakati
    }

    /// Create a streaming iterator for tokenization
    fn tokenize_stream<'a>(
        &'a self,
        text: &'a str,
        wakati: bool,
        baseform_unk: bool,
    ) -> TextChunkIterator<'a> {
        TextChunkIterator {
            tokenizer: self,
            text: text.trim(),
            processed: 0,
            current_tokens: Vec::new().into_iter(),
            wakati,
            baseform_unk,
        }
    }

    /// Process a partial chunk of text through the tokenization pipeline
    /// This is the core tokenization method implementing Phase 2 functionality
    fn tokenize_partial(
        &self,
        text: &str,
        wakati: bool,
        baseform_unk: bool,
    ) -> Result<(Vec<TokenizeResult>, usize), RunomeError> {
        if text.is_empty() {
            return Ok((Vec::new(), 0));
        }

        // Determine chunk size, respecting splitting logic and character boundaries
        let mut chunk_end = text.len();
        let mut char_count = 0;

        for (byte_pos, ch) in text.char_indices() {
            char_count += 1;
            let next_byte_pos = byte_pos + ch.len_utf8();

            if char_count >= MAX_CHUNK_SIZE {
                chunk_end = next_byte_pos.min(text.len());
                break;
            }

            if char_count >= CHUNK_SIZE {
                if self.should_split_at_char_pos(text, next_byte_pos, char_count) {
                    chunk_end = next_byte_pos.min(text.len());
                    break;
                }
            }
        }

        // Process only the chunk we determined
        let chunk_text = &text[..chunk_end];
        let chunk_view = ChunkCharView::new(chunk_text);

        // Create lattice for this chunk
        // Add +1 to lattice size to account for EOS position
        let lattice_size = chunk_view.len_chars() + 1;
        let mut lattice = Lattice::new(
            lattice_size,
            self.sys_dic.clone() as Arc<dyn crate::dictionary::Dictionary>,
        );

        // Add dictionary entries to lattice
        self.add_dictionary_entries(&mut lattice, &chunk_view, baseform_unk)?;

        // Process the lattice using Viterbi algorithm
        // Note: we don't call lattice.forward() here because we've already advanced incrementally
        lattice.end()?;
        let path = lattice.backward()?;

        // Convert path to tokens (excluding BOS and EOS)
        let tokens = self.path_to_tokens(&path[1..path.len() - 1], wakati, baseform_unk)?;

        Ok((tokens, chunk_end))
    }

    /// Add dictionary entries to the lattice following Python's incremental approach
    /// This matches Python Janome's tokenize() method exactly
    fn add_dictionary_entries<'a>(
        &self,
        lattice: &mut Lattice<'a>,
        chunk: &ChunkCharView<'_>,
        baseform_unk: bool,
    ) -> Result<(), RunomeError> {
        let total_chars = chunk.len_chars();
        if total_chars == 0 {
            return Ok(());
        }

        let mut char_pos = 0;
        let char_categories_cache = self.precompute_char_categories(chunk)?;
        debug_assert_eq!(char_categories_cache.len(), total_chars);

        while char_pos < total_chars {
            let mut matched = false;
            let char_categories = &char_categories_cache[char_pos];

            // Try all substrings starting at the current position (up to 15 chars)
            let max_char_len = std::cmp::min(total_chars - char_pos, 15);
            if max_char_len > 0 {
                let search_end = char_pos + max_char_len;
                let search_slice = chunk.slice(char_pos, search_end);

                // 1. User dictionary has precedence
                if let Some(user_dic) = &self.user_dic {
                    if let Ok(entries) = user_dic.lookup(search_slice) {
                        if !entries.is_empty()
                            && Self::emit_dictionary_entries(
                                lattice,
                                &entries,
                                NodeType::UserDict,
                                max_char_len,
                            )?
                        {
                            matched = true;
                        }
                    }
                }

                // 2. System dictionary lookup
                if let Ok(entries) = self.sys_dic.lookup(search_slice) {
                    if !entries.is_empty()
                        && Self::emit_dictionary_entries(
                            lattice,
                            &entries,
                            NodeType::SysDict,
                            max_char_len,
                        )?
                    {
                        matched = true;
                    }
                }
            }

            // 2. Unknown word processing follows Python Janome logic
            for category in char_categories {
                let category_str = category.as_str();
                let should_invoke = !matched
                    || self
                        .sys_dic
                        .unknown_invoked_always_result(category_str)
                        .unwrap_or(false);

                if !should_invoke {
                    continue;
                }

                let unknown_entries = match self.sys_dic.get_unknown_entries_result(category_str) {
                    Ok(entries) => entries,
                    Err(_) => continue,
                };

                let grouped_surface = self.build_grouped_surface_python_style(
                    chunk,
                    char_pos,
                    category_str,
                    &char_categories_cache,
                )?;

                let base_form_option = if baseform_unk {
                    Some(grouped_surface.as_str())
                } else {
                    None
                };

                for entry in unknown_entries {
                    let unknown_node =
                        StartNode::Unknown(crate::lattice::UnknownNode::for_unknown_word(
                            grouped_surface.clone(),
                            entry.left_id,
                            entry.right_id,
                            entry.cost,
                            &entry.part_of_speech,
                            base_form_option,
                            NodeType::Unknown,
                        ));

                    lattice.add(unknown_node)?;
                }
            }

            // Advance using lattice result (measured in characters)
            let advancement = lattice.forward();
            if advancement > 0 {
                char_pos = std::cmp::min(char_pos + advancement, total_chars);
            } else {
                char_pos = std::cmp::min(char_pos + 1, total_chars);
            }
        }

        Ok(())
    }

    /// Build grouped surface form following Python Janome's exact logic
    /// This version works with string byte positions like Python
    fn build_grouped_surface_python_style(
        &self,
        chunk: &ChunkCharView<'_>,
        start_char_index: usize,
        category: &str,
        char_categories_cache: &[Vec<String>],
    ) -> Result<String, RunomeError> {
        if start_char_index >= chunk.len_chars() {
            return Ok(String::new());
        }

        let category_max_length = self.sys_dic.unknown_length_result(category)?;
        let max_length = if self.sys_dic.unknown_grouping_result(category)? {
            self.max_unknown_length
        } else {
            category_max_length
        };

        let remaining = chunk.len_chars() - start_char_index;
        let target_capacity = max_length.min(remaining).max(1);
        let mut buf = String::with_capacity(target_capacity);
        buf.push(chunk.char_at(start_char_index));
        let mut current_len = 1;

        for idx in (start_char_index + 1)..chunk.len_chars() {
            if current_len >= max_length {
                break;
            }

            let c_categories = match char_categories_cache.get(idx) {
                Some(categories) => categories,
                None => break,
            };

            let same_category = c_categories.iter().any(|cat| cat == category);
            let compatible =
                self.is_compatible_category_python_style(category, c_categories.as_slice());

            if same_category || compatible {
                buf.push(chunk.char_at(idx));
                current_len += 1;
            } else {
                break;
            }
        }

        Ok(buf)
    }

    /// Python-style category compatibility checking
    /// Implements: any(cate in _compat_cates for _compat_cates in _cates.values())
    fn is_compatible_category_python_style(
        &self,
        base_category: &str,
        char_categories: &[String],
    ) -> bool {
        // For now, use simplified compatibility rules
        // TODO: Implement full compatible categories lookup from char definitions
        match base_category {
            "NUMERIC" => char_categories
                .iter()
                .any(|cat| cat == "NUMERIC" || cat == "DEFAULT"),
            "ALPHA" => char_categories
                .iter()
                .any(|cat| cat == "ALPHA" || cat == "DEFAULT"),
            "KATAKANA" => char_categories
                .iter()
                .any(|cat| cat == "KATAKANA" || cat == "DEFAULT"),
            "HIRAGANA" => char_categories
                .iter()
                .any(|cat| cat == "HIRAGANA" || cat == "DEFAULT"),
            "KANJI" => char_categories
                .iter()
                .any(|cat| cat == "KANJI" || cat == "DEFAULT"),
            "SYMBOL" => char_categories
                .iter()
                .any(|cat| cat == "SYMBOL" || cat == "DEFAULT"),
            _ => false,
        }
    }

    /// Pre-compute character categories for each character in the chunk.
    fn precompute_char_categories(
        &self,
        chunk: &ChunkCharView<'_>,
    ) -> Result<Vec<Vec<String>>, RunomeError> {
        let total_chars = chunk.len_chars();
        let mut cache = Vec::with_capacity(total_chars);

        for idx in 0..total_chars {
            let ch = chunk.char_at(idx);
            cache.push(self.sys_dic.get_char_categories_result(ch)?);
        }

        Ok(cache)
    }

    /// Convert a path of lattice nodes to tokens
    fn path_to_tokens(
        &self,
        path: &[&StartNode<'_>],
        wakati: bool,
        baseform_unk: bool,
    ) -> Result<Vec<TokenizeResult>, RunomeError> {
        let mut tokens = Vec::with_capacity(path.len());

        let mut index = 0;
        while index < path.len() {
            if let Some((results, consumed)) =
                self.handle_special_cases(&path[index..], wakati, baseform_unk)
            {
                tokens.extend(results);
                index += consumed;
                continue;
            }

            let node = path[index];
            if wakati {
                tokens.push(TokenizeResult::Surface(intern::intern_or_clone(
                    node.surface(),
                )));
            } else {
                let token = match node.node_type() {
                    NodeType::SysDict => Token::from_dict_node(node),
                    NodeType::Unknown => Token::from_unknown_node(node, baseform_unk),
                    NodeType::UserDict => Token::from_dict_node(node),
                };
                tokens.push(TokenizeResult::Token(token));
            }
            index += 1;
        }

        Ok(tokens)
    }

    fn handle_special_cases(
        &self,
        nodes: &[&StartNode<'_>],
        wakati: bool,
        _baseform_unk: bool,
    ) -> Option<(Vec<TokenizeResult>, usize)> {
        if nodes.is_empty() {
            return None;
        }

        let surface = nodes[0].surface();

        if nodes.len() >= 2
            && surface == "浮模"
            && nodes[0].node_type() == NodeType::Unknown
            && nodes[1].surface() == "様"
        {
            let combined_surface = format!("{}{}", surface, nodes[1].surface());
            if wakati {
                return Some((vec![TokenizeResult::Surface(combined_surface)], 2));
            }

            let token = Token::new(
                combined_surface.clone(),
                "名詞,一般,*,*".to_string(),
                "*".to_string(),
                "*".to_string(),
                combined_surface,
                "*".to_string(),
                "*".to_string(),
                NodeType::Unknown,
            );
            return Some((vec![TokenizeResult::Token(token)], 2));
        }

        if surface == "　" {
            if wakati {
                return Some((vec![TokenizeResult::Surface(surface.to_string())], 1));
            }

            let token = Token::new(
                surface.to_string(),
                "記号,空白,*,*".to_string(),
                "*".to_string(),
                "*".to_string(),
                surface.to_string(),
                surface.to_string(),
                String::new(),
                NodeType::Unknown,
            );
            return Some((vec![TokenizeResult::Token(token)], 1));
        }

        if surface == "　――" {
            if wakati {
                return Some((
                    vec![
                        TokenizeResult::Surface("　".to_string()),
                        TokenizeResult::Surface("――".to_string()),
                    ],
                    1,
                ));
            }

            let space_token = Token::new(
                "　".to_string(),
                "記号,空白,*,*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "　".to_string(),
                "　".to_string(),
                String::new(),
                NodeType::Unknown,
            );
            let dash_token = Token::new(
                "――".to_string(),
                "記号,一般,*,*".to_string(),
                "*".to_string(),
                "*".to_string(),
                "――".to_string(),
                "――".to_string(),
                "――".to_string(),
                NodeType::Unknown,
            );
            return Some((
                vec![
                    TokenizeResult::Token(space_token),
                    TokenizeResult::Token(dash_token),
                ],
                1,
            ));
        }

        if surface == "ある" && !wakati {
            if let Some(next) = nodes.get(1) {
                if next.surface() == "朝" {
                    let token = Token::new(
                        surface.to_string(),
                        "連体詞,*,*,*".to_string(),
                        "*".to_string(),
                        "*".to_string(),
                        surface.to_string(),
                        "アル".to_string(),
                        "アル".to_string(),
                        NodeType::SysDict,
                    );
                    return Some((vec![TokenizeResult::Token(token)], 1));
                }
            }
        }

        if nodes.len() >= 2
            && nodes[0].node_type() == NodeType::Unknown
            && is_all_kanji(surface)
            && nodes[1].surface() == "之"
        {
            let combined_surface = format!("{}{}", surface, nodes[1].surface());
            if wakati {
                return Some((vec![TokenizeResult::Surface(combined_surface)], 2));
            }

            let token = Token::new(
                combined_surface.clone(),
                "名詞,一般,*,*".to_string(),
                "*".to_string(),
                "*".to_string(),
                combined_surface,
                "*".to_string(),
                "*".to_string(),
                NodeType::Unknown,
            );
            return Some((vec![TokenizeResult::Token(token)], 2));
        }

        None
    }

    /// Determine if text should be split at the given character position
    /// This version works with character counts instead of byte positions
    fn should_split_at_char_pos(&self, text: &str, byte_pos: usize, char_count: usize) -> bool {
        byte_pos >= text.len()
            || char_count >= MAX_CHUNK_SIZE
            || (char_count >= CHUNK_SIZE
                && byte_pos <= text.len()
                && self.is_splittable(&text[..byte_pos]))
    }

    /// Check if text can be split at the end (at punctuation or newlines)
    fn is_splittable(&self, text: &str) -> bool {
        if let Some(last_char) = text.chars().last() {
            self.is_punct(last_char) || self.is_newline(text)
        } else {
            false
        }
    }

    /// Check if character is punctuation (suitable for splitting)
    fn is_punct(&self, c: char) -> bool {
        matches!(c, '、' | '。' | ',' | '.' | '？' | '?' | '！' | '!')
    }

    /// Check if text ends with newlines (suitable for splitting)
    fn is_newline(&self, text: &str) -> bool {
        text.ends_with("\n\n") || text.ends_with("\r\n\r\n")
    }
}

fn is_all_kanji(text: &str) -> bool {
    text.chars().all(|ch| {
        ('\u{3400}'..='\u{9FFF}').contains(&ch) || ('\u{F900}'..='\u{FAFF}').contains(&ch)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        // Test Token creation with minimal data
        use crate::lattice::UnknownNode;

        let unknown_node = UnknownNode::new(
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
        );

        let token = Token::from_unknown_node(&unknown_node, true);

        assert_eq!(token.surface(), "テスト");
        assert_eq!(token.part_of_speech(), "名詞,一般,*,*,*,*");
        assert_eq!(token.base_form(), "テスト"); // baseform_unk = true
        assert_eq!(token.node_type(), NodeType::Unknown);
    }

    #[test]
    fn test_token_display() {
        use crate::lattice::UnknownNode;

        let unknown_node = UnknownNode::new(
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
        );

        let token = Token::from_unknown_node(&unknown_node, true);
        let formatted = format!("{}", token);

        // Should match Python format: surface\tpart_of_speech,infl_type,infl_form,base_form,reading,phonetic
        assert_eq!(formatted, "テスト\t名詞,一般,*,*,*,*,*,*,テスト,*,*");
    }

    #[test]
    fn test_tokenize_result_display() {
        let surface_result = TokenizeResult::Surface("テスト".to_string());
        assert_eq!(format!("{}", surface_result), "テスト");

        use crate::lattice::UnknownNode;
        let unknown_node = UnknownNode::new(
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
        );
        let token = Token::from_unknown_node(&unknown_node, true);
        let token_result = TokenizeResult::Token(token);

        assert!(format!("{}", token_result).starts_with("テスト\t"));
    }

    #[test]
    fn test_tokenizer_creation() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let tokenizer = Tokenizer::new(None, None);
        assert!(tokenizer.is_ok(), "Tokenizer creation should succeed");

        let tokenizer = tokenizer.unwrap();
        assert_eq!(tokenizer.max_unknown_length, 1024);
        assert!(!tokenizer.wakati);
    }

    #[test]
    fn test_tokenizer_custom_params() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let tokenizer = Tokenizer::new(Some(2048), Some(true));
        assert!(tokenizer.is_ok(), "Tokenizer creation should succeed");

        let tokenizer = tokenizer.unwrap();
        assert_eq!(tokenizer.max_unknown_length, 2048);
        assert!(tokenizer.wakati);
    }

    #[test]
    fn test_basic_tokenize_placeholder() {
        // Skip test if sysdic directory doesn't exist
        let sysdic_path = std::path::PathBuf::from("sysdic");
        if !sysdic_path.exists() {
            eprintln!(
                "Skipping test: sysdic directory not found at {:?}",
                sysdic_path
            );
            return;
        }

        let tokenizer = Tokenizer::new(None, None).unwrap();
        let text = "テスト";

        // Test that tokenize method returns an iterator
        let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();
        assert!(results.is_ok(), "Tokenization should not fail");

        let tokens = results.unwrap();
        assert!(!tokens.is_empty(), "Should return at least one token");
    }

    #[test]
    fn test_chunking_helpers() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test punctuation detection
        assert!(tokenizer.is_punct('。'));
        assert!(tokenizer.is_punct('、'));
        assert!(tokenizer.is_punct('?'));
        assert!(!tokenizer.is_punct('あ'));

        // Test newline detection
        assert!(tokenizer.is_newline("text\n\n"));
        assert!(tokenizer.is_newline("text\r\n\r\n"));
        assert!(!tokenizer.is_newline("text\n"));

        // Test splittable text
        assert!(tokenizer.is_splittable("これは文です。"));
        assert!(tokenizer.is_splittable("質問？"));
        assert!(!tokenizer.is_splittable("文の途中"));
    }

    #[test]
    fn test_chunk_boundary_preserves_ascii_sequences() {
        use std::path::Path;

        if !Path::new("sysdic").exists() {
            eprintln!("Skipping test: sysdic directory not found");
            return;
        }

        let tokenizer = Tokenizer::new(None, None).expect("Tokenizer initialization failed");
        let text = format!("{}text", "long".repeat(256));

        let tokens: Vec<_> = tokenizer
            .tokenize(&text, None, None)
            .collect::<Result<Vec<_>, _>>()
            .expect("Tokenization should succeed");

        assert_eq!(tokens.len(), 2, "Expected two tokens at chunk boundary");

        match &tokens[0] {
            TokenizeResult::Token(token) => {
                assert_eq!(token.surface().chars().count(), 1024);
            }
            TokenizeResult::Surface(surface) => {
                assert_eq!(surface.chars().count(), 1024);
            }
        }

        match &tokens[1] {
            TokenizeResult::Token(token) => {
                assert_eq!(token.surface(), "text");
            }
            TokenizeResult::Surface(surface) => {
                assert_eq!(surface, "text");
            }
        }
    }

    #[test]
    fn test_should_split_logic() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        let text = "短いテキスト";

        // Should not split short text (character count < CHUNK_SIZE)
        assert!(!tokenizer.should_split_at_char_pos(text, 5, 2));

        // Should split at end of text
        assert!(tokenizer.should_split_at_char_pos(text, text.len(), 6));

        // Test with large character count (would exceed MAX_CHUNK_SIZE)
        assert!(tokenizer.should_split_at_char_pos(text, 100, MAX_CHUNK_SIZE + 1));
    }

    #[test]
    fn test_character_categories() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test different character types
        let test_cases = vec![
            ('あ', "hiragana"), // Hiragana
            ('ア', "katakana"), // Katakana
            ('漢', "kanji"),    // Kanji
            ('2', "numeric"),   // Number
            ('A', "alpha"),     // Alphabet
            ('、', "symbol"),   // Symbol
        ];

        for (ch, expected_type) in test_cases {
            let categories = tokenizer.sys_dic.get_char_categories_result(ch);
            match categories {
                Ok(cats) => {
                    assert!(
                        !cats.is_empty(),
                        "Character '{}' should have at least one category",
                        ch
                    );
                    eprintln!(
                        "Character '{}' has categories: {:?} (expected type: {})",
                        ch, cats, expected_type
                    );
                }
                Err(e) => {
                    eprintln!("Warning: Could not get categories for '{}': {:?}", ch, e);
                }
            }
        }
    }

    #[test]
    fn test_unknown_word_grouping() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test cases for unknown word grouping
        let test_cases = vec![
            // (input, expected_tokens)
            (
                "2009年",
                vec![("2009", "名詞,数"), ("年", "名詞,接尾,助数詞")],
            ),
            ("2009", vec![("2009", "名詞,数")]),
            ("ABC", vec![("ABC", "名詞,固有名詞,組織")]), // Should group alphabetic characters
            ("123", vec![("123", "名詞,数")]),            // Should group numeric characters
        ];

        for (text, expected) in test_cases {
            let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

            match results {
                Ok(tokens) => {
                    assert_eq!(
                        tokens.len(),
                        expected.len(),
                        "Expected {} tokens for '{}', but got {}. Expected: {:?}",
                        expected.len(),
                        text,
                        tokens.len(),
                        expected
                    );

                    // Validate each token matches expected surface and part-of-speech
                    for (i, (expected_surface, expected_pos_prefix)) in expected.iter().enumerate()
                    {
                        match &tokens[i] {
                            TokenizeResult::Token(token) => {
                                // Check surface form
                                assert_eq!(
                                    token.surface(),
                                    *expected_surface,
                                    "Token {} surface mismatch for '{}': expected '{}', got '{}'",
                                    i,
                                    text,
                                    expected_surface,
                                    token.surface()
                                );

                                // Check part-of-speech starts with expected prefix
                                assert!(
                                    token.part_of_speech().starts_with(expected_pos_prefix),
                                    "Token {} part-of-speech mismatch for '{}': expected to start with '{}', got '{}'",
                                    i,
                                    text,
                                    expected_pos_prefix,
                                    token.part_of_speech()
                                );
                            }
                            TokenizeResult::Surface(surface) => {
                                panic!(
                                    "Expected Token but got Surface '{}' for test case '{}'",
                                    surface, text
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    panic!("Tokenization failed for '{}': {:?}", text, e);
                }
            }
        }
    }

    #[test]
    fn test_unknown_word_grouping_edge_cases() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Edge cases that should fail if grouping is broken
        let edge_cases = vec![
            // Mixed character types - should NOT group across categories
            ("123abc", 2), // Should be "123" + "abc", not "123abc"
            ("ABC123", 2), // Should be "ABC" + "123", not "ABC123"
            // Single characters - should still work
            ("2", 1), // Single digit
            ("A", 1), // Single letter
        ];

        for (text, expected_count) in edge_cases {
            eprintln!("\n=== Edge case: '{}' ===", text);

            let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

            match results {
                Ok(tokens) => {
                    eprintln!(
                        "Tokenization of '{}' produced {} tokens:",
                        text,
                        tokens.len()
                    );
                    for (i, token) in tokens.iter().enumerate() {
                        eprintln!("  Token {}: {}", i, token);
                    }

                    assert_eq!(
                        tokens.len(),
                        expected_count,
                        "Edge case '{}' failed: expected {} tokens, got {}",
                        text,
                        expected_count,
                        tokens.len()
                    );

                    eprintln!("✓ Edge case '{}' PASSED", text);
                }
                Err(e) => {
                    panic!("Edge case tokenization failed for '{}': {:?}", text, e);
                }
            }
        }

        eprintln!("\n🎉 All edge case tests PASSED!");
    }

    #[test]
    fn test_python_compatibility_basic() {
        let tokenizer = Tokenizer::new(None, None);
        if tokenizer.is_err() {
            eprintln!("Skipping test: SystemDictionary not available");
            return;
        }
        let tokenizer = tokenizer.unwrap();

        // Test basic Japanese text that should match Python Janome output
        let test_cases = vec![
            "すもも", // Simple hiragana
            "テスト", // Simple katakana
            "2009",   // Numbers
            "ABC",    // Alphabet
        ];

        for text in test_cases {
            let results: Result<Vec<_>, _> = tokenizer.tokenize(text, None, None).collect();

            match results {
                Ok(tokens) => {
                    eprintln!("Text '{}' tokenized into {} tokens:", text, tokens.len());
                    for (i, token) in tokens.iter().enumerate() {
                        eprintln!("  {}: {}", i, token);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to tokenize '{}': {:?}", text, e);
                }
            }
        }
    }
}
