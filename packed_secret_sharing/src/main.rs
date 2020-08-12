use packed_secret_sharing::packed_ss::*;
use packed_secret_sharing::*;
use packed_secret_sharing::ntt;

/*
use this prime : 4610415792919412737

512th root ot unity: 1266473570726112470

729th root of unity: 2230453091198852918
*/

fn main() {
    println!("Hello, world!");

    let p = 4610415792919412737u128;
    let r2 = 1266473570726112470u128;
    let r3 = 2230453091198852918u128;

    let mut pss = PackedSecretSharing::new(p, r2, r3, 511, 4, 512);
    //prime: u128, root2: u128, root3:u128, degree: usize, num_secrets: usize, num_shares: usize
    
    let secrets: Vec<u128> = vec![6666666666u128, 8888888888u128, 9999999999u128, 1111111111u128];
    let shares = pss.share(&secrets);

    let secrets = pss.reconstruct(&shares);
    println!("{:?}", secrets);


 //    let mut arr: Vec<u128> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
	// //bench_inplace_bitreverse(&mut arr);
	// let P = 433u128;
	// let mut w_matrix: Vec<u128> = Vec::new();
	// for i in 0..arr.len()/3*2+1 {
	// 	w_matrix.push(150u128.modpow(&(i as u128), &P));	
	// }
	// ntt::DFT_radix3(&mut arr, &P, &150u128);

	// println!("{:?}", arr);
}

