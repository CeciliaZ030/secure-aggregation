#![allow(non_snake_case)]

use num::traits::Unsigned;
use std::convert::*;
use core::fmt::Debug;

use rand::{thread_rng, Rng};
use rand::distributions::uniform::SampleUniform;
use num_traits::{One, Zero};

mod ntt;
mod util;
use util::*;

#[derive(Clone, Debug)]
pub struct PackedSecretSharing<T> {

	prime: T,
	root2: T,
	root3: T,
	pub rootTable2: Vec<T>,
	pub rootTable3: Vec<T>,

	// degree of the sharing poly
	degree2: usize,
	degree3: usize,
	num_secrets: usize,
	num_shares: usize,
}

impl<T> PackedSecretSharing<T>
where T: ModPow + Unsigned + Copy + Debug + From<u64> + SampleUniform + PartialOrd,
{

	pub fn new(prime: T, root2: T, root3:T, 
			   degree2: usize, degree3: usize, num_secrets: usize, num_shares: usize) -> PackedSecretSharing<T> {

		assert!(num_secrets <= degree2);
		assert!(degree2 <= num_shares);
		assert!(num_shares <= degree3);

		let mut rootTable2: Vec<T> = Vec::new();
		for i in 0..degree2 as u64 {
			rootTable2.push(root2.modpow((i as u64).into(), prime));	
		}

  		let mut rootTable3: Vec<T> = Vec::new();
		for i in 0..degree3 as u64 {
			rootTable3.push(root3.modpow((i as u64).into(), prime));
		}

		PackedSecretSharing {

			prime: prime,
			root2: root2,
			root3: root3,
			rootTable2: rootTable2,
			rootTable3: rootTable3,

			degree2: degree2,
			degree3: degree3,
			num_secrets: num_secrets,
			num_shares: num_shares,
		}
	}

	pub fn share_ref<U>(&mut self, secrets: &[U]) -> Vec<U> 
	where U: TryFrom<T> + Into<T> + Copy + HasMax + SampleUniform + Unsigned + Debug,
		  //<<U as Trait>::Type as TryFrom<T>>::Error: Debug
		  <U as TryFrom<T>>::Error: Debug
	{		
		let L2 = self.degree2;
		let L3 = self.degree3;
		let zero = U::zero();

		/* Convert U into T
		T has to be a larger integer type than U to prevent overflow
		*/
		let mut _secrets: Vec<T> = Vec::new();
		for s in secrets.into_iter() {
			_secrets.push((*s).into());
		}

		assert!(_secrets.len() == self.num_secrets);

		/* Pack randomness for unused transform points
		randomness is no greater than max of U to prevent overflow
		*/
		let mut rng = thread_rng();
		println!("{:?}, {:?}", zero, U::max());
		for i in self.num_secrets..L2 {
			_secrets.push(rng.gen_range(zero, &U::max()).into());
		}

		/* use radix2_DFT to from the poly
		*/
		let mut poly = ntt::inverse2(_secrets, self.prime, &self.rootTable2);
		for _ in L2 ..L3 {
			poly.push(T::zero());
		}

		/* share with radix3_DFT
		*/
		let mut shares = ntt::transform3(poly, self.prime, &self.rootTable3);
		shares.split_off(self.num_shares);

		let mut ret: Vec<U> = Vec::new();
		for s in shares {
			if s > U::max().into() {panic!("overflow");}
			ret.push(s.try_into().unwrap());
		}
		ret
	}

	pub fn share<I, U>(&mut self, secrets: I) -> Vec<U> 
	where I: IntoIterator<Item = U>,
		  U: TryFrom<T> + Into<T> + Copy + HasMax + SampleUniform + Unsigned,
		   <U as TryFrom<T>>::Error: Debug
	{		
		let L2 = self.degree2;
		let L3 = self.degree3;
		let zero = U::zero();

		/* Convert U into T
		T has to be a larger integer type than U to prevent overflow
		*/
		let mut _secrets: Vec<T> = Vec::new();
		for s in secrets.into_iter() {
			_secrets.push(s.into());
		}

		assert!(_secrets.len() == self.num_secrets);

		/* Pack randomness for unused transform points
		randomness is no greater than max of U to prevent overflow
		*/
		let mut rng = thread_rng();
		for i in self.num_secrets..L2 {
			_secrets.push(rng.gen_range(zero, &U::max()).into());
		}

		/* use radix2_DFT to from the poly
		*/
		let mut poly = ntt::inverse2(_secrets, self.prime, &self.rootTable2);
		for _ in L2 ..L3 {
			poly.push(T::zero());
		}

		/* share with radix3_DFT
		*/
		let mut shares = ntt::transform3(poly, self.prime, &self.rootTable3);
		shares.split_off(self.num_shares);

		let mut ret: Vec<U> = Vec::new();
		for s in shares {
			ret.push(s.try_into().unwrap());
		}
		ret
	}


	pub fn reconstruct<I, U>(&mut self, shares: I) -> Vec<U> 
	where  I: IntoIterator<Item = U>,
	   	   U: From<T> + Into<T> + Copy + HasMax + SampleUniform + Unsigned
	{
		/* Convert U into T
		Number of shares collected > than threshold
		but smaller than initially distributed number
		*/
		let mut _shares: Vec<T> = Vec::new();
		for s in shares.into_iter() {
			_shares.push(s.into());
		}
		assert!(_shares.len() >= self.degree2);
		assert!(_shares.len() <= self.degree3);

		/* Evaluation point for Lagrange Interpolation
		Only evaluate up to the threshold to reconstruct
		*/
	    let mut shares_point = Vec::new();
	    for i in 0.._shares.len() {
	    	shares_point.push(self.root3.modpow((i as u64).into(), self.prime));
	    }
		self.rootTable2.split_off(self.num_secrets);
		let constructed = ntt::lagrange_interpolation(&shares_point, &_shares, &self.rootTable2, self.prime);

		let mut ret: Vec<U> = Vec::new();
		for s in constructed {
			ret.push(s.into());
		}
		ret		
	}

}