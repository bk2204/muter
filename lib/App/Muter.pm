#!/usr/bin/env perl
# muter - a data transformation tool
#
# Copyright © 2016 brian m. carlson
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.
package App::Muter::Main;

require 5.010001;

use strict;
use warnings;

use Getopt::Long ();
use IO::Handle   ();
use IO::File     ();

sub script {
    my (@args) = @_;

    my $chain = '';
    my $help;
    Getopt::Long::GetOptionsFromArray(
        \@args,
        'chain|c=s' => \$chain,
        'help'      => \$help
        ) or
        return usage(1);

    return usage(0) if $help;
    return usage(1) unless $chain;

    run_chain($chain, load_handles(\@args), \*STDOUT);

    return 0;
}

sub load_handles {
    my ($files) = @_;
    my @handles = map { IO::File->new($_, 'r') } @$files;
    @handles = (\*STDIN) unless @handles;
    return \@handles;
}

sub run_chain {
    my ($chain, $handles, $stdout, $blocksize) = @_;

    $chain = App::Muter::Chain->new($chain);
    $blocksize ||= 512;

    foreach my $io (@$handles) {
        $io->binmode(1);
        while ($io->read(my $buf, $blocksize)) {
            $stdout->print($chain->process($buf));
        }
    }
    $stdout->print($chain->final(''));
    return;
}

sub usage {
    my ($ret) = @_;
    my $fh = $ret ? \*STDERR : \*STDOUT;
    $fh->print(<<'EOM');
muter -c CHAIN | --chain CHAIN [FILES...]

Modify the bytes in the concatentation of FILES (or standard input) by using the
specification in CHAIN.

CHAIN is a colon-separated list of encoding transform.  A transform can be
prefixed with - to reverse it (if possible).  A transform can be followed by a
parenthesized argument as well.

For example, '-hex:hash(sha256):base64' decodes a hex-encoded string, hashes it
with SHA-256, and converts the result to base64.

The following transforms are available:
EOM
    my $reg = App::Muter::Registry->instance;
    foreach my $name ($reg->backends) {
        $fh->print("  $name\n");
        my $meta = $reg->info($name);
        if ($meta->{args} && ref($meta->{args}) eq 'HASH') {
            $fh->print("    ", join(', ', sort keys %{$meta->{args}}), "\n");
        }
    }
    return $ret;
}

## no critic(ProhibitMultiplePackages)
package App::Muter::Chain;

use List::Util ();

sub new {
    my ($class, $chain) = @_;
    $class = ref($class) || $class;
    my $self = bless {}, $class;
    $self->{chain} = [$self->_instantiate($self->_parse_chain($chain))];
    return $self;
}

sub process {
    my ($self, $data) = @_;

    return List::Util::reduce { $b->process($a) } $data, @{$self->{chain}};
}

sub final {
    my ($self, $data) = @_;

    return List::Util::reduce { $b->final($a) } $data, @{$self->{chain}};
}

sub _chain_entry {
    my ($item) = @_;
    $item =~ /^(-?)(\w+)(?:\(([^)]+)\))?$/ or
        die "Chain entry '$item' is invalid";
    return {
        name   => $2,
        method => ($1 ? 'decode' : 'encode'),
        args   => ($3 ? [split /,/, $3] : []),
    };
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

package App::Muter::Registry;

my $instance;

sub instance {
    my $class = shift;
    $class = ref($class) || $class;
    my $self = {names => {}};
    return $instance ||= bless $self, $class;
}

sub register {
    my ($self, $class) = @_;
    my $info = $class->metadata;
    $self->{names}{$info->{name}} = {%$info, class => $class};
    return 1;
}

sub info {
    my ($self, $name) = @_;
    my $info = $self->{names}{$name};
    die "No such transform '$name'" unless $info;
    return $info;
}

sub backends {
    my ($self) = @_;
    my @backends = sort keys %{$self->{names}};
    return @backends;
}

package App::Muter::Backend;

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

=back

=cut

sub metadata {
    my ($class) = @_;
    my $name = lc(ref $class || $class);
    $name =~ s/^.*:://;
    return {name => $name};
}

sub process {
    my ($self, $data) = @_;
    my $func = $self->{m_process};
    return $self->$func($data);
}

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

package App::Muter::Backend::Chunked;

