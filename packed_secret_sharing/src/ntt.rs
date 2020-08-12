use num_traits::*;
use crate::ModPow;

//out-of-place transform
//input reference and perform in-place DFT on copy of input
pub fn transform2(mut a: Vec<u128>, P: &u128, r: &u128) -> Vec<u128> {
	println!("radix2 transforming...{:?}", a.len());
	let L = a.len();

	//calculating omegas
	let w = r;
	let mut w_matrix: Vec<u128> = Vec::new();
	for i in 0..L {
		w_matrix.push(w.modpow(&(i as u128), P));	
	}

	DFT(&mut a, &w_matrix, P);
	a
}

pub fn transform3(mut a: Vec<u128>, P: &u128, r: &u128) -> Vec<u128> {
	println!("radix3 transforming...{:?}", a.len());

	DFT_radix3(&mut a, &P, &r);
	a
}

pub fn inverse3(mut b: Vec<u128>, P: &u128, r: &u128) -> Vec<u128> {
	println!("radix 3 inversing...{:?}", b.len());

	//calculating omegas
	let L = b.len();
	let w = r.modpow(&(P - 2u128), P);

	//clone input for in-place DFT
	DFT_radix3(&mut b, &P, &w);

	// F^-1(Y) = nX
	// Thus divide output by n or multiply n^-1
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	for i in 0..L {
		b[i] = &b[i] * &L_inverse % P;
	}

	b

}


pub fn inverse2(mut b: Vec<u128>, P: &u128, r: &u128) -> Vec<u128> {
	println!("radix2 inversing...{:?}", b.len());
	let L = b.len();

	//calculating omegas
	let w = r.modpow(&(P - 2u128), P);
	let mut w_matrix: Vec<u128> = Vec::new();
	for i in 0..L {
		w_matrix.push(w.modpow(&(i as u128), P));
	}

	//clone input for in-place DFT
	DFT(&mut b, &w_matrix, P);

	// F^-1(Y) = nX
	// Thus divide output by n or multiply n^-1
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	for i in 0..L {
		b[i] = &b[i] * &L_inverse % P;
	}

	b
}

//in-place, use mutable reference
fn DFT(a: &mut Vec<u128>, w_matrix: &Vec<u128>, p: &u128){
	
	let L = a.len();
	let L_bitNum = (L as f64).log2().trunc() as u128;

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
		let m = pow(2, s as usize);
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

fn trigits_len(n: usize) -> usize {
    let mut result = 1;
    let mut value = 3;
    while value < n + 1 {
        result += 1;
        value *= 3;
    }
    result
}


pub fn DFT_radix3(data: &mut Vec<u128>, P: &u128, r: &u128){
    
    //radix3 bit reverse
    let mut t = 0usize;
    let L = data.len();
    let tri_L = trigits_len(L - 1);
    let mut trigits = vec![0; tri_L];

    for i in 0..L {
        if t > i {
            data.swap(t, i);
        }
        for j in 0..tri_L {
            if trigits[j] < 2 {
                trigits[j] += 1;
                t += 3usize.pow((tri_L-j-1)as u32);
                break;
            } else {
                trigits[j] = 0;
                t -= 2 * 3usize.pow((tri_L-j-1)as u32);
            }
        }
    }
    println!("{:?}", data);

    //radix3 DFT
    let mut step = 1;
    let w = r;
    let tri_w = r.modpow(&((L/3) as u128), P);
    let tri_w_sq = tri_w * tri_w % P;
	println!("{:?}, {}",tri_w, tri_w_sq);

    while step < L {
        let jump = 3 * step;
        let factor_stride = w.modpow(&((L/step/3) as u128), P);
        println!("{} s {}", step, factor_stride);

        let mut factor = 1;
        for group in 0usize..step {
            let factor_sq = factor * factor % P;
            println!("	factor {:?}, {}", factor, factor_sq);

            let mut pair = group;
            while pair < data.len() {
                let (x, y, z) = (data[pair],
                                 data[pair + step] * factor % P,
                                 data[pair + 2 * step] * factor_sq % P);

                data[pair] = (x + y + z) % P;
                data[pair + step] = (x % P + tri_w * y % P+ tri_w_sq * z % P) % P;
                data[pair + 2 * step] = (x % P + tri_w_sq * y % P+ tri_w * z % P) % P;
                pair += jump;
            }
            factor = factor * factor_stride % P;
        }
        step = jump;
    }
}










