//! Golden diff tests.
//!
//! These integration tests load Janome-generated JSONL files from `golden/`
//! and assert that Runome emits identical token strings.

use anyhow::{anyhow, Context, Result};
use runome::{RunomeError, TokenizeResult, Tokenizer};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct GoldenToken {
    #[serde(rename = "string")]
    string_repr: String,
}

#[derive(Debug, Deserialize)]
struct GoldenCase {
    case: String,
    mode: String,
    text: String,
    tokens: Vec<GoldenToken>,
}

fn load_golden_case<P: AsRef<Path>>(path: P, case_name: &str) -> Result<GoldenCase> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open golden file {}", path.as_ref().display()))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.context("failed to read golden line")?;
        if line.trim().is_empty() {
            continue;
        }
        let golden: GoldenCase =
            serde_json::from_str(&line).context("failed to decode golden JSONL record")?;
        if golden.case == case_name {
            return Ok(golden);
        }
    }
    Err(anyhow!("case '{}' not found in {:?}", case_name, path.as_ref()))
}

fn runome_token_strings(tokenizer: &Tokenizer, input: &str) -> Result<Vec<String>> {
    let tokens = tokenizer
        .tokenize(input, None, None)
        .collect::<Result<Vec<_>, _>>()
        .context("tokenization failed")?;
    let strings = tokens
        .into_iter()
        .map(|token| match token {
            TokenizeResult::Token(token) => token.to_string(),
            TokenizeResult::Surface(surface) => surface,
        })
        .collect();
    Ok(strings)
}

#[test]
fn diff_basic_sumomo_against_janome() -> Result<()> {
    let golden_path = Path::new("golden/janome_full.jsonl");
    assert!(
        golden_path.exists(),
        "golden file missing; run `python tools/gen_golden.py --inputs fixtures/cases --mode full` first"
    );

    let golden =
        load_golden_case(golden_path, "basic_sumomo.txt").context("loading basic_sumomo case")?;
    assert_eq!(
        golden.mode, "full",
        "expected full-mode golden entry for basic_sumomo"
    );

    let tokenizer = match Tokenizer::new(None, None) {
        Ok(tokenizer) => tokenizer,
        Err(RunomeError::DictDirectoryNotFound { path }) => {
            eprintln!(
                "Skipping diff test because system dictionary was not found at {}",
                path
            );
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    let expected: Vec<String> = golden
        .tokens
        .iter()
        .map(|token| token.string_repr.clone())
        .collect();
    let actual = runome_token_strings(&tokenizer, &golden.text)?;

    assert_eq!(
        actual, expected,
        "Runome output deviates from Janome golden for {}",
        golden.case
    );
    Ok(())
}
