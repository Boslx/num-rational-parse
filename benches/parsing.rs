use criterion::{Criterion, black_box, criterion_group, criterion_main};
use num_rational::Rational32;
use num_rational_parse::RationalParse;

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_flex");

    group.bench_function("integer", |b| {
        b.iter(|| Rational32::from_str_flex(black_box("12345678")))
    });

    group.bench_function("fraction", |b| {
        b.iter(|| Rational32::from_str_flex(black_box("12345/67890")))
    });

    group.bench_function("decimal", |b| {
        b.iter(|| Rational32::from_str_flex(black_box("123.456789")))
    });

    group.bench_function("scientific", |b| {
        b.iter(|| Rational32::from_str_flex(black_box("1.2345e-6")))
    });

    group.bench_function("complex_underscores", |b| {
        b.iter(|| Rational32::from_str_flex(black_box("1_234.567_890e-1_2")))
    });

    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
