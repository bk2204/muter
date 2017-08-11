package App::Muter::Backend::Uuencode;
# ABSTRACT: a uuencode transform for App::Muter

use strict;
use warnings;

our @ISA = qw/App::Muter::Backend::Chunked/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new(
        $args, %opts,
        enchunksize => 45,
        dechunksize => 62,
    );
    return $self;
}

sub encode_chunk {    ## no critic(RequireArgUnpacking)
    my ($self, $data) = @_;
    return pack('u', $data);
}

sub encode_final {
    my ($self, $data) = @_;
    return $self->SUPER::encode_final($data) . "`\n";
}

sub decode_chunk {
    my ($self, $data) = @_;
    return '' unless length $data;
    return unpack('u', $data);
}

App::Muter::Registry->instance->register(__PACKAGE__);

1;
