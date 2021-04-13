use pss::*;

use rand::{thread_rng, Rng};
use criterion::{black_box, Bencher};
use criterion::{criterion_group, criterion_main, Criterion, Fun};


fn share_bench(bench: &mut Bencher, _i: &()) {

    let p = 4610415792919412737u128;
    let r2 = 1266473570726112470u128;
    let r3 = 2230453091198852918u128;

    let mut pss = PackedSecretSharing::<u128>::new(p, r2, r3, 
        512, 729, 51200, 512, 700);

	let mut rng = thread_rng();
    let mut secrets = vec![0u64; 51200];
    for i in 0..51200 {
        secrets[i] = rng.gen_range(0, u64::MAX);
    }

    bench.iter(|| black_box(
    	pss.share(black_box(&secrets))
    ));
}

fn criterion_benchmark(c: &mut Criterion) {
	c.bench_functions(
		"pss",
        vec![
            Fun::new("share", share_bench),
        ],
        (),
	);
}

criterion_group!{
	name = benches;
	config = Criterion::default().sample_size(50);
	targets = criterion_benchmark
}
criterion_main!(benches);