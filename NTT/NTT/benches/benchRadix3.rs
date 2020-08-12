use NTT::ModPow;
use num_traits::*;

pub fn trigits_len(n: usize) -> usize {
    let mut result = 1;
    let mut value = 3;
    while value < n + 1 {
        result += 1;
        value *= 3;
    }
    result
}


pub fn bench_inplace_bitreverse(data: &mut Vec<u128>) {
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
}


pub fn bench_inplace_DFT(a: &mut Vec<u128>, w_matrix: &Vec<u128>, P: &u128) {
	
	let L = a.len();
	let w = w_matrix[L/3];
	let w_sqr = w_matrix[L/3*2];
	let mut i = 1;
	while i < L {
		let jump = 3 * i;
		let stride = L/jump;
		for j in 0..i {
			let mut pair = j;
			while pair < L {
				let (x, y, z) = (a[pair],
								a[pair + i] * w_matrix[j * stride] % P,
								a[pair + 2 * i] * w_matrix[2 * j * stride] %P);

				a[pair] 	  	= (x + y + z) % P;
				a[pair + i]   	= (x % P + w * y % P + w_sqr * z % P) % P;
                a[pair + 2 * i] = (x % P + w_sqr * y % P + w * z % P) % P;
				
				pair += jump;
			}
		}
		i = jump;
	}
}









