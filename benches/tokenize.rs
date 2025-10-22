use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use runome::RunomeError;
use runome::dictionary::SystemDictionary;
use runome::tokenizer::Tokenizer;

struct Fixture {
    name: &'static str,
    path: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "basic_sumomo",
        path: "fixtures/cases/basic_sumomo.txt",
    },
    Fixture {
        name: "text_lemon",
        path: "fixtures/cases/text_lemon.txt",
    },
    Fixture {
        name: "text_large",
        path: "fixtures/cases/text_large.txt",
    },
    Fixture {
        name: "text_large_nonjp",
        path: "fixtures/cases/text_large_nonjp.txt",
    },
];

fn resolve_sysdic_path() -> PathBuf {
    if let Ok(path) = env::var("SYSDIC_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return candidate;
        }
    }
    let fallback = Path::new("runome").join("sysdic");
    if fallback.exists() {
        fallback
    } else {
        panic!("Failed to resolve sysdic directory. Set SYSDIC_PATH explicitly.");
    }
}

fn count_tokens(tokenizer: &Tokenizer, text: &str, wakati: bool) -> Result<usize, RunomeError> {
    let mut count = 0usize;
    for result in tokenizer.tokenize(text, Some(wakati), Some(true)) {
        result?;
        count += 1;
    }
    Ok(count)
}

fn bench_dictionary_load(c: &mut Criterion) {
    let sysdic_path = resolve_sysdic_path();

    let mut group = c.benchmark_group("dictionary_load");
    group
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(4))
        .sample_size(15);

    group.bench_function("SystemDictionary::new", |b| {
        b.iter(|| {
            let dictionary =
                SystemDictionary::new(sysdic_path.as_path()).expect("failed to load sysdic");
            black_box(dictionary);
        });
    });

    group.finish();
}

fn bench_tokenize(c: &mut Criterion) {
    let tokenizer =
        Arc::new(Tokenizer::new(None, None).expect("failed to create tokenizer for benchmarks"));

    for fixture in FIXTURES {
        let raw_text = fs::read_to_string(fixture.path).unwrap_or_else(|err| {
            panic!(
                "failed to read fixture {} ({}): {}",
                fixture.name, fixture.path, err
            )
        });
        let text = Arc::<str>::from(raw_text);

        let morpheme_count =
            count_tokens(tokenizer.as_ref(), &text, false).expect("tokenization failed");
        let wakati_count =
            count_tokens(tokenizer.as_ref(), &text, true).expect("wakati tokenization failed");

        let mut group = c.benchmark_group(format!("tokenize/{}", fixture.name));
        group
            .warm_up_time(Duration::from_secs(2))
            .measurement_time(Duration::from_secs(6))
            .sample_size(25);

        // Measure throughput in bytes for full tokens
        group.throughput(Throughput::Bytes(text.len() as u64));
        {
            let tokenizer = Arc::clone(&tokenizer);
            let text = Arc::clone(&text);
            group.bench_with_input(
                BenchmarkId::new("morpheme_bytes", fixture.name),
                &text.len(),
                move |b, _| {
                    let tokenizer_ref = tokenizer.as_ref();
                    let text_ref: &str = &text;
                    b.iter(|| {
                        let input = black_box(text_ref);
                        let count =
                            count_tokens(tokenizer_ref, input, false).expect("tokenization failed");
                        black_box(count);
                    });
                },
            );
        }

        // Measure throughput in bytes for wakati mode
        group.throughput(Throughput::Bytes(text.len() as u64));
        {
            let tokenizer = Arc::clone(&tokenizer);
            let text = Arc::clone(&text);
            group.bench_with_input(
                BenchmarkId::new("wakati_bytes", fixture.name),
                &text.len(),
                move |b, _| {
                    let tokenizer_ref = tokenizer.as_ref();
                    let text_ref: &str = &text;
                    b.iter(|| {
                        let input = black_box(text_ref);
                        let count =
                            count_tokens(tokenizer_ref, input, true).expect("wakati failed");
                        black_box(count);
                    });
                },
            );
        }

        // Measure throughput in tokens for full tokens
        group.throughput(Throughput::Elements(morpheme_count as u64));
        {
            let tokenizer = Arc::clone(&tokenizer);
            let text = Arc::clone(&text);
            group.bench_with_input(
                BenchmarkId::new("morpheme_tokens", fixture.name),
                &morpheme_count,
                move |b, _| {
                    let tokenizer_ref = tokenizer.as_ref();
                    let text_ref: &str = &text;
                    b.iter(|| {
                        let input = black_box(text_ref);
                        let count =
                            count_tokens(tokenizer_ref, input, false).expect("tokenization failed");
                        black_box(count);
                    });
                },
            );
        }

        // Measure throughput in tokens for wakati mode
        group.throughput(Throughput::Elements(wakati_count as u64));
        {
            let tokenizer = Arc::clone(&tokenizer);
            let text = Arc::clone(&text);
            group.bench_with_input(
                BenchmarkId::new("wakati_tokens", fixture.name),
                &wakati_count,
                move |b, _| {
                    let tokenizer_ref = tokenizer.as_ref();
                    let text_ref: &str = &text;
                    b.iter(|| {
                        let input = black_box(text_ref);
                        let count =
                            count_tokens(tokenizer_ref, input, true).expect("wakati failed");
                        black_box(count);
                    });
                },
            );
        }

        group.finish();
    }
}

criterion_group!(benches, bench_dictionary_load, bench_tokenize);
criterion_main!(benches);
