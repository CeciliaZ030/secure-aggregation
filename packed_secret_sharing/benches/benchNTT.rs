use packed_secret_sharing::*;
use packed_secret_sharing::ntt::*;

use rand::{thread_rng, Rng};
use criterion::{black_box, Bencher};


pub fn radix2_FFT_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let root = 1266473570726112470u128;
	let mut a = vec![0u128; 512];
	let mut rootTable = vec![0u128; 512];

	let mut rng = thread_rng();
	for i in 0..512 {
		a[i] = rng.gen_range(0, &prime);
		rootTable[i] = root.modpow(&(i as u128), &prime);
	}

    bench.iter(|| black_box(
    	DFT_radix2(black_box(&mut a), black_box(&prime), black_box(&rootTable))
    ));
}

pub fn radix2_bit_reverse_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let mut a = vec![0u128; 512];
	let mut rng = thread_rng();
	for i in 0..512 {
		a[i] = rng.gen_range(0, &prime);
	}

    bench.iter(|| black_box(
    	bit_reverse2(black_box(&mut a))
    ));
}

pub fn radix3_FFT_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let root = 2230453091198852918u128;
	let mut a = vec![0u128; 729];
	let mut rootTable = vec![0u128; 729];

	let mut rng = thread_rng();
	for i in 0..729 {
		a[i] = rng.gen_range(0, &prime);
		rootTable[i] = root.modpow(&(i as u128), &prime);
	}

    bench.iter(|| black_box(
    	DFT_radix3(black_box(&mut a), black_box(&prime), black_box(&rootTable))
    ));
}

pub fn radix3_bit_reverse_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let mut a = vec![0u128; 729];
	let mut rng = thread_rng();
	for i in 0..729 {
		a[i] = rng.gen_range(0, &prime);
	}

    bench.iter(|| black_box(
    	bit_reverse3(black_box(&mut a))
    ));
}

pub fn radix2_inverse_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let root = 1266473570726112470u128;
	let mut a = vec![0u128; 512];
	let mut rootTable = vec![0u128; 512];

	let mut rng = thread_rng();
	for i in 0..512 {
		a[i] = rng.gen_range(0, &prime);
		rootTable[i] = root.modpow(&(i as u128), &prime);
	}

    bench.iter(|| black_box(
    	inverse2(black_box(a.clone()), black_box(&prime), black_box(&rootTable))
    ));
}

pub fn radix3_inverse_bench(bench: &mut Bencher, _i: &()) {

	let prime = 4610415792919412737u128;
	let root = 1266473570726112470u128;
	let mut a = vec![0u128; 729];
	let mut rootTable = vec![0u128; 729];

	let mut rng = thread_rng();
	for i in 0..729 {
		a[i] = rng.gen_range(0, &prime);
		rootTable[i] = root.modpow(&(i as u128), &prime);
	}

    bench.iter(|| black_box(
    	inverse3(black_box(a.clone()), black_box(&prime), black_box(&rootTable))
    ));
}