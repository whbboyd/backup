#!/usr/bin/python

import argparse
import hashlib
import os
import subprocess
import sys
import tempfile


def main(argv):
	ap = argparse.ArgumentParser()
	ap.add_argument('sources', nargs='+', help='Source paths.')
	ap.add_argument('destination', help='Target destination. If this is an '
		'extant directory, a uniquely-named backup file will be placed in it; '
		'otherwise, that filename will be used for the backup.')
	ap.add_argument('-r', '--source-root', metavar='DIR', help='The root of '
		'the backup. This should be a prefix to the source path. This prefix '
		'will be removed from file paths when constructing the target file.')
	ap.add_argument('-c', '--old-checksums', metavar='FILE', help='Checksums '
		'to compare against. If empty, all target files will be backed up; '
		'otherwise, all non-matching and new files will be backed up. The '
		'format should be filename, whitespace, hexadecimal checksum (as '
		'output by e.g. md5sum).')
	ap.add_argument('-n', '--new-checksums', metavar='FILE', help='File to '
		'which to write checksums. The file will be overwritten with '
		'filename, whitespace, hexadecimal checksum (as output by e.g. '
		'md5sum).')
	ap.add_argument('-x', '--hash-algo', metavar='ALGORITHM', default='md5',
		help='Checksumming algorithm to use. Available options are '
		'platform-dependent. This option affects the interpretation of '
		'checksums in the old-checksums and new-checksums files. Default is '
		'%(default)s.')
	ap.add_argument('-d', '--dry-run', action='store_true',
		help='Print backup command instead of running it.')
	args = ap.parse_args()

	if args.source_root:
		os.chdir(args.source_root)

	# Load extant checksums
	old_checksums = {}
	if args.old_checksums:
		with open(args.old_checksums) as f:
			for line in f:
				checksum, fname = line.split()
				old_checksums[fname] = checksum.decode('hex')

	hash_algo = hashlib.new(args.hash_algo)

	# Walk the source directory
	diffs = []
	new_checksums = {}
	for source in args.sources:
		for path, dirs, files in _walk(source):
			for fname in files:
				fname = os.path.join(path, fname)
				# If we have specified extant checksums or requested new ones,
				# checksum the files and keep only differences.
				if len(old_checksums) > 0 or args.new_checksums:
					new_checksums[fname] = hash_file(hash_algo, fname)
					if (fname not in old_checksums or
						old_checksums[fname] != new_checksums[fname]):
						diffs.append(fname)
				else:
					diffs.append(fname)

	# Write out checksums
	if args.new_checksums:
		with open(args.new_checksums, 'w') as f:
			[f.write('%s\t%s\n' % (new_checksums[fname].encode('hex'), fname)) for fname in new_checksums]

	# Package up source files
	if os.path.isdir(args.destination):
		args.destination = tempfile.NamedTemporaryFile(
			dir=args.destination, suffix='.tar.gz', delete=False
			).name
	command = ['tar']
	command.append('-czf')
	command.append(args.destination)
	command.extend(diffs)

	if args.dry_run:
		print(' '.join(command))
	else:
		subprocess.check_call(command)


def _walk(path):
	if os.path.isfile(path):
		return [(os.path.dirname(path), [], [os.path.basename(path)])]
	elif os.path.isdir(path):
		return os.walk(path)
	else:
		raise OSError('Path %s is neither file nor directory!' % path)


def hash_file(hasher, fname):
	hasher = hasher.copy()
	f = open(fname)
	filechunk = f.read(1024)
	while filechunk:
		hasher.update(filechunk)
		filechunk = f.read(1024)
	return hasher.digest()


if __name__=='__main__':
	main(sys.argv[1:])

