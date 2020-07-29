fn DFT_u128(a: &Vec<u128>, w_matrix: Vec<Vec<u128>>, p: u128) -> Vec<u128> {
	
	let L = a.len();
	let L_bitNum: u128 = ((L as f64).log2().trunc() as u128) + 1;

	let mut b = Vec::<u128>::new();
	for i in 0..L {
		let mut i_rev = 0;
		for j in 0..L_bitNum {
			if (i & (1 << j)) > 0 {
				i_rev |= 1 << ((L_bitNum - 1) - j);   
			}
		}
		println!("{:?}, {}", i, i_rev);
		b.push(a[i_rev].clone());
		println!("new input {:?}", b[i]);
	}

	let mut s = 1;
	while s < L {

		let (mut i, mut j) = (0, 0);

		while i < L {
			while j < s/2 {

				let t = w_matrix[i][j] * b[i + j + s/2] % p;
				let u = b[i + j] % p;
				b[i + j] = (u + t) % p;
				b[i + j + s/2] = (u - t) % p;
				println!("{:?}", b);
				j+= 1;
			}
			i += s;
		}
		println!("{},{},{}", i, j, s);
		s <<= 1;
	}
	println!("{:?}", b);
	b
}


fn find_prime(L: u128) -> (u128, u128) {
	let P: u128 = 4611686018326724609;
	let k: u128 = (P - 1)/L;
	println!("{:?}", k);
	(P, k)
	//(11,2)
}

fn primative_root(p: u128) -> u128 {
	1468970003788274264
	//6
}

pub fn transform(a: &Vec<u128>) -> Vec<u128> {

	let L: u128 = a.len() as u128;
	let (P, k) = find_prime(L);

	let r = primative_root(P);
	println!("at transform {:?}, {}, {}", P, k, r);
	let w = r;
	println!("primitive root: {:?}, p=nL+1---n: {},p: {}, omega: {}", r, k, P, w);

	let mut w_matrix: Vec<Vec<u128>> = Vec::new();
	for i in 0..L {
		let mut row: Vec<u128> = Vec::new();
		for j in 0..L {
			println!("{:?}, {}",i ,j);
			row.push(w.pow((i*j) as u32) % P);
			row.push(w % P);
		}
		w_matrix.push(row);	
	}
	println!("w_matrix: {:?}", w_matrix);

	DFT_u128(a, w_matrix, P)	

}