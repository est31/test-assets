// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

#![deny(unsafe_code)]
#![cfg_attr(test, deny(warnings))]

/*!
Download test assets

This library downloads test assets using http(s),
and ensures integrity by comparing those assets to a hash.
*/

extern crate sha2;
extern crate hyper;

use std::io;
use hyper::client::Client;
use hyper::status::StatusCode;
use sha2::sha2::Sha256;
use sha2::digest::Digest;

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
#[derive(PartialEq, Eq)]
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
			res.push(from_digit(*v as u32, 16).unwrap());
		}
		return res;
	}
}

#[derive(Debug)]
pub enum TaError {
	Io(io::Error),
	Hyper(hyper::error::Error),
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
	Success,
	DownloadFailed(StatusCode),
	HashMismatch(String),
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
			if printer_counter % 10 == 0 {
				printer_counter = 0;
				if let Some(tlen) = total_len {
					// Print stats
					let percent = ((processed_len as f64 / tlen as f64) * 100.0).floor();
					println!("{}%", percent);
				}
			}
		}
	}
	let expected_hash = try!(Sha256Hash::from_hex(&tfile.hash).map_err(|_| TaError::BadHashFormat));
	if Sha256Hash::from_digest(&mut hasher) == expected_hash {
		return Ok(DownloadOutcome::Success);
	} else {
		return Ok(DownloadOutcome::HashMismatch(hasher.result_str().to_owned()));
	}
}

/// Downloads the test files into the passed directory.
pub fn download_test_files(defs :&[TestAssetDef],
		dir :&str, verbose :bool) -> Result<(), TaError> {
	let mut client = Client::new();
	for tfile in defs.iter() {
		if verbose {
			println!("Fetching file {} ...", tfile.filename);
		}
		let outcome = try!(download_test_file(&mut client, tfile, dir, verbose));
		if verbose {
			use self::DownloadOutcome::*;
			print!("  => ");
			match outcome {
				Success => println!("Success"),
				DownloadFailed(code) => println!("Download failed with code {}", code),
				HashMismatch(found) => println!("Hash mismatch: found {}, expected {}", found, tfile.hash),
			}
		}
	}
	panic!();
}
