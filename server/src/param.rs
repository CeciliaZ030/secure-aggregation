#[derive(Debug)]
pub struct Param {
	P: u128,
	R2: u128,
	R3: u128,
	D2: usize,
	D3: usize,

	rootTwos: Vec<u128>,
	rootThrees: Vec<u128>,
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

		println!("{:?}", rootTwos);
		println!("{:?}", rootThrees);

		Param {
			P: P,
			R2: R2,
			R3: R3,
			D2: D2,
			D3: D3,

			rootTwos: rootTwos,
			rootThrees: rootThrees,		
		}
	}
	pub fn calculate_sharing(&self, numClients: usize, vectorSize: usize) -> Vec<u128> {
		
		let mut degree = numClients/3;
		let numCorrupted = numClients/6;

		// find the nearest exponent of two
		/* Ex: degree = 300 -> 256
		*	   power2 = 8 since 2^8 = 256
		*/
		let mut n = 2;
		let mut power2 = 1;
		while (n < degree) {
			n *= 2;
			power2 += 1;
		}
		power2 -= 1;
		let degree2 = 2usize.pow(power2);
		println!("power2 {:?}, degree {:?}", power2, degree);

		// find the nearest exponent of three
		/* Ex: degree = 300 -> 256 -> 729
		*      power3 = 6 since 3^6 = 729
		*/
		let mut n = 3;
		let mut power3 = 1;
		while (n < degree) {
			n *= 3;
			power3 += 1;
		}
		let degree3 = n;
		println!("power3 {:?}", power3);


		return vec![
			degree2 as u128,					// degree2
			degree3 as u128,					// degree3
			(degree - numCorrupted) as u128,	// block length
			numCorrupted as u128,				// number of corrupted parties
			self.P as u128,						// prime
			self.rootTwos[power2 as usize],		// two-power root of unity
			self.rootThrees[power3 as usize]	// three-power root of unity
		];

	}
}