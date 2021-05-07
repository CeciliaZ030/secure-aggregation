use pss::*;
use pss::ModPow;
use crate::param::*;

pub fn test_suit(corrections: &Vec<Vec<u64>>, param: &Param, pss: &PackedSecretSharing<u128>) -> bool {

	let M = corrections.len();
	let P = param.P as u128;
	let R3 = param.useR3 as u128;

	// let mut pss = PackedSecretSharing::new(
	// 	P, param.useR2 as u128, R3,
	// 	param.useD2, param.useD3, 3*param.L, param.L, M
	// );

	let mut evalPoints = Vec::new();
	let mut corrections_remove_empty = Vec::new();
	for j in 0..M {
		// if corrections[j].len() == 0 {
		// 	dropout.push(j);
	 //    	continue;
		// }
		evalPoints.push(R3.modpow((j+1) as u128, P) as u64);
		corrections_remove_empty.push(corrections[j].clone());
	}
	let result = pss.reconstruct(&corrections_remove_empty, evalPoints.as_slice());

	let mut sum = 0;
	for i in 2*param.L..3*param.L {
		sum = (sum + result[i]) % param.P;
	}
//	println!("EC result (middle section should be 0s): {:?} \n sum (should be 0): {:?}", result, sum);
	/*
		Input Bit Test, Quadratic test, L2-norm bit test
		secrets should be 0
	*/
	for i in param.L..2*param.L {
		if result[i] != 0 {
			return false;
		}
	}
	/*
		Input Bound Test, L2-norm sum test, L2-norm bound test
		secrets sums to 0
	*/
	if sum != 0 {
		return false;
	}
	return true;
}
