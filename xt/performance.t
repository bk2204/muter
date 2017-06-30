#!/usr/bin/env perl

use strict;
use warnings;

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

use IO::File;
use IO::Scalar;
use App::Muter;
use Digest::SHA;
use Time::HiRes;

my @backends = App::Muter::Registry->instance->backends;

my $seed = $ENV{'TEST_SEED'} // time;
diag "Seed is $seed.";
diag "Generating pseudo-random data.";
my $testdata = generate_input($seed);

test_backend($_, $testdata) for grep { $_ ne 'hash' } @backends;

done_testing;

sub test_backend {
    my ($backend, $input) = @_;
    my $start = [Time::HiRes::gettimeofday()];
    my $out   = eval { run_chain($backend, $input, 512) };
    my $end   = [Time::HiRes::gettimeofday()];
    cmp_ok(
        length $out, '>=',
        2 * 1024 * 1024,
        "backend $backend encoded at least 2 MiB"
    );
    my $elapsed = Time::HiRes::tv_interval($start, $end);
    diag "backend $backend took $elapsed seconds";
    return;
}

sub run_chain {
    my ($chain, $input, $blocksize) = @_;
    my $output = '';
    my $ifh    = IO::Scalar->new(\$input);
    my $ofh    = IO::Scalar->new(\$output);

    App::Muter::Main::run_chain("$chain:-$chain", [$ifh], $ofh, $blocksize);

    return $output;
}

sub generate_input {
    return
        join('',
        map { Digest::SHA::sha512(pack('pNN', $seed, $_)) } 1 .. 32768);
}
