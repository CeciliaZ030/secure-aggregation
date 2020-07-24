extern crate num_bigint_dig;

use std::io::{self, BufReader, BufRead};
use std::fs::File;

use u64_impl::U64;
use u64_impl::bigInt_wrapper;

use num_bigint_dig::BigUint;
use num_bigint_dig::IntoBigUint;



fn read_input()  -> io::Result<Vec<u64>> {
    let f = File::open("outfile.txt")?;
    let f = BufReader::new(f);

    let mut v: Vec<u64> = Vec::new();

    for line in f.lines() {
        println!("hello world");
        for i in line.unwrap().split(","){
            v.push(i.trim().parse::<u64>().unwrap());
        }
        
    }
    println!("{:?}", v[0]);

    Ok(v)

}

fn main() {
	println!("hello world");
	let a = read_input().unwrap();
34erwewr    let mut input: Vec<BigUint> = Vec::new();
    for ai in a.into_iter(){
    	input.push(ai.into_biguint().unwrap());
    }

    let b = bigInt_wrapper::transform(&input);
    println!("傻逼 {:?}", b);
    println!("hello?");
}