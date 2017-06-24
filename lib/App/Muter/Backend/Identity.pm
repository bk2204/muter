package App::Muter::Backend::Identity;
# ABSTRACT: an identity transform for App::Muter

use strict;
use warnings;

our @ISA = qw/App::Muter::Backend/;

sub encode {    ## no critic(RequireArgUnpacking)
    return $_[1];
}

{
    no warnings 'once';    ## no critic(ProhibitNoWarnings)

    *decode       = \&encode;
    *encode_final = \&encode;
    *decode_final = \&encode;
}

App::Muter::Registry->instance->register(__PACKAGE__);

1;
