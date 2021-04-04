
	fn input_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
		Get the share and delivery target
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(3))
		}
		let M = self.MAX.read().unwrap().clone();
		let B = self.V / self.param.read().unwrap().L;
		let shares = match msg {
			RecvType::matrix(m) => {
				if m.len() != M || m[0].len() != 5 * B {		//TODO: only 5 sections
					send(&worker.dealer, "Please share with specified parameters.", &clientID);
					return Err(WorkerError::UnexpectedFormat(3)); 
				}
				m
			}, 
			_ => {
				send(&worker.dealer, "Please send your shares as matrix.", &clientID);
				return Err(WorkerError::UnexpectedFormat(3));
			},
		};
	/*
		For each outbounding share
		Send [senderPk, share]
	*/
		let listGuard = match self.clientList.read() {
			Ok(guard) => guard,
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		};
		let senderPk = match self.clientProfiles.read() {
			Ok(guard) => guard.get(&clientID).unwrap().publicKey.clone(),
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		}; 
		
		println!("{:?} forwarded with pk {:?}", str::from_utf8(&clientID).unwrap(), senderPk);
		
		assert_eq!(shares.len(), listGuard.len());
		for i in 0..shares.len() {
			/* 
			attach senderPK so that reciever knows who is this from
			and which sharedKey to use
			*/
			let msg = vec![senderPk.clone(), shares[i].clone()];
			match send_vecs(&worker.dealer, msg, &listGuard[i]) {
				Ok(_) => continue,
				Err(_) => return Err(WorkerError::SharingFail(3)),
			};
			println!("share (len: {:?}) from {:?} to {:?}", 
				shares[i].len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&listGuard[i]).unwrap());
		}
		match self.clientProfiles.write() {
			Ok(mut guard) => guard.get_mut(&clientID).unwrap().hasShared = true,
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		}; 
		worker.threadSender.send(3);
		return Ok(3)
	}

	fn error_correction(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(4))
		}
		let mut recvVecs = vec![Vec::new(); 1];						//TODO
		let M = self.MAX.read().unwrap().clone();
		match msg {
			RecvType::matrix(m) => {
				if m.len() != 1 || m[0].len() != M {			//TODO: only degree test
					send(&worker.dealer, "Please send your degree test matrix. 
											Format: [[Degree test], [Input Bit test], [Quadratic test], [Input bound test], 
											[L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]", &clientID);
					return Err(WorkerError::UnexpectedFormat(4))
				}
				for i in 0..1 {									//TODO
					recvVecs[i] = read_le_u64(&m[0]);
				}
			}, 
			_ => {
				send(&worker.dealer, "Please send your degree test matrix. 
					Format: [[Degree test], [Input Bit test], [Quadratic test], [Input bound test], 
					[L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]", &clientID);
				return Err(WorkerError::UnexpectedFormat(4))
			},
		};
		let idx = self.clientList
						.read().unwrap()
						.iter().position(|s| s == &clientID)
						.unwrap();
		match self.correctionVecs.lock() {
			Ok(mut guard) => {
				for i in 0..M {
					guard[0][i][idx] = recvVecs[0][i].clone();	// Degree test
					// guard[1][i].push(testsVecs[1][i].clone());	// Input Bit test
					// guard[3][i].push(testsVecs[3][i].clone());	// Quadratic test
					// guard[4][i].push(testsVecs[4][i].clone());	// Input bound test
					// guard[5][i].push(testsVecs[5][i].clone());	// L2-norm sum test
					// guard[6][i].push(testsVecs[6][i].clone());	// L2-norm bit test
					// guard[7][i].push(testsVecs[7][i].clone());	// L2-norm bound test
				}
			},
			Err(_) => return Err(WorkerError::MutexLockFail(4)),
		};
		return Ok(4);
	}

	fn shares_collection(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exist
		Get shares & signature
	*/

	//?
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(5))
		}
		let msg = match msg {
			RecvType::matrix(m) => {
				if m.len() != 2 {
					send(&worker.dealer, 
						"Please send your shares with a signature. Format: [shares, Enc(shares)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(5))
				}
				m
			},
			_ => {
				send(&worker.dealer, 
					"Please send your shares key with a signature. Format: [shares, Enc(shares)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(5))
			},
		};
	/*
		Verify & Safe
	*/

		let verifyResult = match self.clientProfiles.read() {
			Ok(mut guard) => {

			 	let veriKey = guard.get(&clientID).unwrap().veriKey.clone();
				veriKey.verify(
					&msg[0], 										//shares
					&Signature::from_bytes(&msg[1]).unwrap()		//signature of shares
				)
			},
			Err(_) => return Err(WorkerError::MutexLockFail(5)),
		};
		match verifyResult {
			Ok(_) => {
				self.shares.lock().unwrap().push(read_le_u64(&msg[0]));
		 		send(&worker.dealer, 
		 			"Your aggregated shares has been save.", 
		 			&clientID);
		 		worker.threadSender.send(5);
		 		return Ok(5)
			},
			Err(_) => {
		 		send(&worker.dealer, "Error: Decryption Fail.", &clientID);
				return Err(WorkerError::DecryptionFail(5))
			},
		}		

	}


	fn reconstruction(&self) -> Result<Vec<u64>, WorkerError> {
		let sharesGuard = match self.shares.lock() {									// Mutex Obtained
			Ok(mut guard) => guard,
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		};		
	    let dropoutsGuard = match self.dropouts.lock() {
	    	Ok(guard) => guard,
	    	Err(_) => return Err(WorkerError::MutexLockFail(0)),
	    };
		// Handles dropouts
		let B = sharesGuard[0].len();
		let N = sharesGuard.len();
		let M = *self.MAX.write().unwrap();
		let param = &self.param.read().unwrap();
		let mut result = Vec::new();
		println!("reconstruction param {:?}, {}, {}, {}", param.useD2, param.useD3, param.L, N);
		let pss = PackedSecretSharing::new(
			param.P, param.R2, param.R3, 
			param.useD2, param.useD3, 5 * M ,param.L, N
		);
		let mut sharesPoints = Vec::new();
		for i in 0..param.useD2 {
			if !dropoutsGuard.contains(&i) {
		    	sharesPoints.push(param.useR3.modpow(i as u64, param.P));
			}
		}
		let ret = pss.reconstruct(&sharesGuard, sharesPoints.as_slice());
		// for i in 0..B {
		// 	let mut pss;
		// 	// When V = B * L + remains
		// 	if i == B-1 && (param.L as usize) * B > self.V {
		// 		println!("{:?} * {} < {}", (param.L as usize), B, self.V);
		// 		pss = PackedSecretSharing::new(
		// 			param.P, param.R2, param.R3, 
		// 			param.useD2, param.useD3, self.V-(param.L as usize)*(B-1), N
		// 		);
		// 	}
		// 	else {
		// 		pss = PackedSecretSharing::new(
		// 			param.P, param.R2, param.R3, 
		// 			param.useD2, param.useD3, param.L, N
		// 		);
		// 	}
		// 	let mut shares = Vec::new();
		// 	for j in 0..N {
		// 		shares.push(sharesGuard[j][i] as u64);
		// 	}
		// 	result.extend(pss.reconstruct(&shares));
		// }
		println!("Reconstruction DONE");
		println!("constructed len {:?}", result.len());
		result.split_off(self.V);
		return Ok(result);
	}