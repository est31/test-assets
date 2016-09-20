// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

#![deny(unsafe_code)]
#![cfg_attr(test, deny(warnings))]

/*!
Download test assets and cache them on disk

This library downloads test assets using http(s),
and ensures integrity by comparing those assets to a hash.

Example:

```
#[test]
fn some_awesome_test() {
	let asset_defs = [
		TestAssetDef {
			filename : format!("file_a.png"),
			hash : format!("<sha256 here>"),
			url : format!("https://url/to/a.png"),
		},
		TestAssetDef {
			filename : format!("file_b.png"),
			hash : format!("<sha256 here>"),
			url : format!("https://url/to/a.png"),
		},
	];
	test_assets::download_test_files(&asset_defs,
		"test-assets", true).unwrap();
	// use your files here
	// with path under test-assets/file_a.png and test-assets/file_b.png
}
```

If you have run the test once, it will re-use the files
instead of re-downloading them.
*/

extern crate sha2;
extern crate hyper;

mod hash_list;

use std::io;
use hyper::client::Client;
use hyper::status::StatusCode;
use sha2::sha2::Sha256;
use sha2::digest::Digest;
use hash_list::HashList;
use std::fs::create_dir_all;


/// Definition for a test file
///
///
pub struct TestAssetDef {
	/// Name of the file on disk. This should be unique for the file.
	pub filename :String,
	/// Sha256 hash of the file's data in hexadecimal lowercase representation
	pub hash :String,
	/// The url the test file can be obtained from
	pub url :String,
}

/// A type for a Sha256 hash value
///
/// Provides conversion functionality to hex representation and back
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Sha256Hash([u8; 32]);

impl Sha256Hash {

	pub fn from_digest(sha :&mut Sha256) -> Self {
		let mut res = Sha256Hash([0; 32]);
		sha.result(&mut res.0);
		return res;
	}

	/// Converts the hexadecimal string to a hash value
	pub fn from_hex(s :&str) -> Result<Self, ()> {
		let mut res = Sha256Hash([0; 32]);
		let mut idx = 0;
		let mut iter = s.chars();
		loop {
			let upper = match iter.next().and_then(|c| c.to_digit(16)) {
				Some(v) => v as u8,
				None => try!(Err(())),
			};
			let lower = match iter.next().and_then(|c| c.to_digit(16)) {
				Some(v) => v as u8,
				None => try!(Err(())),
			};
			res.0[idx] = (upper << 4) | lower;
			idx += 1;
			if idx == 32 {
				break;
			}
		}
		return Ok(res);
	}
	/// Converts the hash value to hexadecimal
	pub fn to_hex(&self) -> String {
		let mut res = String::with_capacity(64);
		for v in self.0.iter() {
			use std::char::from_digit;
			res.push(from_digit(*v as u32 >> 4, 16).unwrap());
			res.push(from_digit(*v as u32 & 15, 16).unwrap());
		}
		return res;
	}
}

#[derive(Debug)]
pub enum TaError {
	Io(io::Error),
	Hyper(hyper::error::Error),
	DownloadFailed(StatusCode),
	BadHashFormat,
}

impl From<io::Error> for TaError {
	fn from(err :io::Error) -> TaError {
		TaError::Io(err)
	}
}

impl From<hyper::error::Error> for TaError {
	fn from(err :hyper::error::Error) -> TaError {
		TaError::Hyper(err)
	}
}

enum DownloadOutcome {
	WithHash(Sha256Hash),
	DownloadFailed(StatusCode),
}

fn download_test_file(client :&mut Client,
		tfile :&TestAssetDef, dir :&str, verbose :bool) -> Result<DownloadOutcome, TaError> {
	use std::io::{Write, Read};
	use std::fs::File;
	let mut response = try!(client.get(&tfile.url).send());
	if !response.status.is_success() {
		return Ok(DownloadOutcome::DownloadFailed(response.status));
	}
	let mut hasher = Sha256::new();
	let mut file = try!(File::create(format!("{}/{}", dir, tfile.filename)));
	let mut printer_counter = 0;
	let mut processed_len = 0;
	let total_len = response.headers.get::<hyper::header::ContentLength>().map(|v| v.0).clone();
	loop {
		let mut arr = [0; 256];
		let len = try!(response.read(&mut arr));
		if len == 0 {
			// EOF reached.
			break;
		}
		let data = &arr[.. len];
		hasher.input(data);
		try!(file.write_all(data));
		processed_len += len;
		if verbose {
			printer_counter += 1;
			if printer_counter % 1000 == 0 {
				printer_counter = 0;
				if let Some(tlen) = total_len {
					// Print stats
					let percent = ((processed_len as f64 / tlen as f64) * 100.0).floor();
					print!(" {}%", percent);
				}
			}
		}
	}
	return Ok(DownloadOutcome::WithHash(Sha256Hash::from_digest(&mut hasher)));
}

/// Downloads the test files into the passed directory.
pub fn download_test_files(defs :&[TestAssetDef],
		dir :&str, verbose :bool) -> Result<(), TaError> {
	let mut client = Client::new();

	use std::io::ErrorKind;

	let hash_list_path = format!("{}/hash_list", dir);
	let mut hash_list = match HashList::from_file(&hash_list_path) {
		Ok(l) => l,
		Err(TaError::Io(ref e)) if e.kind() == ErrorKind::NotFound => HashList::new(),
		e => { try!(e); unreachable!() },
	};
	try!(create_dir_all(dir));
	for tfile in defs.iter() {
		let tfile_hash = try!(Sha256Hash::from_hex(&tfile.hash).map_err(|_| TaError::BadHashFormat));
		if hash_list.get_hash(&tfile.filename).map(|h| h == &tfile_hash)
				.unwrap_or(false) {
			// Hash match
			if verbose {
				println!("File {} has matching hash inside hash list, skipping download", tfile.filename);
			}
			continue;
		}
		if verbose {
			print!("Fetching file {} ...", tfile.filename);
		}
		let outcome = try!(download_test_file(&mut client, tfile, dir, verbose));
		use self::DownloadOutcome::*;
		match &outcome {
			&DownloadFailed(code) => return Err(TaError::DownloadFailed(code)),
			&WithHash(ref hash) => hash_list.add_entry(&tfile.filename, hash),
		}
		if verbose {
			print!("  => ");
			match &outcome {
				&DownloadFailed(code) => println!("Download failed with code {}", code),
				&WithHash(ref found_hash) => {
					if found_hash == &tfile_hash {
						println!("Success")
					} else {
						println!("Hash mismatch: found {}, expected {}",
							found_hash.to_hex(), tfile.hash)
					}
				 },
			}
		}
	}
	try!(hash_list.to_file(&hash_list_path));
	Ok(())
}
