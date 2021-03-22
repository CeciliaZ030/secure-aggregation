#[derive(Debug)]
pub struct Param {
	pub P: u128,
	pub R2: u128,
	pub R3: u128,
	D2: usize,
	D3: usize,
	rootTwos: Vec<u128>,
	rootThrees: Vec<u128>,

	pub useDegree2: usize,
	pub useDegree3: usize,
	pub useRoot2: u128,
	pub useRoot3: u128,
	pub packingLen: usize,
}

impl Param {
	
	pub fn new(P: u128, R2: u128, D2: usize, R3: u128, D3: usize) -> Param {
		
		let mut rootTwos = vec![0u128; D2 + 1];
		rootTwos[D2] = R2;
		let mut temp = R2; 
		for i in 1..D2 {
			temp = (temp * temp) % P;
			rootTwos[D2 - i] = temp;
		}

		let mut rootThrees = vec![0u128; D3 + 1];
		rootThrees[D3] = R3;
		let mut temp = R3; 
		for i in 1..D3 {
			temp = (temp * temp % P) * temp % P;
			rootThrees[D3 - i] = temp;
		}

		println!("prime {:?}", P);
		println!("rootTwos {:?}, {}", rootTwos, D2);
		println!("rootThrees {:?}, {}", rootThrees, D3);

		Param {
			P: P,
			R2: R2,
			R3: R3,
			D2: D2,
			D3: D3,
			rootTwos: rootTwos,
			rootThrees: rootThrees,

			useDegree2: 0,
			useDegree3: 0,
			useRoot2: 0u128,
			useRoot3: 0u128,
			packingLen: 0,
		}
	}
	pub fn calculate_semi_honest(&mut self, numClients: usize, vectorSize: usize, dropouts: usize) -> Vec<u128> {
		
		let mut reconstructLimit = numClients - dropouts;

		// find the nearest exponent of two
		/* Ex: degree2 = 300 -> 256
		*	   power2 = 8 since 2^8 = 256
		*/
		let mut n = 2;
		let mut power2 = 1;
		while (n < reconstructLimit) {
			n *= 2;
			power2 += 1;
		}
		power2 -= 1;
		// Make sure don't exceed the maximun power roots provided
		assert!(power2 <= self.D2);

		self.useDegree2 = 2usize.pow(power2 as u32);
		self.useRoot2 = self.rootTwos[power2];
		self.packingLen = self.useDegree2;
		println!("deg2 < n - dropouts {} = reconstructLimit {}", dropouts, reconstructLimit);
		println!("deg2 {:?} = blocklenth {} + corruption 0", self.useDegree2, self.packingLen);

		// find the nearest exponent of three
		/* Ex: degree3 = 1000 -> 729
		*      power3 = 6 since 3^6 = 729
		*/
		let mut n = 3;
		let mut power3 = 1;
		while (n < numClients) {
			n *= 3;
			power3 += 1;
		}

		// Make sure don't exceed the maximun power roots provided
		assert!(power3 <= self.D3);

		self.useDegree3 = 3usize.pow(power3 as u32);
		self.useRoot3 = self.rootThrees[power3];

		return vec![
			self.useDegree2 as u128,	// degree2
			self.useDegree3 as u128,	// degree3
			self.P,						// prime
			self.useRoot2,				// two-power root of unity
			self.useRoot3,				// three-power root of unity
			self.packingLen as u128
		];
	}

	pub fn calculate_malicious(&mut self, numClients: usize, vectorSize: usize, dropouts: usize, corruption: usize) -> Vec<u128> {
		
		let mut reconstructLimit = numClients - (dropouts + 2 * corruption);

		// find the nearest exponent of two
		/* Ex: degree2 = 300 -> 256
		*	   power2 = 8 since 2^8 = 256
		*/
		let mut n = 2;
		let mut power2 = 1;
		while (n < reconstructLimit) {
			n *= 2;
			power2 += 1;
		}
		power2 -= 1;

		// Make sure don't exceed the maximun power roots provided
		assert!(power2 <= self.D2);

		self.useDegree2 = 2usize.pow(power2 as u32);
		self.useRoot2 = self.rootTwos[power2];
		self.packingLen = self.useDegree2 - corruption;

		println!("deg2 < n - (d {} + 2t {}) = reconstructLimit {}", dropouts, corruption, reconstructLimit);
		println!("deg2 {:?} = blocklenth {} + corruption {}", self.useDegree2, self.packingLen, corruption);

		// find the nearest exponent of three
		/* Ex: degree3 = 1000 -> 729
		*      power3 = 6 since 3^6 = 729
		*/
		let mut n = 3;
		let mut power3 = 1;
		while (n < numClients) {
			n *= 3;
			power3 += 1;
		}

		// Make sure don't exceed the maximun power roots provided
		assert!(power3 <= self.D3);

		self.useDegree3 = 3usize.pow(power3 as u32);
		self.useRoot3 = self.rootThrees[power3];

		return vec![
			self.useDegree2 as u128,	// degree2
			self.useDegree3 as u128,	// degree3
			self.P,						// prime
			self.useRoot2,				// two-power root of unity
			self.useRoot3,				// three-power root of unity
			self.packingLen as u128
		];
	}
}