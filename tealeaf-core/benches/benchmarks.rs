use criterion::{criterion_group, criterion_main, Criterion};

mod common;
mod scenarios;

// Include generated protobuf code
pub mod proto {
    pub mod benchmark {
        include!(concat!(env!("OUT_DIR"), "/benchmark.rs"));
    }
}

use scenarios::*;

fn small_object_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_object");
    small_object::bench_encode(&mut group);
    small_object::bench_decode(&mut group);
    group.finish();
}

fn large_array_100_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_array_100");
    large_array::bench_encode(&mut group, 100);
    large_array::bench_decode(&mut group, 100);
    group.finish();
}

fn large_array_1000_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_array_1000");
    large_array::bench_encode(&mut group, 1000);
    large_array::bench_decode(&mut group, 1000);
    group.finish();
}

fn large_array_10000_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_array_10000");
    large_array::bench_encode(&mut group, 10000);
    large_array::bench_decode(&mut group, 10000);
    group.finish();
}

fn nested_structs_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_structs");
    nested_structs::bench_encode(&mut group, 2);
    nested_structs::bench_decode(&mut group, 2);
    group.finish();
}

fn nested_structs_100_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_structs_100");
    nested_structs::bench_encode(&mut group, 100);
    nested_structs::bench_decode(&mut group, 100);
    group.finish();
}

fn mixed_types_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_types");
    mixed_types::bench_encode(&mut group);
    mixed_types::bench_decode(&mut group);
    group.finish();
}

fn tabular_100_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("tabular_100");
    tabular_data::bench_encode(&mut group, 100);
    tabular_data::bench_decode(&mut group, 100);
    group.finish();
}

fn tabular_1000_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("tabular_1000");
    tabular_data::bench_encode(&mut group, 1000);
    tabular_data::bench_decode(&mut group, 1000);
    group.finish();
}

fn tabular_5000_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("tabular_5000");
    tabular_data::bench_encode(&mut group, 5000);
    tabular_data::bench_decode(&mut group, 5000);
    group.finish();
}

criterion_group!(
    benches,
    small_object_benchmarks,
    large_array_100_benchmarks,
    large_array_1000_benchmarks,
    large_array_10000_benchmarks,
    nested_structs_benchmarks,
    nested_structs_100_benchmarks,
    mixed_types_benchmarks,
    tabular_100_benchmarks,
    tabular_1000_benchmarks,
    tabular_5000_benchmarks,
);

criterion_main!(benches);
