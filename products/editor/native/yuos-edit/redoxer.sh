#!/usr/bin/env bash

set -ex

rm -rf target/redoxer
mkdir -p target/redoxer

redoxer install \
    --no-default-features \
    --no-track \
    --path . \
    --root "target/redoxer"

cmd="env RUST_LOG=cosmic_text=debug,yuos_edit=debug ./bin/yuos-edit"
if [ -f "$1" ]
then
    filename="$(basename "$1")"
    cp "$1" "target/redoxer/${filename}"
    cmd="${cmd} '${filename}'"
fi

cd target/redoxer

redoxer exec \
    --gui \
    --folder . \
    /bin/sh -c \
    "${cmd}"
