use std::convert::TryInto;

pub fn write_u64_le_u8(v: &[u64]) -> &[u8] {
	/*
		Write u64 integer array into continuous bytes array
	*/
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<u64>(),
        )
    }
}

pub fn read_le_u128(input: Vec<u8>) -> Vec<u128> {
	/*
		Read little endian bytes Vec<u8> of u128 integer array
		back to Vec<u128>
	*/
    let mut res = Vec::<u128>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u128>());
        *ptr = rest;
        res.push(u128::from_le_bytes(int_bytes.try_into().unwrap()));
        if (rest.len() < 8) {
            break;
        }
    }
    res
}

pub fn read_le_u64(input: Vec<u8>) -> Vec<u64> {
	/*
		Read little endian bytes Vec<u8> of u64 integer array
		back to Vec<u64>
	*/
    let mut res = Vec::<u64>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_le_bytes(int_bytes.try_into().unwrap()));
        if (rest.len() < 8) {
            break;
        }
    }
    res
}


pub fn read_le_usize(input: &Vec<u8>) -> Vec<u64> {
	/*
		Read little endian bytes Vec<u8> of usize integer array
		back to Vec<usize>
	*/
    let mut res = Vec::<u64>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_le_bytes(int_bytes.try_into().unwrap()));
        if rest.len() < 8 {
            break;
        }
    }
    res
}


pub fn into_be_u64_vec(mut input: u64, size: usize) -> Vec<u64> {
	/*
		Convert u64 integer into Vec<u64> = [bit0 as u64, bit2 as u64, ...]
		Big endian.
	*/
	let mut bitArr = Vec::<u64>::new();
	while input > 0 {
		bitArr.push((input % 2) as u64);
		input = input >> 1;
	}
	for _ in bitArr.len()..size {
		bitArr.push(0)
	}
	bitArr
}
