use pss::*;

fn main() {
    println!("Hello, world!");

    let p = 3073700804129980417u128;
    let r2 = 414345200490731620u128;
    let r3 = 1697820560572790570u128;

    let mut pss = PackedSecretSharing::<u128>::new(p, r2, r3, 8, 27, 2, 10);
    //prime: u128, root2: u128, root3:u128, degree: usize, num_secrets: usize, num_shares: usize
    

    let secrets: Vec<u128> = vec![5u128, 5u128];
    let secrets2: Vec<u128> = vec![5u128, 5u128];

    let mut rawSumm = Vec::new();
    for i in 0..secrets.len() {
        rawSumm.push(secrets[i] + secrets2[i]);
    }

    let shares: Vec<u128> = pss.share(secrets);
    println!("{:?}", shares);
    let shares2: Vec<u128> = pss.share(secrets2);
    println!("{:?}", shares2);


    let mut sum = vec![0u128; 10];
    for i in 0..10 {
        sum[i] = (shares[i] + shares2[i]) as u128;
    }

// ======================================================

    let secrets = pss.reconstruct(sum);

    println!("{:?}", secrets);
    println!("{:?}", rawSumm);


}