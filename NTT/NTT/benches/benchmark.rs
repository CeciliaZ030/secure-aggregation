use criterion::*;
use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use std::path::Path;
use rand::{thread_rng, Rng};

use NTT::*;

mod benchBigUint;
mod benchU128;
mod benchRadix3;
mod benchU64;



fn bench_ntt_biguint(c: &mut Criterion) {

    let path1 = Path::new("sample1.txt");
    let path2 = Path::new("sample2.txt");

	let P1 = 4611686018326724609u64.into_biguint().unwrap();
	let R1 = 1468970003788274264u64.into_biguint().unwrap();
	let X1 = read_input_to_BigUint(&path1).unwrap();

	let P2 = 4611686018326724609u64.into_biguint().unwrap();
	let R2 = 3125141995714774395u64.into_biguint().unwrap();
	let X2 = read_input_to_BigUint(&path2).unwrap();

	let mut group = c.benchmark_group("NTT BigUint");


    group.bench_function("[ Modpow 1 ]", |bench| {
        bench.iter(|| {
            benchBigUint::bench_modpow(black_box(&X1), black_box(&P1), black_box(&R1));
        })
    });
    group.bench_function("[ Modpow 2 ]", |bench| {
        bench.iter(|| {
            benchBigUint::bench_modpow(black_box(&X2), black_box(&P2), black_box(&R2));
        })
    });


    group.bench_function("[ Out of Place Bitreverse 1 ]", |bench| {
        bench.iter(|| {
            benchBigUint::bench_out_of_place_bitreverse(black_box(&X1));
        })
    });
    group.bench_function("[ Out of Place Bitreverse 2 ]", |bench| {
        bench.iter(|| {
            benchBigUint::bench_out_of_place_bitreverse(black_box(&X2));
        })
    });

    group.bench_function("[ Inplace Bitreverse 1 ]", |bench| {
    	let mut _X = X1.clone();
        bench.iter(|| {
            benchBigUint::bench_inplace_bitreverse(black_box(&mut _X));
        })
    });
    group.bench_function("[ Inplace Bitreverse 2 ]", |bench| {
    	let mut _X = X2.clone();
        bench.iter(|| {
            benchBigUint::bench_inplace_bitreverse(black_box(&mut _X));
        })
    });

    group.bench_function("[ Inplace DFT 1 ]", |bench| {
    	let mut _X = X1.clone(); 
    	let w_matrix = benchBigUint::bench_modpow(&X1, &P1, &R1);
        bench.iter(|| {
            benchBigUint::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P1));
        })
    });
    group.bench_function("[ Inplace DFT 2 ]", |bench| {
    	let mut _X = X2.clone();
    	let w_matrix = benchBigUint::bench_modpow(&X2, &P2, &R2);
        bench.iter(|| {
            benchBigUint::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P2));
        })
    });

    group.bench_function("[ Inplace DFT Different Loop 1 ]", |bench| {
    let mut _X = X1.clone(); 
    let w_matrix = benchBigUint::bench_modpow(&X1, &P1, &R1);
    bench.iter(|| {
        benchBigUint::bench_inplace_DFT_different_loop(black_box(&mut _X), black_box(&w_matrix), black_box(&P1));
    })
    });
    group.bench_function("[ Inplace DFT Different Loop 2 ]", |bench| {
        let mut _X = X2.clone();
        let w_matrix = benchBigUint::bench_modpow(&X2, &P2, &R2);
        bench.iter(|| {
            benchBigUint::bench_inplace_DFT_different_loop(black_box(&mut _X), black_box(&w_matrix), black_box(&P2));
        })
    });


    group.bench_function("[ Vector Multiplication with Forloop 1 ]", |bench| {
        let mut _X = X1.clone();
        bench.iter(|| {
            benchBigUint::bench_vector_mul_forloop(black_box(&mut _X), black_box(&P1));
        })
    });
    group.bench_function("[ Vector Multiplication with Forloop 2 ]", |bench| {
        let mut _X = X2.clone();
        bench.iter(|| {
            benchBigUint::bench_vector_mul_forloop(black_box(&mut _X), black_box(&P2));
        })
    });

    group.bench_function("[ Vector Multiplication with Iterator 1 ]", |bench| {
        let mut _X = X1.clone();
        bench.iter(|| {
            benchBigUint::bench_vector_mul_iter(black_box(&mut _X), black_box(&P1));
        })
    });
    group.bench_function("[ Vector Multiplication with Iterator 2 ]", |bench| {
        let mut _X = X2.clone();
        bench.iter(|| {
            benchBigUint::bench_vector_mul_iter(black_box(&mut _X), black_box(&P2));
        })
    });
}

fn bench_radix3(c: &mut Criterion) {

    let mut group = c.benchmark_group("Radix 3");
    /*
    use this prime : 4610415792919412737

    512th root ot unity: 1266473570726112470

    729th root of unity: 2230453091198852918
        Sample
        [1, 2, 3, 4, 5, 6, 7, 8, 9]
        [45, 404, 407, 266, 377, 47, 158, 17, 20]
    */

    let P = 4610415792919412737u128;
    let r = 2230453091198852918u128;

    let mut rng = thread_rng();
    let mut a: Vec<u128> = Vec::new();
    for i in 0..729 {
        a.push(rng.gen_range(0, P));
    }

    let L = a.len();

    let mut w_matrix: Vec<u128> = Vec::new();
    for i in 0..L/3*2+1 {
        w_matrix.push(r.modpow(&(i as u128), &P));  
    }

    group.bench_function("[ Inplace Bitreverse]", |bench| {
        let mut _X = a.clone();
        bench.iter(|| {
            benchRadix3::bench_inplace_bitreverse(black_box(&mut _X));
        })
    });

    group.bench_function("[ Inplace DFT]", |bench| {
        let mut _X = a.clone(); 
        bench.iter(|| {
            benchRadix3::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P));
        })
    });
}

fn bench_u64(c: &mut Criterion) {
    let mut group = c.benchmark_group("U64");

    let path1 = Path::new("sample1.txt");

    let P1_ = 4611686018326724609u64;
    let R1_ = 1468970003788274264u64;
    let X1_ = read_input_to_u64(&path1).unwrap();

    let L = X1_.len();

    //calculating omegas
    let w = R1_;
    let mut w_matrix: Vec<u64> = Vec::new();
    for i in 0..L {

        w_matrix.push(w.modpow(&(i as u64), &P1_));    
    }

    group.bench_function("[ Inplace DFT]", |bench| {
        let mut _X = X1_.clone(); 
        bench.iter(|| {
            benchU64::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P1_));
        })
    });
}

criterion_group!{
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().significance_level(0.1).sample_size(100);
    targets = bench_u64
}
criterion_main!(benches);



