use pyo3::exceptions::{PyException, PyNotImplementedError};
use pyo3::prelude::*;
use std::sync::Arc;

use crate::charfilter::{CharFilter, RegexReplaceCharFilter, UnicodeNormalizeCharFilter};
use crate::error::RunomeError;
use crate::tokenfilter::{
    CompoundNounFilter, ExtractAttributeFilter, LowerCaseFilter, POSKeepFilter, POSStopFilter,
    TokenCountFilter, TokenFilter, UpperCaseFilter,
};
use crate::tokenizer::{Token as RustToken, TokenizeResult, Tokenizer as RustTokenizer};

/// Python wrapper for RunomeError
impl From<RunomeError> for PyErr {
    fn from(err: RunomeError) -> PyErr {
        PyException::new_err(format!("{:?}", err))
    }
}

/// Python Token class - mirrors Janome Token exactly
#[pyclass(name = "Token")]
#[derive(Clone)]
pub struct PyToken {
    inner: RustToken,
}

#[pymethods]
impl PyToken {
    /// surface property
    #[getter]
    fn surface(&self) -> String {
        self.inner.surface().to_string()
    }

    /// part_of_speech property
    #[getter]
    fn part_of_speech(&self) -> String {
        self.inner.part_of_speech().to_string()
    }

    /// infl_type property
    #[getter]
    fn infl_type(&self) -> String {
        self.inner.infl_type().to_string()
    }

    /// infl_form property
    #[getter]
    fn infl_form(&self) -> String {
        self.inner.infl_form().to_string()
    }

    /// base_form property
    #[getter]
    fn base_form(&self) -> String {
        self.inner.base_form().to_string()
    }

    /// reading property
    #[getter]
    fn reading(&self) -> String {
        self.inner.reading().to_string()
    }

    /// phonetic property
    #[getter]
    fn phonetic(&self) -> String {
        self.inner.phonetic().to_string()
    }

    /// node_type property
    #[getter]
    fn node_type(&self) -> String {
        format!("{:?}", self.inner.node_type())
    }

    /// String representation matching Janome format
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Debug representation
    fn __repr__(&self) -> String {
        format!(
            "Token(surface='{}', part_of_speech='{}')",
            self.inner.surface(),
            self.inner.part_of_speech()
        )
    }
}

impl PyToken {
    fn from_rust_token(token: RustToken) -> Self {
        PyToken { inner: token }
    }
}

/// Python iterator for tokenization results
#[pyclass(name = "TokenIterator")]
pub struct PyTokenIterator {
    results: Vec<TokenizeResult>,
    index: usize,
}

#[pymethods]
impl PyTokenIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<PyObject>> {
        if self.index >= self.results.len() {
            return Ok(None);
        }

        let result = &self.results[self.index];
        self.index += 1;

        Python::with_gil(|py| {
            match result {
                TokenizeResult::Token(token) => {
                    // Return PyToken object - Rust tokenizer decided this should be a token
                    #[allow(deprecated)]
                    Ok(Some(PyToken::from_rust_token(token.clone()).into_py(py)))
                }
                TokenizeResult::Surface(surface) => {
                    // Return surface string - Rust tokenizer decided this should be wakati mode
                    #[allow(deprecated)]
                    Ok(Some(surface.clone().into_py(py)))
                }
            }
        })
    }
}

/// Python Tokenizer class - mirrors Janome Tokenizer exactly
#[pyclass(name = "Tokenizer")]
#[derive(Clone)]
pub struct PyTokenizer {
    inner: RustTokenizer,
}