use parent qw/-norequire App::Muter::Backend/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new($args, %opts);
    $self->{chunk}       = '';
    $self->{enchunksize} = $opts{enchunksize} || $opts{chunksize};
    $self->{dechunksize} = $opts{dechunksize} || $opts{chunksize};
    return $self;
}

sub encode {
    my ($self, $data) = @_;
    return $self->_with_chunk($data, $self->{enchunksize}, 'encode_chunk');
}

sub decode {
    my ($self, $data) = @_;
    return $self->_with_chunk($data, $self->{dechunksize}, 'decode_chunk');
}

sub encode_final {
    my ($self, $data) = @_;
    return $self->encode_chunk($self->{chunk} . $data);
}

sub decode_final {
    my ($self, $data) = @_;
    return $self->decode_chunk($self->{chunk} . $data);
}

sub _with_chunk {
    my ($self, $data, $chunksize, $code) = @_;
    my $chunk = $self->{chunk} . $data;
    my $len   = length($chunk);
    my $rem   = $len % $chunksize;
    if ($rem) {
        $self->{chunk} = substr($chunk, -$rem);
        $chunk = substr($chunk, 0, -$rem);
    }
    else {
        $self->{chunk} = '';
    }
    return $self->$code($chunk);
}

package App::Muter::Backend::ChunkedDecode;

use parent qw/-norequire App::Muter::Backend/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new($args, %opts);
    $self->{chunk}  = '';
    $self->{regexp} = $opts{regexp};
    return $self;
}

sub encode {
    my ($self, $data) = @_;
    return $self->encode_chunk($data);
}

sub decode {
    my ($self, $data) = @_;
    $data = $self->{chunk} . $data;
    if ($data =~ $self->{regexp}) {
        $data = $1;
        $self->{chunk} = $2;
    }
    else {
        $self->{chunk} = '';
    }
    return $self->decode_chunk($data);
}

sub encode_final {
    my ($self, $data) = @_;
    return $self->encode_chunk($self->{chunk} . $data);
}

sub decode_final {
    my ($self, $data) = @_;
    return $self->decode_chunk($self->{chunk} . $data);
}

package App::Muter::Backend::Base64;

use MIME::Base64 ();
use parent qw/-norequire App::Muter::Backend::Chunked/;

sub new {
    my ($class, @args) = @_;
    return $class->SUPER::new(@args, enchunksize => 3, dechunksize => 4);
}

sub encode_chunk {
    my (undef, $data) = @_;
    return MIME::Base64::encode($data, '');
}

sub _filter {
    my ($self, $data) = @_;
    $data =~ tr{A-Za-z0-9+/=}{}cd;
    return $data;
}

sub decode {
    my ($self, $data) = @_;
    return $self->SUPER::decode($self->_filter($data));
}

