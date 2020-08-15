use packed_secret_sharing::*;
use packed_secret_sharing::packed::*;

use rand::{thread_rng, Rng};
use criterion::{black_box, Bencher};

pub fn share_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
    let root2 = 1266473570726112470u128;
    let root3 = 2230453091198852918u128;
 	
 	let mut rng = thread_rng();
 	let degree = 511;
    let num_secrets = rng.gen_range(1, 512);
    let num_shares = rng.gen_range(512, 729);
    let mut pss = PackedSecretSharing::new(prime, root2, root3, degree, num_secrets, num_shares);

    let mut secrets = vec![0u128; num_secrets];
	for i in 0..num_secrets {
		secrets[i] = rng.gen_range(0, &prime);
	}

	bench.iter(|| black_box(
    	pss.share(black_box(&secrets))
    ));
}

pub fn reconstruct_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
    let root2 = 1266473570726112470u128;
    let root3 = 2230453091198852918u128;
 	
 	let mut rng = thread_rng();
 	let degree = 511;
    let num_secrets = 10; //rng.gen_range(1, 512);
    let num_shares = rng.gen_range(512, 729);
    let mut pss = PackedSecretSharing::new(prime, root2, root3, degree, num_secrets, num_shares);

    let mut secrets = vec![0u128; num_secrets];
	for i in 0..num_secrets {
		secrets[i] = rng.gen_range(0, &prime);
	}
	let shares = pss.share(&secrets);

// ======================================================

    let mut shares_point = pss.rootTable3.clone();
    shares_point.split_off(512);

    let mut shares_val = shares.clone();
    shares_val.split_off(512);

	bench.iter(|| black_box(
    	pss.reconstruct(black_box(&shares_point), black_box(&shares_val))
    ));
}

pub fn lagrange_interpolation_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let degree = 511;
	let num_roots = 100;

    let mut shares_point = vec![0u128; degree + 1];
    let mut shares_val = vec![0u128; degree + 1];
    let mut rng = thread_rng();
	for i in 0..degree + 1 {
		shares_point[i] = rng.gen_range(0, &prime);
		shares_val[i] = rng.gen_range(0, &prime);
	}

	let mut root = vec![0u128; num_roots];
	for i in 0..num_roots {
		root[i] = rng.gen_range(0, &prime);
	}

	bench.iter(|| black_box(
    	lagrange_interpolation(black_box(&shares_point), black_box(&shares_val), 
    							black_box(&root), black_box(&prime))
    ));
}