#[pymethods]
impl PyTokenizer {
    /// Initialize Tokenizer with Janome-compatible parameters
    ///
    /// Args:
    ///     udic (str): User dictionary file path (CSV format) or directory path to compiled dictionary data (default: '')
    ///     udic_enc (str): Character encoding for user dictionary (default: 'utf8')
    ///     udic_type (str): User dictionary type - 'ipadic' or 'simpledic' (default: 'ipadic')
    ///     max_unknown_length (int): Maximum unknown word length (default: 1024)
    ///     wakati (bool): Wakati mode flag (default: False)
    #[new]
    #[pyo3(signature = (udic = "", *, udic_enc = "utf8", udic_type = "ipadic", max_unknown_length = 1024, wakati = false))]
    fn new(
        udic: &str,
        udic_enc: &str,
        udic_type: &str,
        max_unknown_length: usize,
        wakati: bool,
    ) -> PyResult<Self> {
        let tokenizer = if udic.is_empty() {
            // No user dictionary
            RustTokenizer::new(Some(max_unknown_length), Some(wakati))
                .map_err(|e| PyException::new_err(format!("Failed to create tokenizer: {:?}", e)))?
        } else {
            // Convert udic_type string to enum
            let dict_format = match udic_type {
                "ipadic" => crate::dictionary::user_dict::UserDictFormat::Ipadic,
                "simpledic" => crate::dictionary::user_dict::UserDictFormat::Simpledic,
                _ => {
                    return Err(PyException::new_err(format!(
                        "Unsupported user dictionary type: {}. Use 'ipadic' or 'simpledic'",
                        udic_type
                    )));
                }
            };

            // Convert encoding string to Rust encoding
            let encoding = match udic_enc {
                "utf8" | "utf-8" => encoding_rs::UTF_8,
                "euc-jp" => encoding_rs::EUC_JP,
                "shift_jis" | "sjis" => encoding_rs::SHIFT_JIS,
                _ => {
                    return Err(PyException::new_err(format!(
                        "Unsupported encoding: {}. Supported encodings are 'utf8', 'euc-jp', and 'shift_jis'",
                        udic_enc
                    )));
                }
            };

            // Load user dictionary
            let user_dict = {
                use std::path::Path;
                let connections = crate::dictionary::system_dict::SystemDictionary::instance()
                    .map_err(|e| {
                        PyException::new_err(format!("Failed to load system dictionary: {:?}", e))
                    })?
                    .connection_matrix();

                crate::dictionary::user_dict::UserDictionary::new_with_encoding(
                    Path::new(udic),
                    dict_format,
                    encoding,
                    connections,
                )
                .map_err(|e| {
                    PyException::new_err(format!("Failed to load user dictionary: {:?}", e))
                })?
            };

            // Create tokenizer with user dictionary
            RustTokenizer::with_user_dict(
                Arc::new(user_dict),
                Some(max_unknown_length),
                Some(wakati),
            )
            .map_err(|e| {
                PyException::new_err(format!(
                    "Failed to create tokenizer with user dictionary: {:?}",
                    e
                ))
            })?
        };

        Ok(PyTokenizer { inner: tokenizer })
    }

    /// Get version info to verify we're using the right code
    fn get_version_info(&self) -> String {
        format!("wakati_fix_v1_tokenizer_wakati_{}", self.inner.wakati())
    }

    /// Tokenize text with Janome-compatible parameters
    ///
    /// Args:
    ///     text (str): Input text to tokenize
    ///     wakati (bool): Override wakati mode (default: None)
    ///     baseform_unk (bool): Set base form for unknown words (default: True)
    ///
    /// Returns:
    ///     Iterator yielding Token objects (wakati=False) or strings (wakati=True)
    #[pyo3(signature = (text, wakati = None, baseform_unk = true))]
    fn tokenize(
        &self,
        py: Python<'_>,
        text: &str,
        wakati: Option<bool>,
        baseform_unk: bool,
    ) -> PyResult<PyTokenIterator> {
        // Let the Rust tokenizer handle wakati precedence
        let tokenizer = self.inner.clone();
        let text_owned = text.to_owned();
        let wakati_mode = wakati;
        let results: Result<Vec<_>, _> = py.allow_threads(move || {
            tokenizer
                .tokenize(&text_owned, wakati_mode, Some(baseform_unk))
                .collect()
        });

        let token_results =
            results.map_err(|e| PyException::new_err(format!("Tokenization failed: {:?}", e)))?;

        Ok(PyTokenIterator {
            results: token_results,
            index: 0,
        })
    }
}

/// Python CharFilter base class - mirrors Janome CharFilter
#[pyclass(name = "CharFilter", subclass)]
pub struct PyCharFilter;