sub decode_chunk {
    my (undef, $data) = @_;
    return MIME::Base64::decode($data);
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::URL64;

use MIME::Base64 ();
use parent qw/-norequire App::Muter::Backend::Base64/;

sub encode_chunk {
    my (undef, $data) = @_;
    return MIME::Base64::encode_base64url($data);
}

sub _filter {
    my (undef, $data) = @_;
    return $data;
}

sub decode_chunk {
    my (undef, $data) = @_;
    return MIME::Base64::decode_base64url($data);
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Hex;

use parent qw/-norequire App::Muter::Backend::Chunked/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new(
        $args, %opts,
        enchunksize => 1,
        dechunksize => 2
    );
    $self->{upper} = 1 if defined $args->[0] && $args->[0] eq 'upper';
    return $self;
}

sub metadata {
    my $self = shift;
    my $meta = $self->SUPER::metadata;
    return {
        %$meta,
        args => {
            upper => 'Use uppercase letters',
            lower => 'Use lowercase letters',
        }
    };
}

sub encode_chunk {
    my ($self, $data) = @_;
    my $result = unpack("H*", $data);
    return uc $result if $self->{upper};
    return $result;
}

sub decode_chunk {
    my (undef, $data) = @_;
    return pack("H*", $data);
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Base16;

use parent qw/-norequire App::Muter::Backend::Hex/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new(['upper'], %opts);
    return $self;
}

sub metadata {
    my $self = shift;
    my $meta = $self->SUPER::metadata;
    delete $meta->{args};
    return $meta;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Base32;

use parent qw/-norequire App::Muter::Backend::Chunked/;

sub new {
    my ($class, @args) = @_;
    my $self = $class->SUPER::new(@args, enchunksize => 5, dechunksize => 8);
    $self->{fmap} = [split //, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'];
    return $self->_initialize;
}

sub _initialize {
    my ($self) = @_;
    my $fmap = $self->{fmap};
    $self->{rmap} = {'=' => 0};
    @{$self->{rmap}}{@$fmap} = keys @$fmap;
    return $self;
}

sub encode_chunk {
    my ($self, $data) = @_;
    my @data   = map { ord } split //, $data;
    my $result = '';
    my $map    = $self->{fmap};
    my $lenmap = [0, 2, 4, 5, 7, 8];
    while (my @chunk = splice(@data, 0, 5)) {
        my $len = @chunk;
        push @chunk, (0, 0, 0, 0);
        my @converted = map { $self->{fmap}[$_ & 0x1f] } (
            $chunk[0] >> 3,
            ($chunk[0] << 2) | ($chunk[1] >> 6),
            ($chunk[1] >> 1),
            ($chunk[1] << 4) | ($chunk[2] >> 4),
            ($chunk[2] << 1) | ($chunk[3] >> 7),
            ($chunk[3] >> 2),
            ($chunk[3] << 3) | ($chunk[4] >> 5),
            $chunk[4]
        );
        my $chunk = substr(join('', @converted), 0, $lenmap->[$len]);
        $chunk = substr($chunk . '======', 0, 8);
        $result .= $chunk;
    }
    return $result;
}

sub decode_chunk {
    my ($self, $data) = @_;
    my $lenmap = [5, 4, undef, 3, 2, undef, 1];
    my $trailing = $data =~ /(=+)$/ ? length $1 : 0;
    my $truncate = $lenmap->[$trailing];
    my $result   = '';
    my @data     = map { $self->{rmap}{$_} } split //, $data;
    use bytes;
    while (my @chunk = splice(@data, 0, 8)) {
        my @converted = (
            ($chunk[0] << 3) | ($chunk[1] >> 2),
            ($chunk[1] << 6) | ($chunk[2] << 1) | ($chunk[3] >> 4),
            ($chunk[3] << 4) | ($chunk[4] >> 1),
            ($chunk[4] << 7) | ($chunk[5] << 2) | ($chunk[6] >> 3),
            ($chunk[6] << 5) | $chunk[7],
        );
        my $chunk = join('', map { chr($_ & 0xff) } @converted);
        $result .= substr($chunk, 0, (@data ? 5 : $truncate));
    }
    return $result;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Base32Hex;

use parent qw/-norequire App::Muter::Backend::Base32/;

sub new {
    my ($class, @args) = @_;
    my $self = $class->SUPER::new(@args);
    $self->{fmap} = [split //, '0123456789ABCDEFGHIJKLMNOPQRSTUV'];
    return $self->_initialize;
}

sub _initialize {
    my ($self) = @_;
    $self->{rmap} = {'=' => 0};
    @{$self->{rmap}}{values @{$self->{fmap}}} = keys @{$self->{fmap}};
    return $self;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::URI;

use parent qw/-norequire App::Muter::Backend::ChunkedDecode/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new($args, %opts, regexp => qr/^(.*)(%.?)$/);
    my $arg = $args->[0] // '';
    $self->{chunk} = '';
    $self->{format} = '%%%02' . ($arg eq 'lower' ? 'x' : 'X');
    return $self;
}

sub metadata {
    my $self = shift;
    my $meta = $self->SUPER::metadata;
    return {
        %$meta,
        args => {
            'upper' => 'Use uppercase letters',
            'lower' => 'Use lowercase letters',
        }
    };
}

sub encode_chunk {
    my ($self, $data) = @_;
    $data =~ s/([^A-Za-z0-9-._~])/sprintf $self->{format}, ord($1)/ge;
    return $data;
}

sub decode_chunk {
    my ($self, $data) = @_;
    $data =~ s/%([0-9a-fA-F]{2})/chr(hex($1))/ge;
    return $data;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::XML;

use parent qw/-norequire App::Muter::Backend::ChunkedDecode/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self = $class->SUPER::new($args, %opts, regexp => qr/^(.*)(&[^;]*)$/);
    no warnings 'qw';    ## no critic (ProhibitNoWarnings)
    my $maps = {
        default => [qw/quot amp apos lt gt/],
        html    => [qw/quot amp #x27 lt gt/],
        hex     => [qw/#x22 #x38 #x27 #x3c #x3e/],
    };
    my $type = $args->[0] // 'default';
    @{$self->{fmap}}{qw/" & ' < >/} = map { "&$_;" } @{$maps->{$type}};
    @{$self->{rmap}}{@{$maps->{default}}} = qw/" & ' < >/;
    return $self;
}

sub metadata {
    my $self = shift;
    my $meta = $self->SUPER::metadata;
    return {
        %$meta,
        args => {
            default => 'Use XML entity names',
            html    => 'Use HTML-friendly entity names for XML entities',
            hex     => 'Use hexadecimal entity names for XML entities',
        }
    };
}

# XML encodes Unicode characters.  However, muter only works on byte sequences,
# so immediately encode these into UTF-8.
sub _decode_char {
    my ($self, $char) = @_;
    return chr($1)              if $char =~ /^#([0-9]+)$/;
    return chr(hex($1))         if $char =~ /^#x([a-fA-F0-9]+)$/;
    return $self->{rmap}{$char} if exists $self->{rmap}{$char};
    die "Unknown XML entity &$char;";
}

sub encode_chunk {
    my ($self, $data) = @_;
    $data =~ s/(["&'<>])/$self->{fmap}{$1}/ge;
    return $data;
}

sub decode_chunk {
    my ($self, $data) = @_;
    require Encode;
    $data =~ s/&([^;]+);/Encode::encode('UTF-8', $self->_decode_char($1))/ge;
    return $data;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::QuotedPrintable;

use parent qw/-norequire App::Muter::Backend::ChunkedDecode/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self =
        $class->SUPER::new($args, %opts, regexp => qr/\A(.*)(=[^\n]?)\z/);
    $self->{curlen} = 0;
    return $self;
}

sub encode_chunk {
    my ($self, $data) = @_;
    $data =~ s/([^\x21-\x3c\x3e-\x7e])/sprintf '=%02X', ord($1)/ge;
    my $result = '';
    my $maxlen = 75;
    while ($self->{curlen} + length($data) > $maxlen) {
        my $chunk = substr($data, 0, $maxlen - $self->{curlen});
        $chunk = $1 if $chunk =~ /^(.*)(=.?)$/;
        $data = substr($data, length($chunk));
        $result .= $chunk;
        if ($data) {
            $result .= "=\n";
            $self->{curlen} = 0;
        }
    }
    $result .= $data;
    $self->{curlen} += length($data);
    return $result;
}

sub decode_chunk {
    my ($self, $data) = @_;
    $data =~ s/=\n//g;
    $data =~ s/=([0-9A-F]{2})/chr(hex($1))/ge;
    return $data;
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Vis;

use parent qw/-norequire App::Muter::Backend::ChunkedDecode/;

sub new {
    my ($class, $args, %opts) = @_;
    my $self =
        $class->SUPER::new($args, %opts, regexp => qr/\A(.*?)(\\.{0,2})\z/);
    $self->_setup_maps(map { $_ => 1 } @$args);
    $self->{chunk} = '';
    return $self;
}

sub _setup_maps {
    my ($self, %flags) = @_;
    my $default = {
        (map { $_ => _encode($_, {}) } (0x00 .. 0x20, 0x7f .. 0xff)),
        (map { $_ => chr($_) } 0x21 .. 0x7e),
        0x5c => "\\\\",
    };
    my $cstyle = {
        (
            map { $_ => _encode($_, {}) }
                (0x01 .. 0x06, 0x0e .. 0x1f, 0x7f .. 0xff)
        ),
        (map { $_ => chr($_) } 0x21 .. 0x7e),
        0x00 => "\\000",
        0x07 => "\\a",
        0x08 => "\\b",
        0x09 => "\\t",
        0x0a => "\\n",
        0x0b => "\\v",
        0x0c => "\\f",
        0x0d => "\\r",
        0x20 => "\\s",
        0x5c => "\\\\",
    };
    my $wanted_map = $flags{cstyle} ? $cstyle : $default;
    my @chars = (
        ($flags{sp} || $flags{space} || $flags{white} ? () : (0x20)),
        ($flags{tab} || $flags{white} ? () : (0x09)),
        ($flags{nl}  || $flags{white} ? () : (0x0a)),
    );
    my $extras = {map { $_ => chr($_) } (0x09, 0x0a, 0x20)};
    $self->{map} = {%$wanted_map, map { $_ => chr($_) } @chars};
    $self->{rmap} = {
        reverse(%$wanted_map), reverse(%$extras),
        reverse(%$cstyle), "\\0" => 0x00
    };
    return;
}

sub _encode {
    my ($byte, $map) = @_;
    use bytes;
    return $map->{$byte} if exists $map->{$byte};
    my $ascii = $byte & 0x7f;
    my $meta = $byte & 0x80 ? 'M' : '';
    return "\\$meta^" . chr($ascii ^ 0x40) if $ascii < 0x20 || $ascii == 0x7f;
    return "\\M-" . chr($byte ^ 0x80) if $byte >= 0xa1 && $byte <= 0xfe;
    return sprintf "\\%03o", $byte if $ascii == 0x20;
    die sprintf "byte value %#02x", $byte;
}

sub encode {
    my ($self, $data) = @_;
    $data = $self->{chunk} . $data;
    if (length $data && substr($data, -1) eq "\0") {
        $data = substr($data, 0, -1);
        $self->{chunk} = "\0";
    }
    else {
        $self->{chunk} = '';
    }
    return $self->SUPER::encode($data);
}

sub encode_chunk {
    my ($self, $data) = @_;
    my $result = join('', map { $self->{map}{$_} } unpack('C*', $data));
    # Do this twice to fix multiple consecutive NUL bytes.
    $result =~ s/\\000($|[^0-7])/\\0$1/g for 1 .. 2;
    return $result;
}

sub _decode {
    my ($self, $val) = @_;
    use bytes;
    return '' if !length $val;
    return chr($self->{rmap}{$val} // die "val '$_'") if $val =~ /^\\/;
    return join('', map { chr($self->{rmap}{$_}) } split //, $val);
}

sub decode_chunk {
    my ($self, $data) = @_;
    print STDERR "data is '$data'\n";
    return join('',
        map { $self->_decode($_) }
            split /(\\(?:M[-^].|\^.|[0-7]{3}|\\|[0abtnvfrs]))/,
        $data);
}

sub metadata {
    my $self = shift;
    my $meta = $self->SUPER::metadata;
    return {
        %$meta,
        args => {
            sp     => 'Encode space',
            space  => 'Encode space',
            tab    => 'Encode tab',
            nl     => 'Encode newline',
            white  => 'Encode space, tab, and newline',
            cstyle => 'Encode using C-like escape sequences',
        }
    };
}

App::Muter::Registry->instance->register(__PACKAGE__);

package App::Muter::Backend::Hash;

use Digest::MD5;
use Digest::SHA;

use parent qw/-norequire App::Muter::Backend/;

my $hashes = {};

sub new {
    my ($class, $args, @args) = @_;
    my ($hash) = @$args;
    my $self = $class->SUPER::new($args, @args);
    $self->{hash} = $hashes->{$hash}->();
    return $self;
}

sub encode {
    my ($self, $data) = @_;
    $self->{hash}->add($data);
    return '';
}

sub encode_final {
    my ($self, $data) = @_;
    $self->{hash}->add($data);
    return $self->{hash}->digest;
}

sub metadata {
    my ($self, $data) = @_;
    my $meta = $self->SUPER::metadata;
    $meta->{args} = {map { $_ => '' } keys %$hashes};
    return $meta;
}

sub register_hash {
    my ($name, $code) = @_;
    return $hashes->{$name} unless $code;
    return $hashes->{$name} = $code;
}

register_hash('md5',      sub { Digest::MD5->new });
register_hash('sha1',     sub { Digest::SHA->new });
register_hash('sha224',   sub { Digest::SHA->new(224) });
register_hash('sha256',   sub { Digest::SHA->new(256) });
register_hash('sha384',   sub { Digest::SHA->new(384) });
register_hash('sha512',   sub { Digest::SHA->new(512) });
register_hash('sha3-224', sub { require Digest::SHA3; Digest::SHA3->new(224) });
register_hash('sha3-256', sub { require Digest::SHA3; Digest::SHA3->new(256) });
register_hash('sha3-384', sub { require Digest::SHA3; Digest::SHA3->new(384) });
register_hash('sha3-512', sub { require Digest::SHA3; Digest::SHA3->new(512) });
App::Muter::Registry->instance->register(__PACKAGE__);

# Must be at the end.
package App::Muter::Main;

exit script(@ARGV) unless caller;
