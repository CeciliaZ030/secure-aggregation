use crate::ntt;
use crate::ModPow;

use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct PackedSecretSharing {

	prime: u128,
	root2: u128,
	root3: u128,
	rootTable2: Vec<u128>,
	rootTable3: Vec<u128>,

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
		assert!(degree <= num_shares && degree >= num_secrets);
		// must be power of two to do NTT
  		//assert!((degree + 1).is_power_of_three());

  		let L3 = round_to_pow3(num_secrets);
  		let mut rootTable3: Vec<u128> = Vec::new();
		for i in 0..L3 {
			rootTable3.push(1u128);
		}

		let L2 = degree + 1;
		let mut rootTable2: Vec<u128> = Vec::new();
		for i in 0..L2 {
			rootTable2.push(root2.modpow(&(i as u128), &prime));	
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
	input = [s0, ...., s26]
		|
	FFT(input) --> with 27th root
		|
	poly = [a0, ..., a26]

	pad(poly) = [a0, ...,a31] 
		|
	FFT_radix3(pad_poly) --> with 31th root
		|
	shares = [t0, .., t31]

32 - 5 = 27
27 + 5 = 32
*/

	pub fn share(&mut self, secrets: &Vec<u128>) -> Vec<u128> {
		
		assert!(secrets.len() == self.num_secrets);
		let L2 = self.rootTable2.len();
		let L3 = self.rootTable3.len();

		// pack random values to define poly
		let mut _secrets = secrets.clone();
		let mut rng = thread_rng();
		for i in self.num_secrets..L3 {
			_secrets.push(rng.gen_range(0, &self.prime));
		}

		// use radix3_DFT to from the poly
		let mut poly = ntt::inverse3(_secrets, &self.prime, &self.root3);
		println!("poly {:?}", poly);
		
		if L3 < L2 {
			// expand the poly to fit radix2_DFT
			for i in L3 ..L2 {
				poly.push(rng.gen_range(0, &self.prime));
			}
		} else if L3 > L2 {
			// trancate the poly to fit radix2_DFT
			self.buff = poly.split_off(L2);
		}
		// total shares must be more than degree to reach threshold
		// secrets must not be truncated
		//assert!(poly.len() <= self.num_shares && poly.len() >= self.num_secrets);

		// share with radix2_DFT
		let res = ntt::transform2(poly, &self.prime, &self.root2);
		println!("len {:?}, first share {:?}", res.len(), res[0]);
		res
	}

	pub fn reconstruct(&mut self, shares: &Vec<u128>) -> Vec<u128> {

		// must have more shares than degree+1 but less than the number initialized
		assert!(shares.len() >= self.degree + 1 && shares.len() <= self.num_shares);
		let L2 = self.rootTable2.len();
		let L3 = self.rootTable3.len();

		let mut _shares = shares.clone();
		if shares.len() > L2 {
			_shares.truncate(L2);
		}

		let mut poly = ntt::inverse2(_shares, &self.prime, &self.root2);
		assert!(poly.len() >= self.num_secrets);

		if L3 < L2 {
			// trancate the poly to fit radix3_DFT
			self.buff = poly.split_off(L3);
			println!("runcate");
		} else if L3 > L2 {
			// trancate the poly to fit radix_DFT
			poly.extend(self.buff.clone());
		}
		assert!(poly.len() == L3);
		println!("poly {:?}", poly);

		ntt::transform3(poly, &self.prime, &self.root3)
	}

}

fn round_to_pow3(n: usize) -> usize {
	let mut v = 1;
    while v < n {
        v *= 3;
    }
    v
}























