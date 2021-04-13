use pss::*;
use pss::util::ModPow;

fn main() {
    println!("Hello, world!");

    let p = 3073700804129980417u128;
    let r2 = 414345200490731620u128;
    let r3 = 1697820560572790570u128;
    // 3073700804129980417, 437380319823159113, 1697820560572790570, 16, 27, 10
    let mut pss = PackedSecretSharing::<u128>::new(p, r2, r3, 8, 27, 200, 5, 25);
    //prime: u128, root2: u128, root3:u128, degree2: usize, degree3: usize, 
    //total_len: usize, packing_len: usize, num_shares: usize
    
    
    let mut secrets = vec![0u64; 200];
    for i in 0..200 {
        secrets[i] = i as u64;
    }

    let shares = pss.share(&secrets);
    println!("{:?} * {}", shares.len(), shares[0].len());
    println!("{:?}", shares);

//     let secrets = vec![5u64; 200];
//     let secrets2 = vec![9u64; 200];

//     let mut rawSumm = Vec::new();
//     for i in 0..secrets.len() {
//         rawSumm.push(secrets[i] + secrets2[i]);
//     }

//     let shares = pss.share_ref(&secrets);
//     println!("{:?} * {:?}", shares.len(), shares[0].len());
//     let shares2 = pss.share_ref(&secrets2);
//     //println!("{:?}", shares2);


//     // let mut sum = vec![0u128; 10];
//     // for i in 0..10 {
//     //     sum[i] = (shares[i] + shares2[i]) as u128;
//     // }

// // ======================================================
    let mut shares_point = Vec::new();
    for i in 1..25+1 {
        shares_point.push(r3.modpow((i as u128), p) as u64);
    }
    let secrets = pss.reconstruct(&shares, shares_point.as_slice());

    println!("{:?}", secrets);
//     //println!("{:?}", rawSumm);


}