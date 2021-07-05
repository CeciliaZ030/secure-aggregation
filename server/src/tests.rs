use polynomials::*;
use pss::*;
use pss::ModPow;
use crate::param::*;


pub fn lagrange_degree(poly: &Vec<u64>, param: &Param) -> bool {
	/*
		Can't use yet
		underlying lib does not support field operation
	*/
	let R3 = param.useR3 as u128;
	let P = param.P as u128;

	let mut xs = Vec::new();
	let mut ys = Vec::new();
	let mut products_inc: Vec<Polynomial<i128>> = Vec::new();
	let mut products_dec: Vec<Polynomial<i128>> = Vec::new();
	for i in 0..poly.len() {
		if poly[i] == 0 {
	    	continue;
		}
		println!("point1");
		let new = poly![-1*(poly[i] as i128), 1];
		if products_inc.is_empty() {
			products_inc.push(new);
		} else {
			let last = products_inc[products_inc.len()-1].clone();
			products_inc.push(last * new);
			println!("point2");
		}
		ys.push(R3.modpow((i+1) as u128, P) as i128);
		xs.push(poly[i].clone() as i128);
	}
	for i in poly.len()..0 {
		if poly[i] == 0 {
			continue;
		}
		let new = poly![-1*(poly[i] as i128), 1];
		if products_dec.is_empty() {
			products_dec.push(new);
		} else {
			println!("point3");
			let last = products_dec[products_dec.len()-1].clone();
			products_dec.push(last * new);
		}
	}
	products_dec.reverse();
	let mut construction = poly![0];
	let M = xs.len();
	for i in 0..M {	
		let numerator = products_inc[i-1].clone() * products_dec[i+1].clone();
		let mut denominator = 1i128;
		for j in 0..M {
			if j != i  { 
				denominator *= xs[j] - xs[i];
			}
		}
		construction += (numerator * ys[i] as i128 / denominator);
	}
	if construction.degree() == param.useD2 {
		return true;
	} else {
		return false;
	}
}

/*
shares of party i
	
	[ party j [t1, t2, t3],
	  party j [t1, t2, t3],
	 ...
	 ]

	 		[s00, s01, s02],	//shares of party 0
		    [s10, s11, s13],	//shares of party 1
		    ...
		    [sm0, sm1, sm3]		//shares of party m

*/
pub fn test_suit1(corrections: &Vec<Vec<u64>>, param: &Param, dropout: &mut Vec<usize>, pss: &PackedSecretSharing<u128>) -> bool {
	let M = corrections.len();
	let P = param.P as u128;
	let R3 = param.useR3 as u128;

	//let mut pss = PackedSecretSharing::new(
	//	P, param.useR2 as u128, R3, 
	//	param.useD2, param.useD3, 3*param.L, param.L, M
	//);

	let mut evalPoints = Vec::new();
	let mut corrections_remove_empty = Vec::new();
	for j in 0..M {
		if corrections[j].len() == 0 {
			dropout.push(j);
	    	continue;
		}
		evalPoints.push(R3.modpow((j+1) as u128, P) as u64);
		corrections_remove_empty.push(corrections[j].clone());
	}
	let result = pss.reconstruct(&corrections_remove_empty, evalPoints.as_slice());
	
	let mut sum = 0;
	for i in 2*param.L..3*param.L {
		sum = (sum + result[i]) % param.P;
	}
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

pub fn test_suit(corrections: &Vec<Vec<u64>>, param: &Param) -> bool {
	let M = corrections.len();
	let P = param.P as u128;
	let R3 = param.useR3 as u128;

	let mut pss = PackedSecretSharing::new(
		P, param.useR2 as u128, R3,
		param.useD2, param.useD3, 3*param.L, param.L, M
	);
	println!("EC reconstruct recvShares:{} > 2*d2:{:?}, d3:{}", M, param.useD2, param.useD3);
	let mut evalPoints = Vec::new();
	let mut corrections_remove_empty = Vec::new();
	for j in 0..M {
		if corrections[j].len() != 0 {
	    	evalPoints.push(R3.modpow((j+1) as u128, P) as u64);
			corrections_remove_empty.push(corrections[j].clone());
		}
	}
	println!("remove_empty {:?}, sharesPoints {}", corrections_remove_empty.len(), evalPoints.len());
	let result = pss.reconstruct2(&corrections_remove_empty, evalPoints.as_slice());
	/*
		Todo: check Degree test in result[0..param.L] 
			  degree has to be D2
	*/
	let mut sum = 0;
	for i in 2*param.L..3*param.L {
		sum = (sum + result[i]) % param.P;
	}
	println!("EC result (middle section should be 0s): {:?} \n sum (should be 0): {:?}", result, sum);
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
	println!("EC true");
	return true;
}
