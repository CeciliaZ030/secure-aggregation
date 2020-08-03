extern crate num_bigint_dig;
extern crate num_traits;

use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use num_traits::*;

//out-of-place transform
//input reference and perform in-place DFT on copy of input
pub fn transform(a: &Vec<BigUint>, P: &BigUint, r: &BigUint) -> Vec<BigUint> {
	println!("transforming...");
	let L = a.len();

	//calculating omegas
	let w = r;
	let mut w_matrix: Vec<BigUint> = Vec::new();
	for i in 0..L {
		w_matrix.push(w.modpow(&(i).into_biguint().unwrap(), P));	
	}

	//clone input for in-place DFT
	let mut a_ret = a.clone();
	DFT(&mut a_ret, &w_matrix, P);

	a_ret
}

pub fn inverse(b: &Vec<BigUint>, P: &BigUint, r: &BigUint) -> Vec<BigUint> {
	println!("inversing...");
	let L = b.len();

	//calculating omegas
	let w = r.modpow(&(P - 2u32), P);
	let mut w_matrix: Vec<BigUint> = Vec::new();
	for i in 0..L {
		w_matrix.push(w.modpow(&(i).into_biguint().unwrap(), P));
	}

	//clone input for in-place DFT
	let mut b_ret = b.clone();
	DFT(&mut b_ret, &w_matrix, P);

	// F^-1(Y) = nX
	// Thus divide output by n or multiply n^-1
	let L_inverse = L.into_biguint().unwrap().modpow(&(P - 2u32), P);
	for i in 0..L {
		b_ret[i] = &b_ret[i] * &L_inverse % P
	}

	b_ret
}

//in-place, use mutable reference
fn DFT(a: &mut Vec<BigUint>, w_matrix: &Vec<BigUint>, p: &BigUint){
	
	let L = a.len();
	let L_bitNum = (L as f64).log2().trunc() as u32;

	//Bit reversed permutation
    let mut j = 0;
    for i in 0..L {
        if j > i {
            a.swap(i, j);
        }
        let mut mask = L >> 1;
        while j & mask != 0 {
            j &= !mask;
            mask >>= 1;
        }
        j |= mask;
    }

    //Cooley-Tukey DFT
	for s in 1..(L_bitNum+1) as usize {
		let m= pow(2, s as usize);
		let mut i =0 as usize;
		while i < L {
			let mut j = 0;
			while j < m/2 {
				let t = &w_matrix[j*(L/m as usize)] * &a[i + j + m/2 ] % p;
				let u = &a[i + j] % p;
				a[i + j] = (&u + &t) % p;
				if &u <= &t {
					a[i + j + m/2] = ((p + &u) - &t) % p;
				} else {
					a[i + j + m/2] = (&u - &t) % p;
				}
				j+= 1;
			}
			i += m;
		}
	}
}