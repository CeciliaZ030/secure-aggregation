extern crate num_bigint_dig;
extern crate num_traits;

use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use num_traits::*;


pub fn transform(a: &Vec<BigUint>) -> Vec<BigUint> {

	let L = a.len();

	let (P, k) = find_prime(L);
	let r = primative_root(&P);

	let w = r;
	println!("primitive root: {:?}, p=nL+1---n: {},p: {}, omega: {}", w, k, P, w);
	let mut w_matrix: Vec<Vec<BigUint>> = Vec::new();
	for i in 0..L {
		let mut row: Vec<BigUint> = Vec::new();
		for j in 0..L {
			row.push(w.modpow(&(i * j).into_biguint().unwrap(), &P));
		}
		w_matrix.push(row);	
	}
	println!("w_matrix: {}", w_matrix[0][0]);

	DFT(a, w_matrix, P)

}

fn DFT(a: &Vec<BigUint>, w_matrix: Vec<Vec<BigUint>>, p: BigUint) -> Vec<BigUint> {
	
	let L = a.len();
	let L_bitNum = ((L as f64).log2().trunc() as u32);

	let mut b = Vec::<BigUint>::new();
	for i in 0..L {
        let mut i_rev = 0;
        for j in 0..L_bitNum {
            if (i & (1 << j)) > 0 {
                i_rev |= 1 << ((L_bitNum - 1) - j);   
            }
        }
		//println!("{:?}, {}", i, i_rev);
		b.push(a[i_rev].clone());
	}

	let mut s = 1;
	while s < L {

		let (mut i, mut j) = (0, 0);

		while i < L {
			while j < s/2 {

				let t = &w_matrix[i][j] * &b[i + j + s/2] % &p;
				let u = &b[i + j] % &p;
				b[i + j] = (&u + &t) % &p;
				if &u <= &t {
					b[i + j + s/2] = (&t - &u) % &p;
				} else {
					b[i + j + s/2] = (&u - &t) % &p;
				}
				//println!("{:?}", &b);
				j+= 1;
			}
			i += s;
		}
		println!("{},{},{}", i, j, s);
		s <<= 1;
	}
	//println!("{}", b);
	b
}

fn find_prime(L: usize) -> (BigUint, BigUint) {
	let P: BigUint = 4611686018326724609u64.into_biguint().unwrap();
	let k: BigUint = (&P - (1 as u8))/L;
	(P, k)
	//(11,2)
}

fn primative_root(p: &BigUint) -> BigUint {
	1468970003788274264u64.into_biguint().unwrap()
	//6
}