#!/usr/bin/env perl

use strict;
use warnings;
use experimental 'switch';

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

use IO::File;
use IO::Scalar;
use App::Muter;

App::Muter::Registry->instance->load_backends();

my $testdir = "$FindBin::Bin/tests";

opendir(my $dh, $testdir) or die;
my @files = sort grep { /^[0-9a-f]+/ } readdir $dh;
closedir($dh);

foreach my $test (@files) {
    my $file  = "$testdir/$test";
    my $state = 'input';
    my $count = 0;
    my %entries;
    my $fh = IO::File->new($file, '<') or die;
    while (my $line = <$fh>) {
        $entries{$state} //= {};
        for ($line) {
            when (/^Flags:\s+(.*)$/) {
                set_flags($entries{$state}, $line);
            }
            when (/^Input:\s(.*)$/) {
                $entries{$state}{data} = $1;
            }
            when (/^Output:\s(.*)$/) {
                $entries{$state}{data} = $1;
            }
            when (/^Chain:\s(.*)$/) {
                $state = $count++;
                $entries{$state}{chain} = $1;
            }
            when (/^\s(.*)/) {
                $entries{$state}{data} .= "\n$1";
            }
        }
    }
    close($fh);
    subtest "Test $test" => sub {
        my $inopts = delete $entries{input};
        my $indata = $inopts->{data};
        foreach my $test (sort keys %entries) {
            my $opts = $entries{$test};
            my $func = $opts->{inverse} ? \&test_run_pattern : \&test_run_chain;
            my $data = $opts->{data};
            $func->($opts->{chain}, $indata, $data, "Chain '$opts->{chain}'");
        }
    };
}

done_testing;

sub set_flags {
    my ($entry, $flags) = @_;
    my %flags = map { $_ => 1 } split /\s+/, $flags;
    $entry->{inverse}  = 1    if $flags{inverse};
    return;
}

sub test_run_pattern {
    my ($chain, $input, $output, $desc) = @_;

    subtest $desc => sub {
        test_run_chain($chain,    $input,  $output, "$desc (encoding)");
        test_run_chain("-$chain", $output, $input,  "$desc (decoding)");
    };
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
