#!/bin/bash
# this is a one liner to build and install this repo.
# eventually this should live in xtask but this was much easier
# and we don't have internal windows developers not using WSL (afaik)

cargo xtask dist --debug
SRC="./federation-1/target/debug/supergraph"
DEST="$HOME/.rover/bin/supergraph-v$($SRC --version | sed 's/supergraph //g')"
echo "moving $SRC to $DEST"
rm -f $DEST
cp $SRC $DEST

SRC="./federation-2/target/debug/supergraph"
DEST="$HOME/.rover/bin/supergraph-v$($SRC --version | sed 's/supergraph //g')"
echo "moving $SRC to $DEST"
rm -f $DEST
cp $SRC $DEST