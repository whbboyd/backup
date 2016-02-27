extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;

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
                constructing the destination file.
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
	flag_hash_algorithm: Option<String>,
	flag_dry_run: bool,
}

fn main() {
	println!("Hello, world!");
	let args: Args = Docopt::new(USAGE)
		.and_then(|d| d.decode())
		.unwrap_or_else(|e| e.exit());
	println!("{:?}", args);
}
