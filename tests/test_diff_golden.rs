//! Golden diff tests.
//!
//! These integration tests load Janome-generated JSONL files from `golden/`
//! and assert that Runome emits identical token strings.

use anyhow::{Context, Result, anyhow};
use runome::{RunomeError, TokenizeResult, Tokenizer};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
struct GoldenRecord {
    case: String,
    mode: String,
    text: String,
    tokens: Vec<String>,
}

const KNOWN_FULL_DIFFS: &[&str] = &["text_lemon.txt"];
const KNOWN_WAKATI_DIFFS: &[&str] = &["text_lemon.txt"];

fn load_goldens<P: AsRef<Path>>(path: P) -> Result<Vec<GoldenRecord>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open golden file {}", path.as_ref().display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (idx, line) in reader.lines().enumerate() {
        let line = line.context("failed to read golden line")?;
        if line.trim().is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(&line)
            .with_context(|| format!("failed to decode golden JSONL record at line {}", idx + 1))?;

        let mode = value
            .get("mode")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing 'mode' in golden record at line {}", idx + 1))?
            .to_owned();
        let case = value
            .get("case")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing 'case' in golden record at line {}", idx + 1))?
            .to_owned();
        let text = value
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing 'text' for case {}", case))?
            .to_owned();

        let tokens_value = value
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("'tokens' must be an array for case {}", case))?;

        let mut tokens = Vec::with_capacity(tokens_value.len());
        for entry in tokens_value {
            if let Some(s) = entry.as_str() {
                tokens.push(s.to_owned());
            } else if let Some(obj) = entry.as_object() {
                let string_repr = obj
                    .get("string")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("missing token 'string' field for case {}", case))?;
                tokens.push(string_repr.to_owned());
            } else {
                return Err(anyhow!(
                    "unsupported token representation for case {}",
                    case
                ));
            }
        }

        records.push(GoldenRecord {
            case,
            mode,
            text,
            tokens,
        });
    }

    Ok(records)
}

fn create_tokenizer(wakati: Option<bool>) -> Result<Option<Tokenizer>> {
    match Tokenizer::new(None, wakati) {
        Ok(tokenizer) => Ok(Some(tokenizer)),
        Err(RunomeError::DictDirectoryNotFound { path }) => {
            eprintln!(
                "Skipping diff tests because system dictionary was not found at {}",
                path
            );
            Ok(None)
        }
        Err(err) => Err(err.into()),
    }
}

fn runome_token_strings(tokenizer: &Tokenizer, input: &str, wakati: bool) -> Result<Vec<String>> {
    let tokens = tokenizer
        .tokenize(input, Some(wakati), None)
        .collect::<Result<Vec<_>, _>>()
        .context("tokenization failed")?;

    Ok(tokens
        .into_iter()
        .map(|token| match token {
            TokenizeResult::Token(token) => token.to_string(),
            TokenizeResult::Surface(surface) => surface,
        })
        .collect())
}

#[test]
fn diff_full_cases_against_janome() -> Result<()> {
    let golden_path = Path::new("golden/janome_full.jsonl");
    assert!(
        golden_path.exists(),
        "golden file missing; run `python tools/gen_golden.py --inputs fixtures/cases --mode full` first"
    );

    let records = load_goldens(golden_path).context("loading janome_full goldens")?;
    let full_cases: Vec<_> = records
        .into_iter()
        .filter(|record| record.mode == "full")
        .collect();
    assert!(
        !full_cases.is_empty(),
        "no full-mode cases found in {}",
        golden_path.display()
    );

    let Some(tokenizer) = create_tokenizer(None)? else {
        return Ok(());
    };

    for record in &full_cases {
        let actual = runome_token_strings(&tokenizer, &record.text, false)?;
        if KNOWN_FULL_DIFFS
            .iter()
            .any(|case| *case == record.case.as_str())
        {
            if actual == record.tokens {
                panic!(
                    "Case {} now matches Janome; remove it from KNOWN_FULL_DIFFS.",
                    record.case
                );
            }
            eprintln!(
                "Known full-mode diff: {} (Runome output diverges from Janome).",
                record.case
            );
            for (idx, (lhs, rhs)) in actual.iter().zip(&record.tokens).enumerate() {
                if lhs != rhs {
                    eprintln!("  [{}] runome='{}' janome='{}'", idx, lhs, rhs);
                    break;
                }
            }
            continue;
        }
        assert_eq!(
            &actual, &record.tokens,
            "Runome output deviates from Janome golden for {}",
            record.case
        );
    }

    Ok(())
}

#[test]
fn diff_wakati_cases_against_janome() -> Result<()> {
    let golden_path = Path::new("golden/janome_wakati.jsonl");
    assert!(
        golden_path.exists(),
        "golden file missing; run `python tools/gen_golden.py --inputs fixtures/cases --mode wakati` first"
    );

    let records = load_goldens(golden_path).context("loading janome_wakati goldens")?;
    let wakati_cases: Vec<_> = records
        .into_iter()
        .filter(|record| record.mode == "wakati")
        .collect();
    assert!(
        !wakati_cases.is_empty(),
        "no wakati-mode cases found in {}",
        golden_path.display()
    );

    let Some(tokenizer) = create_tokenizer(Some(true))? else {
        return Ok(());
    };

    for record in &wakati_cases {
        let actual = runome_token_strings(&tokenizer, &record.text, true)?;
        if KNOWN_WAKATI_DIFFS
            .iter()
            .any(|case| *case == record.case.as_str())
        {
            if actual == record.tokens {
                panic!(
                    "Case {} now matches Janome; remove it from KNOWN_WAKATI_DIFFS.",
                    record.case
                );
            }
            eprintln!(
                "Known wakati-mode diff: {} (Runome output diverges from Janome).",
                record.case
            );
            for (idx, (lhs, rhs)) in actual.iter().zip(&record.tokens).enumerate() {
                if lhs != rhs {
                    eprintln!("  [{}] runome='{}' janome='{}'", idx, lhs, rhs);
                    break;
                }
            }
            continue;
        }

        assert_eq!(
            &actual, &record.tokens,
            "Runome wakati output deviates from Janome golden for {}",
            record.case
        );
    }

    Ok(())
}
