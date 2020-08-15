use num_traits::*;
use crate::ModPow;
use crate::packed::*;

//out-of-place transform
//input reference and perform in-place DFT on copy of input
pub fn transform2(mut a: Vec<u128>, P: &u128, rootTable: &Vec<u128>) -> Vec<u128> {
	//println!("radix2 transforming...{:?}", a.len());

	bit_reverse2(&mut a);
	DFT_radix2(&mut a, P, rootTable);

	a
}

pub fn transform3(mut a: Vec<u128>, P: &u128, rootTable: &Vec<u128>) -> Vec<u128> {
	//println!("radix3 transforming...{:?}", a.len());

	bit_reverse3(&mut a);
	DFT_radix3(&mut a, P, rootTable);

	a
}

pub fn inverse3(mut b: Vec<u128>, P: &u128, rootTable: &Vec<u128>) -> Vec<u128> {
	//println!("radix 3 inversing...{:?}", b.len());

	//calculating inverse omegas
	let L = b.len();
	let w = rootTable[1].modpow(&(P - 2u128), P);
	let mut inverseTable = vec![0u128; L];
	for i in 0..L {
		inverseTable[i] = w.modpow(&(i as u128), P);
	}

	bit_reverse3(&mut b);
	DFT_radix3(&mut b, P, &inverseTable);

	// F^-1(Y) = nX
	// Thus divide output by n or multiply n^-1
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	for i in 0..L {
		b[i] = &b[i] * &L_inverse % P;
	}

	b

}


pub fn inverse2(mut b: Vec<u128>, P: &u128, rootTable: &Vec<u128>) -> Vec<u128> {
	//println!("radix2 inversing...{:?}", b.len());
	let L = b.len();

	//calculating inverse omegas
	let w = rootTable[1].modpow(&(P - 2u128), P);
	let mut inverseTable = vec![0u128; L];
	for i in 0..L {
		inverseTable[i] = w.modpow(&(i as u128), P);
	}

	//clone input for in-place DFT
	bit_reverse2(&mut b);
	DFT_radix2(&mut b, P, &inverseTable);

	// F^-1(Y) = nX
	// Thus divide output by n or multiply n^-1
	let L_inverse = (L as u128).modpow(&(P - 2u128), P);
	for i in 0..L {
		b[i] = &b[i] * &L_inverse % P;
	}

	b
}

//in-place, use mutable reference
pub fn DFT_radix2(a: &mut Vec<u128>, P: &u128, rootTable: &Vec<u128>,){
	let L = a.len();
	let L_bitNum = (L as f64).log2().trunc() as u128;

    //Cooley-Tukey DFT
	for s in 1..(L_bitNum+1) as usize {
		let m = pow(2, s as usize);
		let mut i =0 as usize;
		while i < L {
			let mut j = 0;
			while j < m/2 {
				let t = &rootTable[j*(L/m as usize)] * &a[i + j + m/2 ] % P;
				let u = &a[i + j] % P;
				a[i + j] = (&u + &t) % P;
				if &u <= &t {
					a[i + j + m/2] = ((P + &u) - &t) % P;
				} else {
					a[i + j + m/2] = (&u - &t) % P;
				}
				j+= 1;
			}
			i += m;
		}
	}
}

pub fn DFT_radix3(a: &mut Vec<u128>, P: &u128, rootTable: &Vec<u128>) {	
	
	let L = a.len();
	let w = rootTable[L/3];
	let w_sqr = rootTable[L/3*2];

	let mut i = 1;
	while i < L {
		let jump = 3 * i;
		let stride = L/jump;
		for j in 0..i {
			let mut pair = j;
			while pair < L {
				let (x, y, z) = (a[pair],
								a[pair + i] * rootTable[j * stride] % P,
								a[pair + 2 * i] * rootTable[2 * j * stride] %P);
				a[pair] 	  	= (x + y + z) % P;
				a[pair + i]   	= (x % P + w * y % P + w_sqr * z % P) % P;
                a[pair + 2 * i] = (x % P + w_sqr * y % P + w * z % P) % P;
				
				pair += jump;
			}
		}
		i = jump;
	}
}

pub fn bit_reverse2(a: &mut Vec<u128>) {
	let L = a.len();
	let L_bitNum = (L as f64).log2().trunc() as u128;

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

pub fn bit_reverse3 (a: &mut Vec<u128>) {
    //radix3 bit reverse
    let mut t = 0usize;
    let L = a.len();
    let tri_L = trigits_len(L - 1);
    let mut trigits = vec![0; tri_L];
    for i in 0..L {
        if t > i {
            a.swap(t, i);
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
}









