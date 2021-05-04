use pss::*;
use pss::util::ModPow;
use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};

/*
use this prime : 4610415792919412737

512th root ot unity: 1266473570726112470

729th root of unity: 2230453091198852918
*/

fn main() {
    println!("Hello, world!");
    let BENCH_TIMER = Instant::now();


    let p = 3073700804129980417 as u128;
    let r2 = ((437380319823159113 * 437380319823159113) % p) as u128;
    let r3 = 1697820560572790570 as u128;
    let d2 = 16/2;
    let d3 = 27;

    let mut pss = PackedSecretSharing::new(p, r2, r3, d2, d3, d2*4, 4, d3-1);

    let mut rng = thread_rng();
    let mut secrets1 = vec![0u64; d2*4];
    for i in 0..d2*4 {
        secrets1[i] = rng.gen_range(0, 2);
    }
    // let mut secrets2 = vec![0u64; 512*5];
    // for i in 0..512*5 {
    //     secrets2[i] = rng.gen_range(0, 1);
    // }
    println!("secrets1 {:?}", secrets1);

    let mut shares1 = pss.share(&secrets1);

    //let shares2 = pss.share(&secrets2);

    // let mut eval = Vec::new();
    // println!("example {:?}", shares1[0]);
    // let mut IBTT_vec = Vec::new();
    // for n in 0..d3-1 {
    //     let mut IBTT = 0u64;
    //     for i in 0..4 {
    //         IBTT += (shares1[n][i] as u128 *  (1 + p - shares1[n][i] as u128)  %  p) as u64;
    //         IBTT %= p as u64;
    //     }
    //     IBTT_vec.push(vec![IBTT]);
    //     eval.push(r3.modpow((n+1) as u128, p) as u64);
    // }
    // let mut pss = PackedSecretSharing::new(p, r2, r3, d2, d3, d2, d2, d3-1);

    let mut eval = Vec::new();
    println!("example {:?}", shares1);
    for n in 0..d3-1 {
        for i in 0..16/2 {
            shares1[n][i] = (shares1[n][i] as u128 *  ((shares1[n][i]-1) as u128)  %  p) as u64;
        }
        eval.push(r3.modpow((n+1) as u128, p) as u64);
    }
    println!("{:?}", pss.reconstruct(&shares1, &eval));
    println!("Elapse {:?}ms", BENCH_TIMER.elapsed().as_millis());

}









