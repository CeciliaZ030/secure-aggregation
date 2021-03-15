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

			// init through calculate_sharing
			useDegree2: 0,
			useDegree3: 0,
			useRoot2: 0u128,
			useRoot3: 0u128,
		}
	}
	pub fn calculate(&mut self, 
		numClients: usize, vectorSize: usize, dropouts: usize, corruption: usize, malicious: bool){
		
		let mut reconstructLimit = 16;
		let numCorrupted = numClients/6;

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
		//power2 -= 1;
		self.useDegree2 = 2usize.pow(power2 as u32);
		self.useRoot2 = self.rootTwos[power2];
		println!("power2 {:?}, reconstruction limit {:?}", power2, reconstructLimit);

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
		self.useDegree3 = 3usize.pow(power3 as u32);
		self.useRoot3 = self.rootThrees[power3];
		println!("power3 {:?}", power3);

		// blockLength <= numClient/3
		/* Ex: N = 1000, V = 100,000
		*	   L = 1000/3 = 333 
		*	   B = 100,000/333 = 300
		*		256 < 333 < 512
		*	decrease the numbers of NTT rounds
		*	V = B↓ * L↑ 
		*	  = V/(N/3)*(N/3)
		*	
		*	B <= L
		*	V/(N/3) <= N/3
		*	V <= (N^2)/9	10 <= 10*10/9	40 <= 20*20/9	100 <= 30*30/9	160 <= 40*40/9	100,000 <= 1000*1000/9
		*					10 = 3*3+1		40 = 6*6+4		100 = 10*10		160 = 13*12+4	100,000 = 333*300+100
		*/

	}

	pub fn send(&self) -> Vec<u128>{

		return vec![
			self.useDegree2 as u128,	// degree2
			self.useDegree3 as u128,	// degree3
			self.P,						// prime
			self.useRoot2,				// two-power root of unity
			self.useRoot3,				// three-power root of unity
		];


	}
}