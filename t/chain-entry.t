#!/usr/bin/env perl

use strict;
use warnings;

use FindBin;

use lib "$FindBin::Bin/../lib";

use Test::More;

use App::Muter;

my @examples = (
    {
        chain  => 'hex',
        parsed => [
            {
                name   => 'hex',
                method => 'encode',
                args   => [],
            }
        ]
    }, {
        chain  => '-hex',
        parsed => [
            {
                name   => 'hex',
                method => 'decode',
                args   => [],
            }
        ]
    }, {
        chain  => '-hex:base64',
        parsed => [
            {
                name   => 'hex',
                method => 'decode',
                args   => [],
            }, {
                name   => 'base64',
                method => 'encode',
                args   => [],
            }
        ]
    }, {
        chain  => '-hex(upper):xml(html):hash(sha256)',
        parsed => [
            {
                name   => 'hex',
                method => 'decode',
                args   => ['upper'],
            }, {
                name   => 'xml',
                method => 'encode',
                args   => ['html'],
            }, {
                name   => 'hash',
                method => 'encode',
                args   => ['sha256'],
            }
        ]
    },
);

foreach my $test (@examples) {
    is_deeply(
        $test->{parsed},
        [App::Muter::Chain->_parse_chain($test->{chain})],
        "$test->{chain} parses properly"
    );
}

done_testing();
