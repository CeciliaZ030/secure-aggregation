use std::io::{self, BufReader, BufRead};
use std::fs::File;
use std::path::Path;

pub mod ntt;
pub mod packed_ss;

/** I/O util **/

// pub fn read_input_to_BigUint(p : &Path)  -> io::Result<Vec<BigUint>> {
//     let f = File::open(p)?;
//     let f = BufReader::new(f);

//     let mut v: Vec<BigUint> = Vec::new();

//     for line in f.lines() {
//         for i in line.unwrap().split(" "){
//             let temp = i.trim().parse::<u64>().unwrap();
//             v.push(temp.into_biguint().unwrap());
//         }
//     }
//     Ok(v)
// }

pub fn read_input_to_u128(p : &Path)  -> io::Result<Vec<u128>> {
    let f = File::open(p)?;
    let f = BufReader::new(f);

    let mut v: Vec<u128> = Vec::new();

    for line in f.lines() {
        for i in line.unwrap().split(" "){
            let temp = i.trim().parse::<u128>().unwrap();
            v.push(temp);
        }
    }
    Ok(v)
}

/** Arithmatic util **/

pub trait ModPow<T> {
    fn modpow(&self, exponent: &T, modulus: &T) -> T;
}

impl ModPow<u128> for u128 {
    /// Panics if the modulus is zero.
    fn modpow(&self, exponent: &Self, modulus: &Self) -> Self {

        assert!(*modulus != 0u128, "divide by zero!");
        if exponent == &0u128 {
            return 1
        }

        let mut base = self % modulus;
        let mut exp = exponent.clone();
        let mut res = 1;

        while exp > 0 {
            if exp % 2u128 == 1 {
                res = res * base % modulus;
            }
            exp >>= 1;
            base = base * base % modulus;
        }
        return res
    }
}