use u64_impl::ModPow;
use num_traits::*;

pub fn bench_modpow(a: &Vec<u128>, P: &u128, r: &u128) -> Vec<u128>{
	let L = a.len();
	let mut w_matrix: Vec<u128> = Vec::new();
	for i in 0..L {
		w_matrix.push(r.modpow(&(i as u128), P));	
	}
	w_matrix
}


pub fn bench_out_of_place_bitreverse(a: &Vec<u128>) {
	let L = a.len();
	let L_bitNum = ((L as f64).log2().trunc() as u32);

	let mut b = Vec::<u128>::new();
	for i in 0..L {
        let mut i_rev = 0;
        for j in 0..L_bitNum {
            if (i & (1 << j)) > 0 {
                i_rev |= 1 << ((L_bitNum - 1) - j);   
            }
        }
		b.push(a[i_rev].clone());
	}
}

pub fn bench_inplace_bitreverse(a: &mut Vec<u128>) {
	let L = a.len();
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
}

pub fn bench_inplace_DFT(b: &mut Vec<u128>, w_matrix: &Vec<u128>, p: &u128) {

	let L = b.len();
	let L_bitNum = ((L as f64).log2().trunc() as u32);

	for s in 1..(L_bitNum+1) as usize {
		let m = pow(2,s as usize);
		let mut i =0 as usize;
		while i< L {
			let mut j = 0;
			while j < m/2 {

				let t = &w_matrix[j*(L/m as usize)] * &b[i + j + m/2 ] % p;
				let u = &b[i + j] % p;
				
				b[i + j] = (&u + &t) % p;

				if &u <= &t {
					b[i + j + m/2] = ((p + &u) - &t) % p;
				} else {
					b[i + j + m/2] = (&u - &t) % p;
				}

				j+= 1;
			}
			i += m;
		}
	}
}

pub fn bench_inplace_DFT_different_loop(b: &mut Vec<u128>, w_matrix: &Vec<u128>, p: &u128) {

	let n = b.len();

	let mut k = 0;
    let mut step = 1;
    while step < n {
        let jump = step << 1;
        for mut i in 0..step {
            while i < n {
                let j = i + step;
                unsafe {
                    let t = &w_matrix[k] * &b[j] % p;
                    let u = &b[i] % p;
                    if &u <= &t {
						b[j] = ((p + &u) - &t) % p;
					} else {
						b[j] = (&u - &t) % p;
					}
                    b[i] = (&u + &t) % p;
                }

                i += jump;
            }
            k += 1;
        }
        step <<= 1;
    }
}

pub fn bench_vector_mul_forloop(a: &mut Vec<u128>, P: &u128) {
	let L = a.len();
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	for i in 0..L {
		a[i] = &a[i] * &L_inverse % P;
	}
}

pub fn bench_vector_mul_iter(a: &mut Vec<u128>, P: &u128) {
	let L = a.len();
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	a.iter_mut().for_each(|ai| *ai = ai.clone() * &L_inverse % P)
}

