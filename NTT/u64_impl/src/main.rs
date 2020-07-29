#![feature(test)]
#![allow(non_snake_case)]


extern crate test;
extern crate num_bigint_dig;

use std::io::{self, BufReader, BufRead};
use std::fs::File;
use rand::prelude::*;
use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;
use u64_impl::bigInt_wrapper;


//Banchmark Example
pub fn add_two(a: i32) -> i32 {
    a + 2
}


#[cfg(test)]
mod tests {

    use super::*;
    use test::Bencher;

    #[test]
    fn it_works() {
        assert_eq!(4, add_two(2));
    }

    #[bench]
    fn bench_add_two(b: &mut Bencher) {
        b.iter(|| add_two(2));
    }

    #[bench]
    fn bench_ntt(b: &mut Bencher) {

        b.iter(||{
            let mut rng = thread_rng();
            let s : u32 = rng.gen_range(0, 1);
            let mut arr = match s {
                0 => read_input(&"sample1.txt"),
                1 => read_input(&"sample2.txt"),
                _ => panic!("Invalid number"),
            }.unwrap();

            let mut X: Vec<BigUint> = Vec::new();
            let prime = arr[0].into_biguint().unwrap();
            let root = arr[1].into_biguint().unwrap();
            for i in 2..arr.len() {
                X.push(arr[i].into_biguint().unwrap());
            }
            bigInt_wrapper::transform(&X, &prime, &root);
        });
    }

}


pub fn read_input(p : &str)  -> io::Result<Vec<u64>> {
    let f = File::open(p)?; //may use path
    let f = BufReader::new(f);

    let mut v: Vec<u64> = Vec::new();

    for line in f.lines() {
        //First numbers of samples are prime and root of unity
        for i in line.unwrap().split(","){
            v.push(i.trim().parse::<u64>().unwrap());
        }        
    }
    Ok(v)
}


fn main() {

	println!("hello world");
	let a = read_input("sample1").unwrap();
    let mut X: Vec<BigUint> = Vec::new();
    let prime = arr[0].into_biguint().unwrap();
    let root = arr[1].into_biguint().unwrap();
    for i in 2..arr.len() {
        X.push(arr[i].into_biguint().unwrap());
    }

    bigInt_wrapper::transform(&X, &prime, &root);

}