#[pymethods]
impl PyCharFilter {
    /// Apply the filter to input text
    fn apply(&self, _text: &str) -> PyResult<String> {
        Err(PyNotImplementedError::new_err(
            "CharFilter.apply() must be implemented by subclass",
        ))
    }

    /// Callable interface for Python compatibility
    fn __call__(&self, text: &str) -> PyResult<String> {
        self.apply(text)
    }
}

/// Python wrapper for RegexReplaceCharFilter
#[pyclass(name = "RegexReplaceCharFilter", extends = PyCharFilter)]
pub struct PyRegexReplaceCharFilter {
    inner: RegexReplaceCharFilter,
}

#[pymethods]
impl PyRegexReplaceCharFilter {
    /// Create a new RegexReplaceCharFilter
    ///
    /// Args:
    ///     pattern (str): Regular expression pattern to match
    ///     replacement (str): Replacement string
    #[new]
    fn new(pattern: &str, replacement: &str) -> PyResult<(Self, PyCharFilter)> {
        let inner = RegexReplaceCharFilter::new(pattern, replacement)
            .map_err(|e| PyException::new_err(format!("Invalid regex pattern: {:?}", e)))?;
        Ok((PyRegexReplaceCharFilter { inner }, PyCharFilter))
    }

    /// Apply regex replacement to text
    fn apply(&self, text: &str) -> PyResult<String> {
        self.inner
            .apply(text)
            .map_err(|e| PyException::new_err(format!("CharFilter error: {:?}", e)))
    }

    /// Callable interface
    fn __call__(&self, text: &str) -> PyResult<String> {
        self.apply(text)
    }
}

/// Python wrapper for UnicodeNormalizeCharFilter
#[pyclass(name = "UnicodeNormalizeCharFilter", extends = PyCharFilter)]
pub struct PyUnicodeNormalizeCharFilter {
    inner: UnicodeNormalizeCharFilter,
}

#[pymethods]
impl PyUnicodeNormalizeCharFilter {
    /// Create a new UnicodeNormalizeCharFilter
    ///
    /// Args:
    ///     form (str): Normalization form - 'NFC', 'NFKC', 'NFD', or 'NFKD' (default: 'NFKC')
    #[new]
    #[pyo3(signature = (form = "NFKC"))]
    fn new(form: &str) -> PyResult<(Self, PyCharFilter)> {
        let inner = UnicodeNormalizeCharFilter::new(form)
            .map_err(|e| PyException::new_err(format!("Invalid normalization form: {:?}", e)))?;
        Ok((PyUnicodeNormalizeCharFilter { inner }, PyCharFilter))
    }

    /// Apply Unicode normalization to text
    fn apply(&self, text: &str) -> PyResult<String> {
        self.inner
            .apply(text)
            .map_err(|e| PyException::new_err(format!("CharFilter error: {:?}", e)))
    }

    /// Callable interface
    fn __call__(&self, text: &str) -> PyResult<String> {
        self.apply(text)
    }
}

/// Enum to handle different TokenFilter output types
#[derive(Clone)]
pub enum PyTokenFilterOutput {
    Token(PyToken),
    String(String),
    Tuple(String, usize),
}

#[allow(deprecated)]
impl IntoPy<PyObject> for PyTokenFilterOutput {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            PyTokenFilterOutput::Token(token) => token.into_py(py),
            PyTokenFilterOutput::String(s) => s.into_py(py),
            PyTokenFilterOutput::Tuple(s, count) => (s, count).into_py(py),
        }
    }
}

/// Generic iterator for token filter results
#[pyclass(name = "TokenFilterIterator")]
pub struct PyTokenFilterIterator {
    results: Vec<PyTokenFilterOutput>,
    index: usize,
}

#[pymethods]
impl PyTokenFilterIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<PyObject>> {
        if self.index >= self.results.len() {
            return Ok(None);
        }

        let result = self.results[self.index].clone();
        self.index += 1;

        #[allow(deprecated)]
        Python::with_gil(|py| Ok(Some(result.into_py(py))))
    }
}

/// Python TokenFilter base class - mirrors Janome TokenFilter
#[pyclass(name = "TokenFilter", subclass)]
pub struct PyTokenFilter;

