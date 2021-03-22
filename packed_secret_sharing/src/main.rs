use packed_secret_sharing::packed::*;
use packed_secret_sharing::*;
use packed_secret_sharing::ntt;

/*
use this prime : 4610415792919412737

512th root ot unity: 1266473570726112470

729th root of unity: 2230453091198852918



3073700804129980417, 414345200490731620, 1697820560572790570, 8, 27, 2, 10
from [5, 5]

[5, 1411300642881442990, 755230403484066526, 1873193081170044888, 2073840665913069205, 1167364399051202624, 816231852543661536, 1062052081340565995, 296681500362385504, 7730202141009458]
[5, 98563416899351090, 702991160794567105, 29933450899899199, 2176346442046929493, 1596775264978130561, 260292684598891845, 1251985437174463998, 3059651942944547382, 1182115622347807999]

[1937113453806524834, 1081261608772129829, 50, 1685750291142969254, 2941097270934669262, 1135448392566993238, 2330023659693944815, 104085561992962891, 524567367177051120, 2467612606526845848]
*/


fn main() {
    println!("Hello, world!");

    let p = 3073700804129980417u128;
    let r2 = 414345200490731620u128;
    let r3 = 1697820560572790570u128;

    let mut pss = PackedSecretSharing::new(p, r2, r3, 8, 27, 2, 10);
    //prime: u128, root2: u128, root3:u128, degree: usize, num_secrets: usize, num_shares: usize
    

    let secrets: Vec<u64> = vec![5u64, 5u64];
    let shares: Vec<u64> = vec![5, 1411300642881442990, 755230403484066526, 1873193081170044888, 2073840665913069205, 1167364399051202624, 816231852543661536, 1062052081340565995, 296681500362385504, 7730202141009458];
//pss.share_u64(&secrets);
    println!("{:?}", shares);

    let secrets2: Vec<u64> = vec![5u64, 5u64];
    let shares2: Vec<u64> = vec![5, 98563416899351090, 702991160794567105, 29933450899899199, 2176346442046929493, 1596775264978130561, 260292684598891845, 1251985437174463998, 3059651942944547382, 1182115622347807999];//pss.share_u64(&secrets2);
    println!("{:?}", shares2);

    let mut rawSumm = Vec::new();
    for i in 0..secrets2.len() {
        rawSumm.push(secrets[i] + secrets2[i]);
    }

    let mut sum = vec![0u128; 10];
    for i in 0..10 {
        sum[i] = (shares[i] + shares2[i]) as u128;
    }

// ======================================================

    // let mut secrets_point = vec![0u128; 4];
    // for i in 0..4 {
    // 	secrets_point[i] = r2.modpow(&(i as u128), &p);
    // }
    // let mut shares_point = vec![0u128; 512];
    // for i in 0..512 {
    // 	shares_point[i] = r3.modpow(&(i as u128), &p);
    // }

    // let mut shares_val = sum.clone();
    // shares_val.split_off(512);

    // let secrets = pss.reconstruct_with_points(&shares_point, &shares_val);
    let test: Vec<u128> = vec![1937113453806524834, 1081261608772129829, 50, 1685750291142969254, 2941097270934669262, 1135448392566993238, 2330023659693944815, 104085561992962891, 524567367177051120, 2467612606526845848];
    let secrets = pss.reconstruct(&test);

    println!("{:?}", secrets);
    println!("{:?}", rawSumm);


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


