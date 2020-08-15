mod benchNTT;
mod benchPacked;

use crate::{benchNTT::*, benchPacked::*, };
use criterion::{criterion_group, criterion_main, Criterion, Fun};

fn criterion_benchmark(c: &mut Criterion) {
    // c.bench_functions(
    //     "Radix-2 NTT",
    //     vec![
    //         Fun::new("radix-2 DFT", radix2_FFT_bench),
    //         Fun::new("radix-2 bit reverse", radix2_bit_reverse_bench),
    //         Fun::new("inverse", radix2_inverse_bench),
    //     ],
    //     (),
    // );
    // c.bench_functions(
    //     "Radix-3 NTT",
    //     vec![
    //         Fun::new("radix-3 DFT", radix3_FFT_bench),
    //         Fun::new("radix-3 bit reverse", radix3_bit_reverse_bench),
    //         Fun::new("inverse", radix3_inverse_bench),
    //     ],
    //     (),
    // );
    c.bench_functions(
        "Lagrange Interpolation",
        vec![
            Fun::new("lagrange interpolation", lagrange_interpolation_bench),
        ],
        (),
    );
    // c.bench_functions(
    //     "Packed Secret Sharing",
    //     vec![
    //         Fun::new("sharing", share_bench),
    //         Fun::new("reconstructing", reconstruct_bench),
    //     ],
    //     (),
    // );

}

// TODO: Powers

criterion_group!{
	name = benches;
	config = Criterion::default().sample_size(50);
	targets = criterion_benchmark
}
criterion_main!(benches);