#[pymethods]
impl PyTokenFilter {
    /// Apply the filter to token iterator
    fn apply(&self, _tokens: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Err(PyNotImplementedError::new_err(
            "TokenFilter.apply() must be implemented by subclass",
        ))
    }

    /// Callable interface for Python compatibility
    fn __call__(&self, tokens: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        self.apply(tokens)
    }
}

/// Helper function to extract tokens from Python iterator
fn extract_tokens_from_iterator(
    _py: Python,
    tokens: &Bound<'_, PyAny>,
) -> PyResult<Vec<RustToken>> {
    let mut rust_tokens = Vec::new();

    // Handle both iterator and list inputs
    let iter = if let Ok(iterator) = tokens.try_iter() {
        iterator
    } else {
        return Err(PyException::new_err(
            "Expected an iterator or iterable of Token objects",
        ));
    };

    for item in iter {
        let item = item?;

        // Try to extract PyToken
        if let Ok(py_token) = item.extract::<PyToken>() {
            rust_tokens.push(py_token.inner.clone());
        } else {
            return Err(PyException::new_err(
                "TokenFilter expects Token objects in the input iterator",
            ));
        }
    }

    Ok(rust_tokens)
}

/// Python wrapper for LowerCaseFilter
#[pyclass(name = "LowerCaseFilter", extends = PyTokenFilter)]
pub struct PyLowerCaseFilter {
    inner: LowerCaseFilter,
}

#[pymethods]
impl PyLowerCaseFilter {
    /// Create a new LowerCaseFilter
    #[new]
    fn new() -> (Self, PyTokenFilter) {
        (
            PyLowerCaseFilter {
                inner: LowerCaseFilter,
            },
            PyTokenFilter,
        )
    }

    /// Apply lowercase transformation to tokens
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let filtered: Vec<RustToken> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = filtered
            .into_iter()
            .map(|t| PyTokenFilterOutput::Token(PyToken::from_rust_token(t)))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for UpperCaseFilter
#[pyclass(name = "UpperCaseFilter", extends = PyTokenFilter)]
pub struct PyUpperCaseFilter {
    inner: UpperCaseFilter,
}

#[pymethods]
impl PyUpperCaseFilter {
    /// Create a new UpperCaseFilter
    #[new]
    fn new() -> (Self, PyTokenFilter) {
        (
            PyUpperCaseFilter {
                inner: UpperCaseFilter,
            },
            PyTokenFilter,
        )
    }

    /// Apply uppercase transformation to tokens
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let filtered: Vec<RustToken> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = filtered
            .into_iter()
            .map(|t| PyTokenFilterOutput::Token(PyToken::from_rust_token(t)))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for POSStopFilter
#[pyclass(name = "POSStopFilter", extends = PyTokenFilter)]
pub struct PyPOSStopFilter {
    inner: POSStopFilter,
}

#[pymethods]
impl PyPOSStopFilter {
    /// Create a new POSStopFilter
    ///
    /// Args:
    ///     pos_list (List[str]): List of POS tags to filter out
    #[new]
    fn new(pos_list: Vec<String>) -> (Self, PyTokenFilter) {
        (
            PyPOSStopFilter {
                inner: POSStopFilter::new(pos_list),
            },
            PyTokenFilter,
        )
    }

    /// Apply POS stop filtering
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let filtered: Vec<RustToken> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = filtered
            .into_iter()
            .map(|t| PyTokenFilterOutput::Token(PyToken::from_rust_token(t)))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for POSKeepFilter
#[pyclass(name = "POSKeepFilter", extends = PyTokenFilter)]
pub struct PyPOSKeepFilter {
    inner: POSKeepFilter,
}

#[pymethods]
impl PyPOSKeepFilter {
    /// Create a new POSKeepFilter
    ///
    /// Args:
    ///     pos_list (List[str]): List of POS tags to keep
    #[new]
    fn new(pos_list: Vec<String>) -> (Self, PyTokenFilter) {
        (
            PyPOSKeepFilter {
                inner: POSKeepFilter::new(pos_list),
            },
            PyTokenFilter,
        )
    }

