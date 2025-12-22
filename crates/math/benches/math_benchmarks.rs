//! Benchmarks for toraniko-math operations.
#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ndarray::{Array1, Array2};
use rand::Rng;
use toraniko_math::{
    CenterXSection, NormXSection, constrained_wls, exp_weights, weighted_least_squares, winsorize,
};

fn random_array(n: usize) -> Array1<f64> {
    let mut rng = rand::thread_rng();
    Array1::from_iter((0..n).map(|_| rng.r#gen::<f64>() * 0.1 - 0.05))
}

fn random_matrix(rows: usize, cols: usize) -> Array2<f64> {
    let mut rng = rand::thread_rng();
    Array2::from_shape_fn((rows, cols), |_| rng.r#gen::<f64>())
}

fn random_sector_matrix(n_assets: usize, n_sectors: usize) -> Array2<f64> {
    let mut rng = rand::thread_rng();
    let mut matrix = Array2::zeros((n_assets, n_sectors));
    for i in 0..n_assets {
        let sector = rng.gen_range(0..n_sectors);
        matrix[[i, sector]] = 1.0;
    }
    matrix
}

fn bench_winsorize(c: &mut Criterion) {
    let mut group = c.benchmark_group("winsorize");

    for size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = random_array(size);
            b.iter(|| winsorize(black_box(&data), black_box(0.05)).unwrap());
        });
    }

    group.finish();
}

fn bench_center_xsection(c: &mut Criterion) {
    let mut group = c.benchmark_group("center_xsection");

    for size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = random_array(size);
            let transform = CenterXSection::new(true);
            b.iter(|| transform.apply(black_box(&data)));
        });
    }

    group.finish();
}

fn bench_norm_xsection(c: &mut Criterion) {
    let mut group = c.benchmark_group("norm_xsection");

    for size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = random_array(size);
            let transform = NormXSection::new(0.0, 1.0);
            b.iter(|| transform.apply(black_box(&data)));
        });
    }

    group.finish();
}

fn bench_exp_weights(c: &mut Criterion) {
    let mut group = c.benchmark_group("exp_weights");

    for (window, half_life) in [(252, 126), (504, 126), (1000, 500)] {
        group.bench_with_input(
            BenchmarkId::new("window", format!("{window}_{half_life}")),
            &(window, half_life),
            |b, &(window, half_life)| {
                b.iter(|| exp_weights(black_box(window), black_box(half_life)));
            },
        );
    }

    group.finish();
}

fn bench_weighted_least_squares(c: &mut Criterion) {
    let mut group = c.benchmark_group("wls");
    group.sample_size(50);

    for (n_assets, n_factors) in [(100, 10), (500, 20), (1000, 50), (3000, 100)] {
        group.throughput(Throughput::Elements((n_assets * n_factors) as u64));
        group.bench_with_input(
            BenchmarkId::new("assets_factors", format!("{n_assets}x{n_factors}")),
            &(n_assets, n_factors),
            |b, &(n_assets, n_factors)| {
                let y = random_array(n_assets);
                let x = random_matrix(n_assets, n_factors);
                let weights = Array1::from_iter((0..n_assets).map(|i| (i + 1) as f64));

                b.iter(|| {
                    weighted_least_squares(black_box(&y), black_box(&x), black_box(&weights))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_constrained_wls(c: &mut Criterion) {
    let mut group = c.benchmark_group("constrained_wls");
    group.sample_size(30);

    for (n_assets, n_sectors, n_styles) in
        [(100, 11, 3), (500, 11, 3), (1000, 11, 5), (3000, 11, 5)]
    {
        group.bench_with_input(
            BenchmarkId::new("factor_estimation", format!("{n_assets}_{n_sectors}_{n_styles}")),
            &(n_assets, n_sectors, n_styles),
            |b, &(n_assets, n_sectors, n_styles)| {
                let returns = random_array(n_assets);
                let weights = Array1::from_iter((0..n_assets).map(|i| ((i + 1) * 1000) as f64));
                let sectors = random_sector_matrix(n_assets, n_sectors);
                let styles = random_matrix(n_assets, n_styles);

                b.iter(|| {
                    constrained_wls(
                        black_box(&returns),
                        black_box(&weights),
                        black_box(&sectors),
                        black_box(&styles),
                    )
                    .unwrap()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_winsorize,
    bench_center_xsection,
    bench_norm_xsection,
    bench_exp_weights,
    bench_weighted_least_squares,
    bench_constrained_wls,
);

criterion_main!(benches);
