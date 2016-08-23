#!/usr/bin/env perl

use strict;
use warnings;

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

use Digest::SHA;
use IO::Scalar;
use App::Muter;

my @patterns = (
    "\x00A7\x80",
    "©",
    "aa?",
    "bc>",
    "test/?^t~",
    "This is\aa sentence\nwith control\bcharacters in it.",
    "\x01\x23\x45\x67\x89\xab\xcd\xef",
    "\xef\xbb\xbf",
    q{"Hello, ol' New Jersey! <:>"},
    # Triggered a bug in vis.
    "\x86\xe8\x9c\xd2\x09\x12\x7f\x53\xf7\xb8\x92\x0c",
    # Triggered a bug in xml.
    "&abc",
);

my @random_patterns = map { byte_pattern($_) } 0 .. 20;

my @techniques = qw/
    ascii85
    base16
    base32
    base32hex
    base64
    base64(mime)
    hex
    quotedprintable
    uri
    uri(lower)
    url64
    xml
    xml(hex)
    xml(html)
    vis
    vis(cstyle)
    vis(octal)
    vis(white)
    /;

foreach my $tech (@techniques) {
    subtest "Technique $tech" => sub {
        my $num = 0;
        foreach my $input (@patterns) {
            test_run_pattern($tech, $input, "fixed pattern " . $num++);
        }
        $num = 0;
        foreach my $input (@random_patterns) {
            is(length($input), $num, "byte pattern is of proper length");
            test_run_pattern($tech, $input, "byte pattern " . $num++);
        }
    };
}

done_testing;

sub test_run_pattern {
    my ($chain, $input, $desc) = @_;

    test_run_chain("$chain:-$chain", $input, $input, "$desc") or
        diag explain run_chain($chain, $input);
    if ($chain =~ /^([^(]+)\(.*\)/) {
        test_run_chain("$chain:-$1", $input, $input, "$desc (base)");
    }
    return;
}

sub test_run_chain {
    my ($chain, $input, $output, $desc) = @_;

    return subtest $desc => sub {
        is(run_chain($chain, $input, 1),   $output, "$desc (1-byte chunks)");
        is(run_chain($chain, $input, 2),   $output, "$desc (2-byte chunks)");
        is(run_chain($chain, $input, 3),   $output, "$desc (3-byte chunks)");
        is(run_chain($chain, $input, 4),   $output, "$desc (4-byte chunks)");
        is(run_chain($chain, $input, 16),  $output, "$desc (16-byte chunks)");
        is(run_chain($chain, $input, 512), $output, "$desc (512-byte chunks)");
    };
}

sub run_chain {
    my ($chain, $input, $blocksize) = @_;
    my $output = '';
    my $ifh    = IO::Scalar->new(\$input);
    my $ofh    = IO::Scalar->new(\$output);

    App::Muter::Main::run_chain($chain, [$ifh], $ofh, $blocksize);

    return $output;
}

# These are "random" patterns of a given length.  They're designed to be
# reproducible, but handle a variety of byte patterns.
sub byte_pattern {
    my ($len) = @_;
    my $s     = '';
    my $count = 0;
    while (length($s) < $len) {
        $s .= Digest::SHA::sha512(pack("NN", $len, $count));
    }
    return substr($s, 0, $len);
}