    /// Apply POS keep filtering
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let filtered: Vec<RustToken> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = filtered
            .into_iter()
            .map(|t| PyTokenFilterOutput::Token(PyToken::from_rust_token(t)))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for CompoundNounFilter
#[pyclass(name = "CompoundNounFilter", extends = PyTokenFilter)]
pub struct PyCompoundNounFilter {
    inner: CompoundNounFilter,
}

#[pymethods]
impl PyCompoundNounFilter {
    /// Create a new CompoundNounFilter
    #[new]
    fn new() -> (Self, PyTokenFilter) {
        (
            PyCompoundNounFilter {
                inner: CompoundNounFilter,
            },
            PyTokenFilter,
        )
    }

    /// Apply compound noun combining
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let filtered: Vec<RustToken> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = filtered
            .into_iter()
            .map(|t| PyTokenFilterOutput::Token(PyToken::from_rust_token(t)))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for ExtractAttributeFilter (terminal filter)
#[pyclass(name = "ExtractAttributeFilter", extends = PyTokenFilter)]
pub struct PyExtractAttributeFilter {
    inner: ExtractAttributeFilter,
}

#[pymethods]
impl PyExtractAttributeFilter {
    /// Create a new ExtractAttributeFilter
    ///
    /// Args:
    ///     attr (str): Attribute to extract (default: 'surface')
    #[new]
    #[pyo3(signature = (attr = "surface"))]
    fn new(attr: &str) -> PyResult<(Self, PyTokenFilter)> {
        let inner = ExtractAttributeFilter::new(attr.to_string())
            .map_err(|e| PyException::new_err(format!("Invalid attribute: {:?}", e)))?;
        Ok((PyExtractAttributeFilter { inner }, PyTokenFilter))
    }

    /// Apply attribute extraction - returns strings
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let strings: Vec<String> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = strings
            .into_iter()
            .map(PyTokenFilterOutput::String)
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Python wrapper for TokenCountFilter (terminal filter)
#[pyclass(name = "TokenCountFilter", extends = PyTokenFilter)]
pub struct PyTokenCountFilter {
    inner: TokenCountFilter,
}

#[pymethods]
impl PyTokenCountFilter {
    /// Create a new TokenCountFilter
    ///
    /// Args:
    ///     attr (str): Attribute to count (default: 'surface')
    ///     sorted (bool): Whether to sort results by count (default: False)
    #[new]
    #[pyo3(signature = (attr = "surface", sorted = false))]
    fn new(attr: &str, sorted: bool) -> PyResult<(Self, PyTokenFilter)> {
        let inner = TokenCountFilter::new(attr.to_string(), sorted)
            .map_err(|e| PyException::new_err(format!("Invalid attribute: {:?}", e)))?;
        Ok((PyTokenCountFilter { inner }, PyTokenFilter))
    }

    /// Apply token counting - returns (string, count) tuples
    fn apply(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        let rust_tokens = extract_tokens_from_iterator(py, tokens)?;

        let counts: Vec<(String, usize)> = self.inner.apply(rust_tokens.into_iter()).collect();

        let results: Vec<PyTokenFilterOutput> = counts
            .into_iter()
            .map(|(s, c)| PyTokenFilterOutput::Tuple(s, c))
            .collect();

        Ok(PyTokenFilterIterator { results, index: 0 })
    }

    /// Callable interface
    fn __call__(&self, py: Python, tokens: &Bound<'_, PyAny>) -> PyResult<PyTokenFilterIterator> {
        self.apply(py, tokens)
    }
}

/// Wrapper to hold Python filter objects
pub enum PyFilterWrapper {
    CharFilter(Py<PyAny>),
    TokenFilter(Py<PyAny>),
}

impl PyFilterWrapper {
    fn apply_char_filter(&self, py: Python, text: &str) -> PyResult<String> {
        match self {
            PyFilterWrapper::CharFilter(filter) => {
                let result = filter.call_method1(py, "__call__", (text,))?;
                result.extract::<String>(py)
            }
            _ => Err(PyException::new_err("Not a CharFilter")),
        }
    }

