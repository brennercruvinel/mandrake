#!/usr/bin/env bash
# Copies every file in $1 into $2.
set -e
src=$1
dst=$2
mkdir -p $dst
for f in $(ls $src); do
  cp $src/$f $dst/
done
