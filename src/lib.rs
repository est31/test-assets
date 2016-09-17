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

/// Definition for a test file
///
///
pub struct TestFileDef {
	/// Name of the file on disk. This should be unique for the file.
	pub filename :String,
	/// Sha256 hash of the file's data in hexadecimal lowercase representation
	pub hash :String,
	/// The url the test file can be obtained from
	pub url :String,
}

#[derive(Debug)]
pub enum TfError {
	Io(io::Error),
	Hyper(hyper::error::Error),
}

impl From<io::Error> for TfError {
	fn from(err :io::Error) -> TfError {
		TfError::Io(err)
	}
}

impl From<hyper::error::Error> for TfError {
	fn from(err :hyper::error::Error) -> TfError {
		TfError::Hyper(err)
	}
}

enum DownloadOutcome {
	Success,
	DownloadFailed(StatusCode),
	HashMismatch(String),
}

fn download_test_file(client :&mut Client,
		tfile :&TestFileDef, dir :&str) -> Result<DownloadOutcome, TfError> {
	use std::io::{Write, Read};
	use std::fs::File;
	use sha2::digest::Digest;
	use sha2::sha2::Sha256;
	let mut response = try!(client.get(&tfile.url).send());
	if !response.status.is_success() {
		return Ok(DownloadOutcome::DownloadFailed(response.status));
	}
	let mut hasher = Sha256::new();
	let mut file = try!(File::create(format!("{}/{}", dir, tfile.filename)));
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
	}
	if hasher.result_str() == tfile.hash {
		return Ok(DownloadOutcome::Success);
	} else {
		return Ok(DownloadOutcome::HashMismatch(hasher.result_str().to_owned()));
	}
}

/// Downloads the test files into the passed directory.
pub fn download_test_files(defs :&[TestFileDef],
		dir :&str, verbose :bool) -> Result<(), TfError> {
	let mut client = Client::new();
	for tfile in defs.iter() {
		if verbose {
			println!("Fetching file {} ...", tfile.filename);
		}
		let outcome = try!(download_test_file(&mut client, tfile, dir));
		if verbose {
			use self::DownloadOutcome::*;
			print!("  => ");
			match outcome {
				Success => println!("Success"),
				DownloadFailed(code) => println!("Download failed with code {}", code),
				HashMismatch(found) => println!("Hash mismatch, found hash {}, expected hash {}", found, tfile.hash),
			}
		}
	}
	panic!();
}
