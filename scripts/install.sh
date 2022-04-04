#!/bin/bash
# this is a one liner to build and install this repo.
# eventually this should live in xtask but this was much easier
# and we don't have internal windows developers not using WSL (afaik)
_ostype="$(uname -s)"
case "$_ostype" in
    Linux)
        _ostype=x86_64-unknown-linux-gnu
        ;;

    Darwin)
         _ostype=x86_64-apple-darwin
        ;;

    MINGW* | MSYS* | CYGWIN*)
        _ostype=x86_64-pc-windows-msvc
        ;;

    *)
        err "no precompiled binaries available for OS: $_ostype"
        ;;
esac
cargo xtask dist --debug
SRC="./federation-1/target/$_ostype/debug/supergraph"
DEST="$HOME/.rover/bin/supergraph-v$($SRC --version | sed 's/supergraph //g')"
echo "moving $SRC to $DEST"
rm -f $DEST
cp $SRC $DEST

SRC="./federation-2/target/$_ostype/debug/supergraph"
DEST="$HOME/.rover/bin/supergraph-v$($SRC --version | sed 's/supergraph //g')"
echo "moving $SRC to $DEST"
rm -f $DEST
cp $SRC $DEST
