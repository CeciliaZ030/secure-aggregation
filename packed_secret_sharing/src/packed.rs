use crate::ntt;
use crate::ModPow;

use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct PackedSecretSharing {

	prime: u128,
	root2: u128,
	root3: u128,
	pub rootTable2: Vec<u128>,
	pub rootTable3: Vec<u128>,

	// degree of the sharing poly
	degree: usize,
	num_secrets: usize,
	num_shares: usize,

	buff: Vec<u128>,

}

impl PackedSecretSharing {

	pub fn new(prime: u128, root2: u128, root3:u128, 
			   degree: usize, num_secrets: usize, num_shares: usize) -> PackedSecretSharing {

		// degree must allow num_secrets to uniquely define the poly
		// must be power of two to do NTT
		assert!(degree <= num_shares && degree >= num_secrets);
  		assert!((degree + 1).is_power_of_two());

		let L2 = degree + 1;
		let mut rootTable2 = vec![0u128; L2];
		for i in 0..L2 {
			rootTable2[i] = root2.modpow(&(i as u128), &prime);	
		}

  		let L3 = round_to_pow3(num_shares);
  		let mut rootTable3 = vec![0u128; L3];
		for i in 0..L3 {
			rootTable3[i] = root3.modpow(&(i as u128), &prime);
		}

		PackedSecretSharing {

			prime: prime,
			root2: root2,
			root3: root3,
			rootTable2: rootTable2,
			rootTable3: rootTable3,

			degree: degree,
			num_secrets: num_secrets,
			num_shares: num_shares,

			buff: Vec::new(),
		}

	}

/*
	secret = [s0, ...., s512]
		|
	FFT2(input) --> with 512th root
		|
	poly = [a0, ..., a512]
		|
	pad(poly) = [a0, ...,a729] 
		|
	FFT3(pad_poly) --> with 729th root
		|
	shares = [t0, .., t729]

======================================
	
	shares = [t0, .., t512]
		|
	Lagrange(shares)
		|
	poly = [a0, ..., a512]
		|
	FFT2(poly) --> with 512th root
		|
	secret = [s0, ...., s26]
*/

	pub fn share(&mut self, secrets: &Vec<u128>) -> Vec<u128> {
		
		assert!(secrets.len() == self.num_secrets);
		let L2 = self.rootTable2.len();
		let L3 = self.rootTable3.len();

		// pack random values to define poly
		let mut _secrets = secrets.clone();
		let mut rng = thread_rng();
		for i in self.num_secrets..L2 {
			_secrets.push(rng.gen_range(0, &self.prime));
		}

		// use radix2_DFT to from the poly
		let mut poly = ntt::inverse2(_secrets.clone(), &self.prime, &self.rootTable2);
		//println!("poly first coeff {:?}", poly[0]);


		for i in L2 ..L3 {
			poly.push(0u128);
		}

		// share with radix2_DFT
		let shares = ntt::transform3(poly, &self.prime, &self.rootTable3);
		//println!("len {:?}, first share {:?}", shares.len(), shares[0]);

		shares
	}

	pub fn reconstruct(&mut self, shares_point: &Vec<u128>, shares_val: &Vec<u128>) -> Vec<u128> {

		// must have more shares than degree+1 but less than the number initialized
		assert!(shares_point.len() >= self.degree + 1 && shares_point.len() <= self.num_shares);
		assert!(shares_point.len() == shares_val.len());

		self.rootTable2.split_off(self.num_secrets);
		lagrange_interpolation(&shares_point, &shares_val, &self.rootTable2, &self.prime)

	}

}

/**		UTIL	**/
pub fn lagrange_interpolation (points: &Vec<u128>, values: &Vec<u128>, roots: &Vec<u128>, P: &u128) -> Vec<u128> {
	
	//println!("Lagrange Interpolation \nrecieved {:?} points, evaluating {} roots", points.len(), roots.len());
	let L = points.len();

	let mut denominators: Vec<u128> = Vec::new();
	for i in 0..L {
		let mut d = 1;
		for j in 0..L {
			if i != j {
				if points[i] >= points[j]{
					d *= (points[i] - points[j]);
				} else {
					d *= (points[i] + P - points[j]) % P;
				}
				d %= P;
			}
		}
		d = (d as u128).modpow(&(P - 2u128), &P);
		denominators.push(d);
	}

	let mut evals: Vec<u128> = Vec::new();
	for r in roots {
		let mut eval = 0u128;
		for i in 0..L {
			let mut li = 1u128;
			for j in 0..L {
				if i != j {
					if r >= &points[j] {
						li *= (r - points[j]);
					} else {
						li *= (r + P - points[j]) % P;
					}
					li %= P;
				}
			}
			li = li * denominators[i] % P;
			eval += li * values[i] % P;
		}
		evals.push(eval % P);
	}
	
	evals
}

pub fn trigits_len(n: usize) -> usize {
    let mut result = 1;
    let mut value = 3;
    while value < n + 1 {
        result += 1;
        value *= 3;
    }
    result
}

fn round_to_pow3(n: usize) -> usize {
	let mut v = 1;
    while v < n {
        v *= 3;
    }
    v
}



















