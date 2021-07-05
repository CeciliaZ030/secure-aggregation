use std::cmp::*;

#[derive(Debug, Clone)]
pub struct Param {
	pub P: u64,
	pub R2: u64,
	pub R3: u64,
	D2: usize,
	D3: usize,
	rootTwos: Vec<u64>,
	rootThrees: Vec<u64>,

	pub useD2: usize,
	pub useD3: usize,
	pub useR2: u64,
	pub useR3: u64,
	pub L: usize,
}

impl Param {
	
	pub fn new(P: u64, R2: u64, D2: usize, R3: u64, D3: usize) -> Param {
		
		let mut rootTwos = vec![0u64; D2 + 1];
		rootTwos[D2] = R2;
		let mut temp = R2 as u128; 
		let P_u128 = P as u128;
		for i in 1..D2 {
			temp = (temp * temp) % P_u128;
			rootTwos[D2 - i] = temp as u64;
		}

		let mut rootThrees = vec![0u64; D3 + 1];
		rootThrees[D3] = R3;
		let mut temp = R3 as u128; 
		for i in 1..D3 {
			temp = (temp * temp % P_u128) * temp % P_u128;
			rootThrees[D3 - i] = temp as u64;
		}

		// println!("prime {:?}", P);
		// println!("rootTwos {:?}, {}", rootTwos, D2);
		// println!("rootThrees {:?}, {}", rootThrees, D3);

		Param {
			P: P,
			R2: R2,
			R3: R3,
			D2: D2,
			D3: D3,
			rootTwos: rootTwos,
			rootThrees: rootThrees,

			useD2: 0,
			useD3: 0,
			useR2: 0u64,
			useR3: 0u64,
			L: 0,
		}
	}

	/* Reconstruction Limit
		Theoretical limit to ensure security.  
		Actual polynomial limit should be D2, 
		which must be smaller than Reconstruction Limit
	*/
	pub fn calculate_semi_honest(&mut self, 
		numClients: usize, vectorSize: usize, dropouts: usize) -> Vec<u64> {
		
		let mut reconstructLimit = numClients - dropouts;

		// find the nearest exponent of two
		/* Ex: degree2 = 300 -> 256
		*	   power2 = 8 since 2^8 = 256
		*/
		let mut n = 2;
		let mut power2 = 1;
		while n < reconstructLimit {
			n *= 2;
			power2 += 1;
		}
		power2 -= 1;
		// Make sure don't exceed the maximun power roots provided
		assert!(power2 <= self.D2);

		self.useD2 = 2usize.pow(power2 as u32);
		self.useR2 = self.rootTwos[power2];
		self.L = greatest_factor_under(vectorSize, self.useD2 as usize);

		println!("deg2 < n {} - dropouts {} = reconstructLimit {}", numClients, dropouts, reconstructLimit);
		println!("deg2 {:?} = blocklenth {} + corruption 0", self.useD2, self.L);

		// find the nearest exponent of three
		/* Ex: degree3 = 1000 -> 729
		*      power3 = 6 since 3^6 = 729
		*/
		let mut n = 3;
		let mut power3 = 1;
		while n < numClients {
			n *= 3;
			power3 += 1;
		}
		// Make sure don't exceed the maximun power roots provided
		assert!(power3 <= self.D3);

		self.useD3 = 3usize.pow(power3 as u32);
		self.useR3 = self.rootThrees[power3];

		return vec![
			self.P,					// prime
			self.useR2,				// two-power root of unity
			self.useR3,				// three-power root of unity
			self.useD2 as u64,		// degree2
			self.useD3 as u64,		// degree3
			self.L as u64			// block length			
		];
	}

	pub fn calculate_malicious(&mut self, 
		numClients: usize, vectorSize: usize, dropouts: usize, corruption: usize) -> Vec<u64> {
		
		let mut reconstructLimit = (numClients - (dropouts + 2 * corruption))/2;

		// find the nearest exponent of two
		/* Ex: degree2 = 300 -> 256
		*	   power2 = 8 since 2^8 = 256
		*/
		let mut n = 2;
		let mut power2 = 1;
		while n < min(reconstructLimit, numClients/2) {
			n *= 2;
			power2 += 1;
		}
		power2 -= 1;
		// Make sure don't exceed the maximun power roots provided
		assert!(power2 <= self.D2);
		self.useD2 = n/2;
		self.useR2 = self.rootTwos[power2];
		self.L = greatest_factor_under(vectorSize, self.useD2 - corruption);

		println!("deg2 < n {} - (d {} + 2t {}) = reconstructLimit {}", numClients, dropouts, corruption, reconstructLimit);
		println!("deg2 {:?} = blocklenth {} + corruption {}", self.useD2, self.L, corruption);
		
		// find the nearest exponent of three
		/* Ex: degree3 = 1000 -> 729
		*      power3 = 6 since 3^6 = 729
		*/
		let mut n = 3;
		let mut power3 = 1;
		while n < numClients {
			n *= 3;
			power3 += 1;
		}
		// Make sure don't exceed the maximun power roots provided
		assert!(power3 <= self.D3);
		self.useD3 = 3usize.pow(power3 as u32);
		self.useR3 = self.rootThrees[power3];

		return vec![
			self.P,					// prime
			self.useR2,				// two-power root of unity
			self.useR3,				// three-power root of unity
			self.useD2 as u64,		// degree2
			self.useD3 as u64,		// degree3
			self.L as u64			// block length
		];
	}
}

fn greatest_factor_under(mut a: usize, b: usize) -> usize {
	/* Find the greatest facotr of a under b
	   n/k = q
	   where k is the largest factor where k < b
	   k < b
	   kq = n < bq
	   then we find the smallest factor q s.t. q | n
	*/
	assert!(a >= b);
	for i in 2..a {
		if a % i == 0 {
			if b * i >= a { 
				return a/i; 
			}
		}
	}
	return 1;
}


