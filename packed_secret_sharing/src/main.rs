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

    let mut pss = PackedSecretSharing::new(p, r2, r3, 512, 729, 4, 600);
    //prime: u128, root2: u128, root3:u128, degree: usize, num_secrets: usize, num_shares: usize
    
    let secrets: Vec<u128> = vec![6666666666u128, 8888888888u128, 9999999999u128, 1111111111u128];
    let shares = pss.share(&secrets);

    let secrets2: Vec<u128> = vec![10000u128, 200000u128, 30u128, 4000000000u128];
    let shares2 = pss.share(&secrets2);

    let mut sum = vec![0u128; 600];
    for i in 0..600 {
        sum[i] = shares[i] + shares2[i];
    }
// ======================================================

    let mut secrets_point = vec![0u128; 4];
    for i in 0..4 {
    	secrets_point[i] = r2.modpow(&(i as u128), &p);
    }
    let mut shares_point = vec![0u128; 512];
    for i in 0..512 {
    	shares_point[i] = r3.modpow(&(i as u128), &p);
    }

    let mut shares_val = sum.clone();
    shares_val.split_off(512);

    let secrets = pss.reconstruct_with_points(&shares_point, &shares_val);

    println!("{:?}", secrets);

    let value:Vec<u8> = vec![0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56];
    println!("{:?}", read_be_u64(value));

}

use std::convert::TryInto;
fn read_be_u64(input: Vec<u8>) -> Vec<u64> {
    let mut res = Vec::<u64>::new();
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_be_bytes(int_bytes.try_into().unwrap()));
        if (rest.len() < 8) {
            break;
        }
    }
    res
}


