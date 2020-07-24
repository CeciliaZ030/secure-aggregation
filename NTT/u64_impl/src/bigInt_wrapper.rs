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
	let mut w_matrix: Vec<BigUint> = Vec::new();
	for i in 0..L {
		//row.push(w.modpow(&(i * j).into_biguint().unwrap(), &P));
		w_matrix.push(w.modpow(&(i).into_biguint().unwrap(), &P));	
	}

	DFT(a, w_matrix, P)

}

fn DFT(a: &Vec<BigUint>, w_matrix: Vec<BigUint>, p: BigUint) -> Vec<BigUint> {
	
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
		//println!("{:}, {}", i, i_rev);
		b.push(a[i_rev].clone());
	}

	for s in 1..(L_bitNum+1) as usize {
		let m= pow(2,s as usize);
		let mut i =0 as usize;

		//println!("m={} L= {} , L/m= {}, w_matrix[1][L/m as usize]={}",m,L,L/m,w_matrix[1][L/m as usize]);
		while i< L {
			let mut j = 0;
			while j < m/2 {
				let t = &w_matrix[j*(L/m as usize)] * &b[i + j + m/2 ] % &p;
				let u = &b[i + j] % &p;
				
				b[i + j] = (&u + &t) % &p;
				if &u <= &t {
					b[i + j + m/2] = ((&p+&u) - &t) % &p;
				} else {
					b[i + j + m/2] = (&u - &t) % &p;
				}
				//println!("{:?}", &b);
				j+= 1;

			}
			i += m;
		}
		println!("{},{}", i, s);
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