use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use u64_impl::bigUint::*;
use u64_impl::*;

fn main() {

	println!("hello world");

	let X2 = read_input_to_BigUint("sample2.txt").unwrap();
	let P2 = 4611686018326724609u64.into_biguint().unwrap();
	let R2 = 3125141995714774395u64.into_biguint().unwrap();

	let X1 = read_input_to_BigUint("sample1.txt").unwrap();
	let P1 = 4611686018326724609u64.into_biguint().unwrap();
	let R1 = 1468970003788274264u64.into_biguint().unwrap();

	assert_eq!(inverse(&transform(&X1, &P1, &R1), &P1, &R1), X1);
	assert_eq!(inverse(&transform(&X2, &P2, &R2), &P2, &R2), X2);

}



