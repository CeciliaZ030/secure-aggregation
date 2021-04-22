use pss::*;
use pss::util::ModPow;
use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};

fn main() {
    println!("Hello, world!");
    let BENCH_TIMER = Instant::now();

    let p = 4610415792919412737u128;
    let r2 = 1266473570726112470u128;
    let r3 = 2230453091198852918u128;

    let mut pss = PackedSecretSharing::<u128>::new(p, r2, r3, 
        512, 729, 51200, 512, 700);

    let mut rng = thread_rng();
    let mut secrets = vec![0u64; 5120*8];
    for i in 0..5120*8 {
        secrets[i] = rng.gen_range(0, u64::MAX);
    }
    let shares = pss.share(&secrets);
    println!("Elapse {:?}ms", BENCH_TIMER.elapsed().as_millis());

}