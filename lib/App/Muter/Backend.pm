package App::Muter::Backend;
# ABSTRACT: App::Muter::Backend - a backend for muter

use strict;
use warnings;

=method $class->new($args, %opts)

Create a new backend.

$args is an arrayref of arguments provided to the chain.  Currently only the
first argument is considered, and it will typically be a variant of the main
algorithm (e.g. I<lower> for lowercase).

%opts is a set of additional parameters.  The I<transform> value is set to
either I<encode> for encoding or I<decode> for decoding.

Returns the new object.

=cut

sub new {
    my ($class, $args, %opts) = @_;
    $class = ref($class) || $class;
    my $self = {args => $args, options => \%opts, method => $opts{transform}};
    bless $self, $class;
    $self->{m_process} = $self->can($opts{transform});
    $self->{m_final}   = $self->can("$opts{transform}_final");
    return $self;
}

=method $class->metadata

Get metadata about this class.

Returns a hashref containing the metadata about this backend.  The following
keys are defined:

=over 4

=item name

The name of this backend.  This should be a lowercase string and is the
identifier used in the chain.

=item args

A hashref mapping possible arguments to the transform to a human-readable
description.

=back

=cut

sub metadata {
    my ($class) = @_;
    my $name = lc(ref $class || $class);
    $name =~ s/^.*:://;
    return {name => $name};
}

=method $self->process($data)

Process a chunk of data.  Returns the processed chunk.  Note that for buffering
reasons, the data returned may be larger or smaller than the original data
passed in.

=cut

sub process {
    my ($self, $data) = @_;
    my $func = $self->{m_process};
    return $self->$func($data);
}

=method $self->final($data)

Process the final chunk of data.  Returns the processed chunk.  Note that for
buffering reasons, the data returned may be larger or smaller than the original
data passed in.

Calling this function is obligatory.  If all actual data has been passed to the
process function, this function can simply be called with the empty string.

=cut

sub final {
    my ($self, $data) = @_;
    my $func = $self->{m_final};
    return $self->$func($data);
}

sub decode {
    my $self = shift;
    my $name = $self->metadata->{name};
    die "The $name technique doesn't have an inverse transformation.\n";
}

1;
