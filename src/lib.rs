// Copyright (c) 2016 est31 <MTest31@outlook.com>
// and contributors. All rights reserved.
// Licensed under MIT license, or Apache 2 license,
// at your option. Please see the LICENSE file
// attached to this source distribution for details.

#![forbid(unsafe_code)]
#![cfg_attr(test, deny(warnings))]

/*!
Download test assets, managing them outside of git.

This library downloads test assets using http(s),
and ensures integrity by comparing those assets to a hash.

By managing the download separately, you can keep them
out of VCS and don't make them bloat your repository.

Usage example:

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

extern crate curl;
extern crate sha2;

mod hash_list;

use curl::easy::Easy;
use hash_list::HashList;
use sha2::digest::Digest;
use sha2::Sha256;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};

/// Definition for a test file
///
///
pub struct TestAssetDef {
    /// Name of the file on disk. This should be unique for the file.
    pub filename: String,
    /// Sha256 hash of the file's data in hexadecimal lowercase representation
    pub hash: String,
    /// The url the test file can be obtained from
    pub url: String,
}

/// A type for a Sha256 hash value
///
/// Provides conversion functionality to hex representation and back
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Sha256Hash([u8; 32]);

impl Sha256Hash {
    pub fn from_digest(sha: Sha256) -> Self {
        let sha = sha.finalize();
        let bytes = sha[..].try_into().unwrap();
        Sha256Hash(bytes)
    }

    /// Converts the hexadecimal string to a hash value
    pub fn from_hex(s: &str) -> Result<Self, ()> {
        let mut res = Sha256Hash([0; 32]);
        let mut idx = 0;
        let mut iter = s.chars();
        loop {
            let upper = match iter.next().and_then(|c| c.to_digit(16)) {
                Some(v) => v as u8,
                None => return Err(()),
            };
            let lower = match iter.next().and_then(|c| c.to_digit(16)) {
                Some(v) => v as u8,
                None => return Err(()),
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
    Curl(curl::Error),
    DownloadFailed(u32),
    BadHashFormat,
}

impl From<io::Error> for TaError {
    fn from(err: io::Error) -> TaError {
        TaError::Io(err)
    }
}

impl From<curl::Error> for TaError {
    fn from(err: curl::Error) -> TaError {
        TaError::Curl(err)
    }
}

enum DownloadOutcome {
    WithHash(Sha256Hash),
    DownloadFailed(u32),
}

fn download_test_file(
    client: &mut Easy,
    tfile: &TestAssetDef,
    dir: &str,
) -> Result<DownloadOutcome, TaError> {
    client.url(&tfile.url)?;
    let mut content = Vec::new();

    {
        let mut transfer = client.transfer();
        transfer.write_function(|data| {
            content.extend_from_slice(data);
            Ok(data.len())
        })?;
        transfer.perform()?;
    }

    let mut hasher = Sha256::new();
    let mut file = File::create(format!("{}/{}", dir, tfile.filename))?;
    file.write_all(&content)?;
    hasher.update(&content);

    let response_code = client.response_code()?;
    if response_code < 200 || response_code > 399 {
        return Ok(DownloadOutcome::DownloadFailed(response_code));
    }
    return Ok(DownloadOutcome::WithHash(Sha256Hash::from_digest(
        hasher,
    )));
}

/// Downloads the test files into the passed directory.
pub fn download_test_files(defs: &[TestAssetDef], dir: &str, verbose: bool) -> Result<(), TaError> {
    let mut client = Easy::new();
    client.follow_location(true)?;

    use std::io::ErrorKind;

    let hash_list_path = format!("{}/hash_list", dir);
    let mut hash_list = match HashList::from_file(&hash_list_path) {
        Ok(l) => l,
        Err(TaError::Io(ref e)) if e.kind() == ErrorKind::NotFound => HashList::new(),
        e => {
            e?;
            unreachable!()
        }
    };
    create_dir_all(dir)?;
    for tfile in defs.iter() {
        let tfile_hash = Sha256Hash::from_hex(&tfile.hash).map_err(|_| TaError::BadHashFormat)?;
        if hash_list
            .get_hash(&tfile.filename)
            .map(|h| h == &tfile_hash)
            .unwrap_or(false)
        {
            // Hash match
            if verbose {
                println!(
                    "File {} has matching hash inside hash list, skipping download",
                    tfile.filename
                );
            }
            continue;
        }
        if verbose {
            print!("Fetching file {} ...", tfile.filename);
        }
        let outcome = download_test_file(&mut client, tfile, dir)?;
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
                        println!(
                            "Hash mismatch: found {}, expected {}",
                            found_hash.to_hex(),
                            tfile.hash
                        )
                    }
                }
            }
        }
    }
    hash_list.to_file(&hash_list_path)?;
    Ok(())
}
