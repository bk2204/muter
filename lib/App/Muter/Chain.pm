package App::Muter::Chain;
# ABSTRACT: main programmatic interface to muter

use strict;
use warnings;

use List::Util ();

=head1 SYNOPSIS

    App::Muter::Registry->instance->load_backends();
    my $chain = App::Muter::Chain->new($chain);
    while (<$fh>) {
        print $chain->process($_);
    }
    print $chain->final('');

=head1 DESCRIPTION

This is the main programmatic (Perl) interface to muter.  It takes an arbitrary
chain and processes data incrementally, in whatever size chunks it's given.

=method $class->new($chain)

Create a new chain object using the specified chain, which is identical to the
argument to muter's B<-c> option.

=cut

sub new {
    my ($class, $chain) = @_;
    $class = ref($class) || $class;
    my $self = bless {}, $class;
    $self->{chain} = [$self->_instantiate($self->_parse_chain($chain))];
    return $self;
}

=method $self->process($data)

Process a chunk of data.  Chunks need not be all the same size.  Returns the
transformed data, which may be longer or shorter than the input data.

=cut

sub process {
    my ($self, $data) = @_;

    return List::Util::reduce { $b->process($a) } $data, @{$self->{chain}};
}

=method $self->final($data)

Process the final chunk of data.  If all the data has already been sent via the
I<process> method, simply pass an empty string.

=cut

sub final {
    my ($self, $data) = @_;

    return List::Util::reduce { $b->final($a) } $data, @{$self->{chain}};
}

sub _chain_entry {
    my ($item) = @_;
    if ($item =~ /^(-?)(\w+)(?:\(([^)]+)\))?$/) {
        return {
            name   => $2,
            method => ($1 ? 'decode' : 'encode'),
            args   => ($3 ? [split /,/, $3] : []),
        };
    }
    elsif ($item =~ /^(-?)(\w+),([^)]+)$/) {
        return {
            name   => $2,
            method => ($1 ? 'decode' : 'encode'),
            args   => ($3 ? [split /,/, $3] : []),
        };
    }
    else {
        die "Chain entry $item is invalid";
    }
}

sub _parse_chain {
    my (undef, $chain) = @_;
    my @items = split /:/, $chain;
    return map { _chain_entry($_) } @items;
}

sub _instantiate {
    my (undef, @entries) = @_;
    my $registry = App::Muter::Registry->instance;
    return map {
        my $class = $registry->info($_->{name})->{class};
        $class->new($_->{args}, transform => $_->{method});
    } @entries;
}

1;
