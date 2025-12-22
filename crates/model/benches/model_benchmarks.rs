//! Benchmarks for toraniko-model factor estimation.
#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ndarray::{Array1, Array2};
use rand::Rng;
use toraniko_model::WlsFactorEstimator;
use toraniko_traits::FactorEstimator;

fn random_returns(n: usize) -> Array1<f64> {
    let mut rng = rand::thread_rng();
    Array1::from_iter((0..n).map(|_| rng.r#gen::<f64>() * 0.1 - 0.05))
}

fn random_market_caps(n: usize) -> Array1<f64> {
    let mut rng = rand::thread_rng();
    Array1::from_iter((0..n).map(|_| rng.r#gen::<f64>() * 1e12 + 1e9))
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

fn random_style_scores(n_assets: usize, n_styles: usize) -> Array2<f64> {
    let mut rng = rand::thread_rng();
    Array2::from_shape_fn((n_assets, n_styles), |_| rng.r#gen::<f64>() * 2.0 - 1.0)
}

fn bench_wls_estimator_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("wls_estimator_single");
    group.sample_size(50);

    // Simulate realistic factor model dimensions
    let scenarios = [
        (100, 11, 3, "small_universe"),   // 100 stocks, 11 sectors, 3 styles
        (500, 11, 5, "medium_universe"),  // 500 stocks
        (1000, 11, 5, "large_universe"),  // 1000 stocks
        (3000, 11, 5, "full_universe"),   // Full US universe
        (5000, 11, 8, "global_universe"), // Global universe
    ];

    for (n_assets, n_sectors, n_styles, name) in scenarios {
        group.throughput(Throughput::Elements(n_assets as u64));
        group.bench_with_input(
            BenchmarkId::new("scenario", name),
            &(n_assets, n_sectors, n_styles),
            |b, &(n_assets, n_sectors, n_styles)| {
                let estimator = WlsFactorEstimator::new();
                let returns = random_returns(n_assets);
                let mkt_caps = random_market_caps(n_assets);
                let sectors = random_sector_matrix(n_assets, n_sectors);
                let styles = random_style_scores(n_assets, n_styles);

                b.iter(|| {
                    estimator
                        .estimate_single(
                            black_box(&returns),
                            black_box(&mkt_caps),
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

fn bench_factor_estimation_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("factor_estimation_scaling");
    group.sample_size(30);

    // Scale number of assets
    for n_assets in [100, 500, 1000, 2000, 3000, 5000] {
        let n_sectors = 11;
        let n_styles = 5;

        group.throughput(Throughput::Elements(n_assets as u64));
        group.bench_with_input(
            BenchmarkId::new("n_assets", n_assets),
            &n_assets,
            |b, &n_assets| {
                let estimator = WlsFactorEstimator::new();
                let returns = random_returns(n_assets);
                let mkt_caps = random_market_caps(n_assets);
                let sectors = random_sector_matrix(n_assets, n_sectors);
                let styles = random_style_scores(n_assets, n_styles);

                b.iter(|| {
                    estimator
                        .estimate_single(
                            black_box(&returns),
                            black_box(&mkt_caps),
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

fn bench_style_factor_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("style_factor_scaling");
    group.sample_size(30);

    let n_assets = 1000;
    let n_sectors = 11;

    // Scale number of style factors
    for n_styles in [1, 3, 5, 8, 10, 15, 20] {
        group.bench_with_input(
            BenchmarkId::new("n_styles", n_styles),
            &n_styles,
            |b, &n_styles| {
                let estimator = WlsFactorEstimator::new();
                let returns = random_returns(n_assets);
                let mkt_caps = random_market_caps(n_assets);
                let sectors = random_sector_matrix(n_assets, n_sectors);
                let styles = random_style_scores(n_assets, n_styles);

                b.iter(|| {
                    estimator
                        .estimate_single(
                            black_box(&returns),
                            black_box(&mkt_caps),
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

fn bench_sector_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("sector_scaling");
    group.sample_size(30);

    let n_assets = 1000;
    let n_styles = 5;

    // Scale number of sectors
    for n_sectors in [5, 11, 20, 30, 50] {
        group.bench_with_input(
            BenchmarkId::new("n_sectors", n_sectors),
            &n_sectors,
            |b, &n_sectors| {
                let estimator = WlsFactorEstimator::new();
                let returns = random_returns(n_assets);
                let mkt_caps = random_market_caps(n_assets);
                let sectors = random_sector_matrix(n_assets, n_sectors);
                let styles = random_style_scores(n_assets, n_styles);

                b.iter(|| {
                    estimator
                        .estimate_single(
                            black_box(&returns),
                            black_box(&mkt_caps),
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

fn bench_with_winsorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("winsorization_impact");
    group.sample_size(30);

    let n_assets = 1000;
    let n_sectors = 11;
    let n_styles = 5;

    // With winsorization
    group.bench_function("with_winsor", |b| {
        let estimator = WlsFactorEstimator::new(); // default has winsorization
        let returns = random_returns(n_assets);
        let mkt_caps = random_market_caps(n_assets);
        let sectors = random_sector_matrix(n_assets, n_sectors);
        let styles = random_style_scores(n_assets, n_styles);

        b.iter(|| {
            estimator
                .estimate_single(
                    black_box(&returns),
                    black_box(&mkt_caps),
                    black_box(&sectors),
                    black_box(&styles),
                )
                .unwrap()
        });
    });

    // Without winsorization
    group.bench_function("without_winsor", |b| {
        let config = toraniko_model::WlsConfig { winsor_factor: None, residualize_styles: true };
        let estimator = WlsFactorEstimator::with_config(config);
        let returns = random_returns(n_assets);
        let mkt_caps = random_market_caps(n_assets);
        let sectors = random_sector_matrix(n_assets, n_sectors);
        let styles = random_style_scores(n_assets, n_styles);

        b.iter(|| {
            estimator
                .estimate_single(
                    black_box(&returns),
                    black_box(&mkt_caps),
                    black_box(&sectors),
                    black_box(&styles),
                )
                .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_wls_estimator_single,
    bench_factor_estimation_scaling,
    bench_style_factor_scaling,
    bench_sector_scaling,
    bench_with_winsorization,
);

criterion_main!(benches);
