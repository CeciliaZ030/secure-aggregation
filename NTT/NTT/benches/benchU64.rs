use NTT::ModPow;
use num_traits::*;

pub fn bench_inplace_DFT(a: &mut Vec<u64>, w_matrix: &Vec<u64>, p: &u64) {

	let L = a.len();
	let L_bitNum = ((L as f64).log2().trunc() as u32);

    //Cooley-Tukey DFT
	for s in 1..(L_bitNum+1) as usize {
		let m = pow(2, s as usize);
		let mut i =0 as usize;
		while i < L {
			let mut j = 0;
			while j < m/2 {
				let castup = (w_matrix[j * (L/m as usize)] as u128) * (a[i + j + m/2 ] as u128) % *p as u128; 
				let t = castup as u64;
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