    fn apply_token_filter(&self, py: Python, tokens: PyObject) -> PyResult<PyObject> {
        match self {
            PyFilterWrapper::TokenFilter(filter) => filter.call_method1(py, "__call__", (tokens,)),
            _ => Err(PyException::new_err("Not a TokenFilter")),
        }
    }
}

/// Python Analyzer class - complete text analysis pipeline
#[pyclass(name = "Analyzer")]
pub struct PyAnalyzer {
    char_filters: Vec<PyFilterWrapper>,
    tokenizer: PyTokenizer,
    token_filters: Vec<PyFilterWrapper>,
}

#[pymethods]
impl PyAnalyzer {
    /// Initialize Analyzer with filters and tokenizer
    ///
    /// Args:
    ///     char_filters (List[CharFilter]): CharFilters for preprocessing (default: [])
    ///     tokenizer (Tokenizer): Tokenizer for morphological analysis (default: new Tokenizer())
    ///     token_filters (List[TokenFilter]): TokenFilters for post-processing (default: [])
    #[new]
    #[pyo3(signature = (*, char_filters = vec![], tokenizer = None, token_filters = vec![]))]
    fn new(
        _py: Python,
        char_filters: Vec<Bound<'_, PyAny>>,
        tokenizer: Option<PyTokenizer>,
        token_filters: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Validate tokenizer
        let tokenizer = if let Some(t) = tokenizer {
            // Check if tokenizer is in wakati mode
            if t.inner.wakati() {
                return Err(PyException::new_err(
                    "Invalid argument: A Tokenizer with wakati=True option is not accepted.",
                ));
            }
            t
        } else {
            // Create default tokenizer
            PyTokenizer::new("", "utf8", "ipadic", 1024, false)?
        };

        // Wrap char filters
        let char_filters: Vec<PyFilterWrapper> = char_filters
            .into_iter()
            .map(|f| PyFilterWrapper::CharFilter(f.unbind()))
            .collect();

        // Wrap token filters
        let token_filters: Vec<PyFilterWrapper> = token_filters
            .into_iter()
            .map(|f| PyFilterWrapper::TokenFilter(f.unbind()))
            .collect();

        Ok(PyAnalyzer {
            char_filters,
            tokenizer,
            token_filters,
        })
    }

    /// Analyze text through the complete pipeline
    ///
    /// Args:
    ///     text (str): Input text to analyze
    ///
    /// Returns:
    ///     Iterator over analysis results (type depends on final TokenFilter)
    fn analyze(&self, py: Python, text: &str) -> PyResult<PyObject> {
        // Stage 1: Apply CharFilters sequentially
        let mut processed_text = text.to_string();
        for filter in &self.char_filters {
            processed_text = filter.apply_char_filter(py, &processed_text)?;
        }

        // Stage 2: Tokenize the preprocessed text
        let tokens = self.tokenizer.tokenize(py, &processed_text, None, true)?;
        #[allow(deprecated)]
        let mut current_iter = tokens.into_py(py);

        // Stage 3: Apply TokenFilters sequentially
        for filter in &self.token_filters {
            current_iter = filter.apply_token_filter(py, current_iter)?;
        }

        Ok(current_iter)
    }
}

/// Python module definition
#[pymodule]
fn runome(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Token and Tokenizer classes
    m.add_class::<PyToken>()?;
    m.add_class::<PyTokenizer>()?;
    m.add_class::<PyTokenIterator>()?;

    // CharFilter classes
    m.add_class::<PyCharFilter>()?;
    m.add_class::<PyRegexReplaceCharFilter>()?;
    m.add_class::<PyUnicodeNormalizeCharFilter>()?;

    // TokenFilter classes
    m.add_class::<PyTokenFilter>()?;
    m.add_class::<PyLowerCaseFilter>()?;
    m.add_class::<PyUpperCaseFilter>()?;
    m.add_class::<PyPOSStopFilter>()?;
    m.add_class::<PyPOSKeepFilter>()?;
    m.add_class::<PyCompoundNounFilter>()?;
    m.add_class::<PyExtractAttributeFilter>()?;
    m.add_class::<PyTokenCountFilter>()?;
    m.add_class::<PyTokenFilterIterator>()?;

    // Analyzer
    m.add_class::<PyAnalyzer>()?;

    Ok(())
}
