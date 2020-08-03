use criterion::*;
use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use std::path::Path;

use u64_impl::*;

mod benchBigUint;
mod benchU128;



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

fn bench_ntt_u128(c: &mut Criterion) {

    let path1 = Path::new("sample1.txt");
    let path2 = Path::new("sample2.txt");

    let P1 = 4611686018326724609u128;
    let R1 = 1468970003788274264u128;
    let X1 = read_input_to_u128(&path1).unwrap();

    let P2 = 4611686018326724609u128;
    let R2 = 3125141995714774395u128;
    let X2 = read_input_to_u128(&path2).unwrap();

    let mut group = c.benchmark_group("NTT U128");


    group.bench_function("[ Modpow 1 ]", |bench| {
        bench.iter(|| {
            benchU128::bench_modpow(black_box(&X1), black_box(&P1), black_box(&R1));
        })
    });
        group.bench_function("[ Modpow 2 ]", |bench| {
        bench.iter(|| {
            benchU128::bench_modpow(black_box(&X2), black_box(&P2), black_box(&R2));
        })
    });


    group.bench_function("[ Out of Place Bitreverse 1 ]", |bench| {
        bench.iter(|| {
            benchU128::bench_out_of_place_bitreverse(black_box(&X1));
        })
    });
    group.bench_function("[ Out of Place Bitreverse 2 ]", |bench| {
        bench.iter(|| {
            benchU128::bench_out_of_place_bitreverse(black_box(&X2));
        })
    });

    group.bench_function("[ Inplace Bitreverse 1 ]", |bench| {
        let mut _X = X1.clone();
        bench.iter(|| {
            benchU128::bench_inplace_bitreverse(black_box(&mut _X));
        })
    });
    group.bench_function("[ Inplace Bitreverse 2 ]", |bench| {
        let mut _X = X2.clone();
        bench.iter(|| {
            benchU128::bench_inplace_bitreverse(black_box(&mut _X));
        })
    });

    group.bench_function("[ Inplace DFT 1 ]", |bench| {
        let mut _X = X1.clone(); 
        let w_matrix = benchU128::bench_modpow(&X1, &P1, &R1);
        bench.iter(|| {
            benchU128::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P1));
        })
    });
    group.bench_function("[ Inplace DFT 2 ]", |bench| {
        let mut _X = X2.clone();
        let w_matrix = benchU128::bench_modpow(&X2, &P2, &R2);
        bench.iter(|| {
            benchU128::bench_inplace_DFT(black_box(&mut _X), black_box(&w_matrix), black_box(&P2));
        })
    });

    group.bench_function("[ Inplace DFT Different Loop 1 ]", |bench| {
    let mut _X = X1.clone(); 
    let w_matrix = benchU128::bench_modpow(&X1, &P1, &R1);
    bench.iter(|| {
        benchU128::bench_inplace_DFT_different_loop(black_box(&mut _X), black_box(&w_matrix), black_box(&P1));
    })
    });
    group.bench_function("[ Inplace DFT Different Loop 2 ]", |bench| {
        let mut _X = X2.clone();
        let w_matrix = benchU128::bench_modpow(&X2, &P2, &R2);
        bench.iter(|| {
            benchU128::bench_inplace_DFT_different_loop(black_box(&mut _X), black_box(&w_matrix), black_box(&P2));
        })
    });


    group.bench_function("[ Vector Multiplication with Forloop 1 ]", |bench| {
        let mut _X = X1.clone();
        bench.iter(|| {
            benchU128::bench_vector_mul_forloop(black_box(&mut _X), black_box(&P1));
        })
    });
    group.bench_function("[ Vector Multiplication with Forloop 2 ]", |bench| {
        let mut _X = X2.clone();
        bench.iter(|| {
            benchU128::bench_vector_mul_forloop(black_box(&mut _X), black_box(&P2));
        })
    });

    group.bench_function("[ Vector Multiplication with Iterator 1 ]", |bench| {
        let mut _X = X1.clone();
        bench.iter(|| {
            benchU128::bench_vector_mul_iter(black_box(&mut _X), black_box(&P1));
        })
    });
    group.bench_function("[ Vector Multiplication with Iterator 2 ]", |bench| {
        let mut _X = X2.clone();
        bench.iter(|| {
            benchU128::bench_vector_mul_iter(black_box(&mut _X), black_box(&P2));
        })
    });
}

criterion_group!{
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().significance_level(0.1).sample_size(100);
    targets = bench_ntt_u128
}
criterion_main!(benches);



