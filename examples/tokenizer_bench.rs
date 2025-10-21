use runome::tokenizer::{TokenizeResult, Tokenizer};
use std::env;
use std::fs;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bench_mode = args.iter().any(|arg| arg == "--bench");
    let dump_all = args.iter().any(|arg| arg == "--dump");
    let input_path = args
        .iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .map(|s| s.as_str())
        .unwrap_or("fixtures/cases/text_lemon.txt");

    // Initialize tokenizer
    let tokenizer = match Tokenizer::new(None, None) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialize tokenizer: {}", e);
            std::process::exit(1);
        }
    };

    // Load test text
    let text = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(_) => {
            eprintln!(
                "Warning: could not read {}. Falling back to sample text.",
                input_path
            );
            "これは日本語のテスト文章です。形態素解析を行います。".to_string()
        }
    };

    if bench_mode {
        // Benchmark mode - run multiple iterations for profiling
        let iterations = 10;
        let mut total_tokens = 0;

        eprintln!("Running benchmark with {} iterations...", iterations);
        let start = Instant::now();

        for _ in 0..iterations {
            let tokens: Vec<_> = tokenizer
                .tokenize(&text, None, None)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            total_tokens += tokens.len();
        }

        let duration = start.elapsed();
        eprintln!("Processed {} tokens in {:?}", total_tokens, duration);
        eprintln!("Average time per iteration: {:?}", duration / iterations);
    } else {
        // Normal mode - single run with output
        let tokens: Vec<_> = tokenizer
            .tokenize(&text, None, None)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Print first 10 tokens
        for (i, token) in tokens.iter().take(10).enumerate() {
            println!("{}: {:?}", i, token);
        }
        println!("... ({} total tokens)", tokens.len());
        if dump_all {
            for (idx, token) in tokens.iter().enumerate() {
                match token {
                    TokenizeResult::Token(tok) => {
                        println!("#{} len={} {}", idx, tok.surface().chars().count(), tok);
                    }
                    TokenizeResult::Surface(surface) => {
                        println!("#{} len={} {}", idx, surface.chars().count(), surface);
                    }
                }
            }
        }
    }
}
