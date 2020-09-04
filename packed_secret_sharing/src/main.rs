use packed_secret_sharing::packed::*;
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

    let mut pss = PackedSecretSharing::new(p, r2, r3, 511, 4, 600);
    //prime: u128, root2: u128, root3:u128, degree: usize, num_secrets: usize, num_shares: usize
    
    let secrets: Vec<u128> = vec![6666666666u128, 8888888888u128, 9999999999u128, 1111111111u128];
    let shares = pss.share(&secrets);

// ======================================================

    let mut secrets_point = vec![0u128; 4];
    for i in 0..4 {
    	secrets_point[i] = r2.modpow(&(i as u128), &p);
    }
    let mut shares_point = vec![0u128; 512];
    for i in 0..512 {
    	shares_point[i] = r3.modpow(&(i as u128), &p);
    }

    let mut shares_val = shares.clone();
    shares_val.split_off(512);

    let secrets = pss.reconstruct(&shares_point, &shares_val);
    println!("{:?}", secrets);


}

