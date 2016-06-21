#!/usr/bin/env perl

use strict;
use warnings;

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

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
);

my @techniques = qw/
    hex
    base64
    url64
    uri
    uri(lower)
    base32
    base32hex
    xml
    xml(hex)
    xml(html)
    quotedprintable
    /;

foreach my $tech (@techniques) {
    subtest "Technique $tech" => sub {
        my $num = 0;
        foreach my $input (@patterns) {
            test_run_pattern($tech, $input, "pattern " . $num++);
        }
    };
}

done_testing;

sub test_run_pattern {
    my ($chain, $input, $desc) = @_;

    test_run_chain("$chain:-$chain", $input, $input, "$desc");
    if ($chain =~ /^([^(]+)\(.*\)/) {
        test_run_chain("$chain:-$1", $input, $input, "$desc (base)");
    }
    return;
}

sub test_run_chain {
    my ($chain, $input, $output, $desc) = @_;

    subtest $desc => sub {
        is(run_chain($chain, $input, 1),   $output, "$desc (1-byte chunks)");
        is(run_chain($chain, $input, 2),   $output, "$desc (2-byte chunks)");
        is(run_chain($chain, $input, 3),   $output, "$desc (3-byte chunks)");
        is(run_chain($chain, $input, 4),   $output, "$desc (4-byte chunks)");
        is(run_chain($chain, $input, 16),  $output, "$desc (16-byte chunks)");
        is(run_chain($chain, $input, 512), $output, "$desc (512-byte chunks)");
    };
    return;
}

sub run_chain {
    my ($chain, $input, $blocksize) = @_;
    my $output = '';
    my $ifh    = IO::Scalar->new(\$input);
    my $ofh    = IO::Scalar->new(\$output);

    App::Muter::Main::run_chain($chain, [$ifh], $ofh, $blocksize);

    return $output;
}
