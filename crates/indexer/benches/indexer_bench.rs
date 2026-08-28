//! Benchmarks for archaeologist-indexer.
//!
//! Measures:
//! - Full parse throughput per language for 1k / 10k / 100k lines
//! - Symbol extraction latency per language

use std::fmt::Write as _;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use archaeologist_indexer::{extract_symbols, parse, Lang};

fn gen_rust(n: usize) -> Vec<u8> {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(src, "pub fn func_{i}(x: u32) -> u32 {{ x + {i} }}");
    }
    src.into_bytes()
}

fn gen_python(n: usize) -> Vec<u8> {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(src, "def func_{i}(x):\n    return x + {i}");
    }
    src.into_bytes()
}

fn gen_javascript(n: usize) -> Vec<u8> {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(src, "function func_{i}(x) {{ return x + {i}; }}");
    }
    src.into_bytes()
}

fn gen_typescript(n: usize) -> Vec<u8> {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(
            src,
            "function func_{i}(x: number): number {{ return x + {i}; }}"
        );
    }
    src.into_bytes()
}

fn gen_go(n: usize) -> Vec<u8> {
    let mut src = String::from("package bench\n\n");
    for i in 0..n {
        let _ = writeln!(src, "func Func{i}(x int) int {{ return x + {i} }}");
    }
    src.into_bytes()
}

fn gen_java(n: usize) -> Vec<u8> {
    let mut src = String::from("class Bench {\n");
    for i in 0..n {
        let _ = writeln!(
            src,
            "    public static int func{i}(int x) {{ return x + {i}; }}"
        );
    }
    src.push('}');
    src.into_bytes()
}

fn gen_c(n: usize) -> Vec<u8> {
    let mut src = String::new();
    for i in 0..n {
        let _ = writeln!(src, "int func_{i}(int x) {{ return x + {i}; }}");
    }
    src.into_bytes()
}

fn gen_cpp(n: usize) -> Vec<u8> {
    let mut src = String::from("#include <cstdint>\n");
    for i in 0..n {
        let _ = writeln!(src, "int func_{i}(int x) {{ return x + {i}; }}");
    }
    src.into_bytes()
}

type GenFn = fn(usize) -> Vec<u8>;

struct LangConfig {
    lang: Lang,
    name: &'static str,
    gen: GenFn,
}

fn all_langs() -> Vec<LangConfig> {
    vec![
        LangConfig {
            lang: Lang::Rust,
            name: "rust",
            gen: gen_rust,
        },
        LangConfig {
            lang: Lang::Python,
            name: "python",
            gen: gen_python,
        },
        LangConfig {
            lang: Lang::JavaScript,
            name: "javascript",
            gen: gen_javascript,
        },
        LangConfig {
            lang: Lang::TypeScript,
            name: "typescript",
            gen: gen_typescript,
        },
        LangConfig {
            lang: Lang::Go,
            name: "go",
            gen: gen_go,
        },
        LangConfig {
            lang: Lang::Java,
            name: "java",
            gen: gen_java,
        },
        LangConfig {
            lang: Lang::C,
            name: "c",
            gen: gen_c,
        },
        LangConfig {
            lang: Lang::Cpp,
            name: "cpp",
            gen: gen_cpp,
        },
    ]
}

fn bench_parse_throughput(c: &mut Criterion) {
    let sizes: &[(usize, &str)] = &[(1_000, "1k"), (10_000, "10k"), (100_000, "100k")];

    let mut group = c.benchmark_group("parse_throughput");

    for cfg in all_langs() {
        for &(n, label) in sizes {
            let src = (cfg.gen)(n);
            group.throughput(Throughput::Bytes(src.len() as u64));
            group.bench_with_input(BenchmarkId::new(cfg.name, label), &src, |b, src| {
                b.iter(|| {
                    let r = parse(black_box(src), cfg.lang).unwrap();
                    black_box(r);
                });
            });
        }
    }

    group.finish();
}

fn bench_symbol_extraction(c: &mut Criterion) {
    let n = 1_000;

    let mut group = c.benchmark_group("symbol_extraction");

    for cfg in all_langs() {
        let src = (cfg.gen)(n);
        let result = parse(&src, cfg.lang).unwrap();
        group.bench_function(cfg.name, |b| {
            b.iter(|| {
                let syms = extract_symbols(black_box(&result), black_box(&src));
                black_box(syms);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parse_throughput, bench_symbol_extraction);
criterion_main!(benches);
