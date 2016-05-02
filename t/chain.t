#!/usr/bin/env perl

use strict;
use warnings;

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

use IO::Scalar;
use Muter;

test_run_pattern('hex', "\x00A7\x80", '00413780', 'basic hex');

test_run_pattern('base64', '', '', 'base64 empty data');
test_run_pattern('base64', 'abcdefg', 'YWJjZGVmZw==', 'base64 pattern 1');
test_run_pattern('base64', 'hij', 'aGlq', 'base64 pattern 2');
test_run_pattern('base64', "klmn\n", 'a2xtbgo=', 'base64 pattern 3');
test_run_pattern('base64', 'aa?', 'YWE/', 'base64 pattern 4');
test_run_pattern('base64', 'bc>', 'YmM+', 'base64 pattern 5');

test_run_pattern('url64', '', '', 'url64 empty data');
test_run_pattern('url64', 'abcdefg', 'YWJjZGVmZw', 'url64 pattern 1');
test_run_pattern('url64', 'hij', 'aGlq', 'url64 pattern 2');
test_run_pattern('url64', "klmn\n", 'a2xtbgo', 'url64 pattern 3');
test_run_pattern('url64', 'aa?', 'YWE_', 'url64 pattern 4');
test_run_pattern('url64', 'bc>', 'YmM-', 'url64 pattern 5');

test_run_chain('-hex:base64', '00413780', 'AEE3gA==', 'simple chain');

test_run_chain('-hex:hash(sha256):url64', '616263',
	'ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0',
	'simple chain with consuming filter');

done_testing;

sub test_run_pattern {
	my ($chain, $input, $output, $desc) = @_;

	subtest $desc => sub {
		test_run_chain($chain, $input, $output, "$desc (encoding)");
		test_run_chain("-$chain", $output, $input, "$desc (decoding)");
	};
	return;
}

sub test_run_chain {
	my ($chain, $input, $output, $desc) = @_;

	subtest $desc => sub {
		is(run_chain($chain, $input, 1), $output, "$desc (1-byte chunks)");
		is(run_chain($chain, $input, 2), $output, "$desc (2-byte chunks)");
		is(run_chain($chain, $input, 3), $output, "$desc (3-byte chunks)");
		is(run_chain($chain, $input, 4), $output, "$desc (4-byte chunks)");
		is(run_chain($chain, $input, 16), $output, "$desc (16-byte chunks)");
		is(run_chain($chain, $input, 512), $output, "$desc (512-byte chunks)");
	};
	return;
}

sub run_chain {
	my ($chain, $input, $blocksize) = @_;
	my $output = '';
	my $ifh = IO::Scalar->new(\$input);
	my $ofh = IO::Scalar->new(\$output);

	Muter::Main::run_chain($chain, [$ifh], $ofh, $blocksize);

	return $output;
}
