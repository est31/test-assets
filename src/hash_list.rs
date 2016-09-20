// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

/*!
Hash list module
*/

use std::io::{Write, BufRead, BufReader, BufWriter};
use ::Sha256Hash;
use std::collections::HashMap;
use ::TaError;

pub struct HashList {
	name_to_hash_map :HashMap<String, Sha256Hash>,
}

impl HashList {
	pub fn from_file(path :&str) -> Result<Self, TaError> {
		use std::fs::File;
		let rdr = try!(File::open(path));
		let mut brdr = BufReader::new(rdr);
		return Ok(try!(HashList::from_reader(&mut brdr)));
	}

	pub fn from_reader<T :BufRead>(brdr :&mut T) -> Result<Self, TaError> {
		let mut name_to_hash_map = HashMap::new();
		for oline in brdr.lines() {
			let line = try!(oline);
			if line.starts_with("#") {
				continue;
			}
			let mut spi = line.split(" ");
			let hash_str = match spi.next() { Some(v) => v, None => continue };
			let hash = try!(Sha256Hash::from_hex(hash_str).map_err(|_| TaError::BadHashFormat));
			let name = match spi.next() { Some(v) => v, None => continue };
			name_to_hash_map.insert(name.to_owned(), hash);
		}
		return Ok(HashList {
			name_to_hash_map : name_to_hash_map,
		});
	}

	pub fn to_file(&self, path :&str) -> Result<(), TaError> {
		use std::fs::File;
		let wrt = try!(File::create(path));
		let mut bwrtr = BufWriter::new(wrt);
		self.to_writer(&mut bwrtr)
	}

	pub fn to_writer<W :Write>(&self, bwrtr :&mut BufWriter<W>) -> Result<(), TaError> {
		for (name, hash) in &self.name_to_hash_map {
			try!(bwrtr.write(format!("{} {}\n", hash.to_hex(), name).as_bytes()));
		}
		Ok(())
	}

	pub fn new() -> Self {
		return HashList {
			name_to_hash_map : HashMap::new(),
		};
	}

	pub fn get_hash<'a>(&'a self, filename :&str) -> Option<&'a Sha256Hash> {
		self.name_to_hash_map.get(filename)
	}

	pub fn add_entry(&mut self, filename :&str, hash :&Sha256Hash) {
		self.name_to_hash_map.insert(filename.to_owned(), hash.clone());
	}
}
