use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use u64_impl::*;
use std::path::Path;


fn main() {

	println!("hello world");

	let path1 = Path::new("sample1.txt");
	let path2 = Path::new("sample2.txt");

	let X2 = read_input_to_BigUint(&path2).unwrap();
	let P2 = 4611686018326724609u64.into_biguint().unwrap();
	let R2 = 3125141995714774395u64.into_biguint().unwrap();

	let X1 = read_input_to_BigUint(&path1).unwrap();
	let P1 = 4611686018326724609u64.into_biguint().unwrap();
	let R1 = 1468970003788274264u64.into_biguint().unwrap();

	let P1_ = 4611686018326724609u128;
    let R1_ = 1468970003788274264u128;
    let X1_ = read_input_to_u128(&path1).unwrap();

    let P2_ = 4611686018326724609u128;
    let R2_ = 3125141995714774395u128;
    let X2_ = read_input_to_u128(&path2).unwrap();

	assert_eq!(bigUint::inverse(&bigUint::transform(&X1, &P1, &R1), &P1, &R1), X1);
	assert_eq!(bigUint::inverse(&bigUint::transform(&X2, &P2, &R2), &P2, &R2), X2);

	assert_eq!(U128::inverse(&U128::transform(&X1_, &P1_, &R1_), &P1_, &R1_), X1_);
	assert_eq!(U128::inverse(&U128::transform(&X2_, &P2_, &R2_), &P2_, &R2_), X2_);
}
