use crypto::digest::Digest;
use crypto::sha1::Sha1;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use tar::Builder;
use walkdir::WalkDir;

use MainError;

pub fn load_checksums(fname: String) -> Result<HashMap<String, String>, MainError> {
	match File::open(fname) {
		Ok(checksums_file) => {
			let mut checksums : HashMap<String, String> = HashMap::new();
			let checksums_reader = BufReader::new(&checksums_file);
			for line in checksums_reader.lines() {
				match line {
					Ok(l) => {
						let mut fields = l.split_whitespace();
						let checksum = match fields.next() {
							Some(f) => f,
							_ => continue
						};
						let filename = match fields.next() {
							Some(f) => f,
							_ => continue
						};
						trace!("Previous version checksum: {}\t{}", filename, checksum);
						checksums.insert(filename.to_string(), checksum.to_string());
					},
					_ => continue
				}
			}
			checksums.shrink_to_fit();
			Ok(checksums)
		},
		Err(e) => Err(MainError::OtherError(format!("Couldn't open checksums file: {}", e)))
	}
}

pub fn checksum_directory(sources: Vec<String>, source_root: &PathBuf)
		-> HashMap<String, String> {
	let mut checksums : HashMap<String, String> = HashMap::new();
	//TODO: Make this runtime-swappable
	let mut sha1 = Sha1::new();
	//NOTE: Consider making this runtime-configurable? 
	let mut buf = [0u8; 1<<20];
	for source in sources {
		let mut source_path = source_root.clone();
		source_path.push(source);
		for entry in WalkDir::new(&source_path).into_iter().filter_map(|e| e.ok()) {
			let path = entry.path();
			if !path.is_file() {
				trace!("Skipping {} (not a file)", path.display());
				continue
			}
			let open_result = File::open(path);
			match open_result {
				Ok(mut file) => {
					let mut read_len: usize = 1;
					while read_len > 0 {
						read_len = file.read(&mut buf).unwrap();
						sha1.input(&buf[0 .. read_len]);
					}
					let key = path.strip_prefix(&source_root)
						.and_then(|p| Ok(p.to_str().unwrap().to_string()))
						.unwrap_or(path.to_str().unwrap().to_string());
					let value = sha1.result_str().to_string();
					trace!("Current version checksum: {}\t{}", key, value);
					checksums.insert(key, value);
					sha1.reset();
				},
				Err(e) => {
					trace!("Skipping {} ({})", path.display(), e);
					continue
				}
			}
		}
	}
	checksums.shrink_to_fit();
	checksums
}

pub fn save_checksums(checksums: &HashMap<String, String>, fname:&str)
		-> Result<(), MainError> {
	match File::create(fname) {
		Ok(mut file) => {
			for (key, value) in checksums {
				try!(file.write_all(
					&(format!("{}\t{}\n", value, key).into_bytes()))
					.or_else(|e| Err(MainError::OtherError(
						format!("Error writing to checksum file {}: {}", fname, e)))));
			}
			trace!("Wrote {} current version checksums to {}...",
				checksums.len(), fname);
			Ok(())
		},
		Err(e) => Err(MainError::OtherError(
			format!("Error creating checksum file {}: {}", fname, e)))
	}
}

pub fn write_archive(
		new_checksums: &HashMap<String, String>,
		old_checksums: &HashMap<String, String>,
		source_root: &PathBuf,
		destination: &str)
		-> Result<(), MainError> {
	match File::create(destination) {
		Ok(file) => {
			//TODO: We probably don't always want to gzip this.
			let mut archive = Builder::new(GzEncoder::new(file, Compression::Best));
			for (fname, hash) in new_checksums {
				let old_hash = old_checksums.get(fname);
				if old_hash.map_or(true, |h| h != hash) {
					trace!("Mismatched hashes, archiving: {}\told: {}\tnew: {}",
						fname, old_hash.unwrap_or(&"<none>".to_string()), hash);
					let mut full_fname = source_root.clone();
					full_fname.push(fname);
					archive.append_file(fname, &mut File::open(full_fname).unwrap()).unwrap();
				} else {
					trace!("Matched hashes, not archiving: {}\t{}", fname, hash);
				}
			}
			Ok(())
		},
		Err(e) => Err(MainError::OtherError(
			format!("Error creating target file {}: {}", destination, e)))
	}
}
