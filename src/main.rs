extern crate crypto;
extern crate docopt;
extern crate env_logger;
extern crate flate2;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate tar;
extern crate walkdir;

use crypto::digest::Digest;
use crypto::sha1::Sha1;
use docopt::Docopt;
use env_logger::LogBuilder;
use flate2::Compression;
use flate2::write::GzEncoder;
use log::{LogLevel, LogRecord, SetLoggerError};
use std::collections::HashMap;
use std::env;
use std::env::current_dir;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::exit;
use tar::Builder;
use walkdir::WalkDir;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

const USAGE: &'static str = "
Checksum-based incremental backup utility using standard tools and formats.

This program checksums the files in the target directory, optionally compares
them to a set of preexisting checksums, collects changed files in a tarball,
and writes the new checksums.

Usage:
  backup [options] [--] <source>... <destination>
  backup (-h | --help)
  backup --version

Options:
  -h --help     Show this screen.
  --version     Show version.
  -r <dir>, --source-root <dir>
                The root of the backup. This should be a prefix to the source
                path. This prefix will be removed from file paths when
                constructing the destination file. Default is the current
                working directory.
  -c <file>, --old-checksums <file>
                Checksums to compare against. If not specified, all target
                files will be backed up; otherwise, all non-matching and new
                files will be backed up. The format should be filename,
                whitespace, hexadecimal checksum (as output by e.g. md5sum).
  -n <file>, --new-checksums <file>
                File to which to write checksums. The file will be overwritten
                by filename, whitespace, hexadecimal checksum (as output by
                e.g. sha1sum).
  -x <algorithm>, --hash-algorithm <algorithm>
                Checksumming algorithm to use. Available options are platform-
                dependent. This option affects the interpretation of checksums
                in the old-checksums and new-checksums files. BUG: At the
                moment, this option is ignored. [default: sha1]
  -d, --dry-run
                Don't actually write any files, print what would be done
                instead.
";

#[derive(Debug,RustcDecodable)]
struct Args {
	arg_source: Vec<String>,
	arg_destination: String,
	flag_source_root: Option<String>,
	flag_old_checksums: Option<String>,
	flag_new_checksums: Option<String>,
	flag_hash_algorithm: String,
	flag_dry_run: bool,
}

enum MainError {
	DocoptError(docopt::Error),
	OtherError(String),
}

fn do_main() -> Result<(),MainError> {

	// Parse commandline arguments
	let args : Args = try!(Docopt::new(USAGE)
		.and_then(|d| Ok(d.version(VERSION.and_then(|v| Some(v.to_string())))))
		.and_then(|d| d.decode())
		.or_else(|e| Err(MainError::DocoptError(e))));

	if args.flag_dry_run {
		info!("[dry-run] Dry-run specified, not writing anything.");
	}

	// Figure out source root. If not specified on the commandline, it's the
	// current directory.
	let source_root = try!(args.flag_source_root
		.ok_or(())
		.and_then(|d| Ok(PathBuf::from(d)))
		.or_else(|_| current_dir()
			.or_else(|e| Err(MainError::OtherError(
				format!("Couldn't use current directory as source root: {}", e)
				.to_string()
			)))
		)
	);
	if !source_root.is_dir() {
		return Err(MainError::OtherError("Source root path is not a directory".to_string()));
	}
	debug!("Using {} as source directory...", source_root.as_path().display());

	// Load extant checksums
	let mut old_checksums : HashMap<String, String> = HashMap::new();
	match args.flag_old_checksums {
		Some(f) => {
			debug!("Loading previous version checksums from {}...", f);
			let checksums_file = File::open(f).unwrap_or_else(|e| {
					error!("Couldn't open checksums file: {}", e);
					exit(4);
				});
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
						old_checksums.insert(filename.to_string(), checksum.to_string());
					},
					_ => continue
				}
			}
		},
		_ => (),
	}
	debug!("Loaded {} previous version checksums...", old_checksums.len());

	// Walk specified files in the source directory and checksum files
	debug!("Walking/checking source directory...");
	let mut new_checksums : HashMap<String, String> = HashMap::new();
	let mut sha1 = Sha1::new();
	let mut buf = [0u8; 1048576];
	for source in args.arg_source {
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
					new_checksums.insert(key, value);
					sha1.reset();
				},
				Err(e) => {
					trace!("Skipping {} ({})", path.display(), e);
					continue
				}
			}
		}
	}

	// Write new checksums
	try!(match (args.flag_dry_run, args.flag_new_checksums) {
		(false, Some(fname)) => {
			debug!("Writing current version checksums...");
			match File::create(&fname) {
				Ok(mut file) => {
					for (key, value) in &new_checksums {
						try!(file.write_all(
							&(format!("{}\t{}\n", value, key).into_bytes()))
							.or_else(|e| Err(MainError::OtherError(
								format!("Error writing to checksum file {}: {}", fname, e)))));
					}
					trace!("Wrote {} current version checksums to {}...",
						new_checksums.len(), fname);
					Ok(())
				},
				Err(e) => Err(MainError::OtherError(
					format!("Error creating checksum file {}: {}", fname, e)))
			}
		},
		(true, Some(fname)) => {
			info!("[dry-run] Checksums would be written to {}", fname);
			Ok(())
		},
		_ => {
			debug!(concat!("No current version checksum file specified, ",
				"not writing current version checksums..."));
			Ok(())
		}
	});

	// Package altered files in source root into a tarball and write it to the destination
	if !args.flag_dry_run {
		debug!("Writing backup file to {}...", args.arg_destination);
		try!(match File::create(&args.arg_destination) {
			Ok(file) => {
				//TODO: We probably don't always want to gzip this.
				let mut archive = Builder::new(GzEncoder::new(file, Compression::Best));
				for (fname, hash) in &new_checksums {
					let old_hash = &old_checksums.get(fname);
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
				format!("Error creating target file {}: {}", args.arg_destination, e)))
		})
	} else {
		info!("[dry-run] Output file would be written to {}", args.arg_destination);
		info!("[dry-run] Output would contain the following files:");
		for (fname, hash) in &new_checksums {
			let old_hash = &old_checksums.get(fname);
			if old_hash.is_none() || old_hash.unwrap() != hash {
				info!("[dry-run]\t{}\t{}", fname, hash);
			}
		}
	}

	debug!("Done!");
	Ok(())
}

fn main() {
	if let Err(e) = init_log() {
		writeln!(&mut std::io::stderr(), "Could not set logger: {}", e).unwrap();
		exit(5);
	}

	if let Err(e) = do_main() {
		match e {
			MainError::OtherError(s) => {
				error!("{}", s);
				exit(3);
			},
			MainError::DocoptError(e) => e.exit(),
		}
	}
}

fn init_log() -> Result<(), SetLoggerError> {
	let mut builder = LogBuilder::new();
	builder.format(|record: &LogRecord| {
		format!("[{} {}:{}] [{}] {}",
			record.location().module_path(),
			record.location().file(),
			record.location().line(),
			record.level(),
			record.args()) } );
	match env::var("RUST_LOG") {
		Ok(log_str) => { builder.parse(&log_str); },
		Err(_) => { builder.filter(None, LogLevel::Info.to_log_level_filter()); }
	}
	try!(builder.init());
	Ok(())
}
