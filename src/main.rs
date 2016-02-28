extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use rustc_serialize::hex::FromHex;
use std::collections::HashMap;
use std::env::current_dir;
use std::error::Error;
use std::io::BufReader;
use std::io::BufRead;
use std::io::Write;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

const USAGE: &'static str = "
Checksum-based incremental backup utility using standard tools and formats.

This program checksums the files in the target directory, optionally compares
them to a set of preexisting checksums, collects changed files in a tarball,
and writes the new checksums;

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
                e.g. md5sum).
  -x <algorithm>, --hash-algorithm <algorithm>
                Checksumming algorithm to use. Available options are platform-
                dependent. This option affects the interpretation of checksums
                in the old-checksums and new-checksums files. [default: md5]
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
		.and_then(|d| d.decode())
		.or_else(|e| Err(MainError::DocoptError(e))));

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

	// Load extant checksums
	let mut old_checksums : HashMap<String, String> = HashMap::new();
	match args.flag_old_checksums {
		Some(f) => {
			let checksums_file = File::open(f).unwrap_or_else(|e| {
					println!("Couldn't open checksums file: {}", e);
					exit(4);
				});
			let checksums_reader = BufReader::new(&checksums_file);
			for line in checksums_reader.lines() {
				match line {
					Ok(l) => {
						let mut fields = l.split_whitespace();
						let checksum = match fields.next() {
							Some(f) => f,
							None => { continue; }
						};
						let filename = match fields.next() {
							Some(f) => f,
							None => { continue; }
						};
						old_checksums.insert(filename.to_string(), checksum.to_string());
					},
					Err(_) => continue
				}
			}
			println!("{:?}", checksums_file);
		},
		_ => (),
	}


println!("{:?}", old_checksums);
println!("{:?}",source_root);
	return Ok(());
}


fn main() {
	match do_main() {
		Err(e) => match e {
			MainError::OtherError(s) => {
				writeln!(&mut std::io::stderr(), "{}", s).unwrap();
				exit(3);
			},
			MainError::DocoptError(e) => e.exit(),
		},
		_ => (),
	}
}
