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
    let mut secrets2 = vec![0u64; 200];
    for i in 0..200 {
        secrets2[i] = i as u64;
    }

    let shares = pss.share(&secrets);
    let shares2 = pss.share(&secrets2);
    println!("{:?} * {}", shares.len(), shares[0].len());
    println!("{:?}", shares);

    let mut added_shares = vec![vec![0; shares[0].len()]; shares.len()];
    for i in 0..shares.len() {
        for j in 0..shares[0].len() {
            added_shares[i][j] = shares[i][j] + shares2[i][j];
        }
    }


    let mut shares_point = Vec::new();
    for i in 1..25+1 {
        shares_point.push(r3.modpow((i as u128), p) as u64);
    }
    let secrets = pss.reconstruct(&added_shares.as_slice(), shares_point.as_slice());

    println!("{:?}", secrets);
//     //println!("{:?}", rawSumm);


}