#!/bin/sh
set -ex

# cargo install mdbook
# mdbook build docs

mkdir -p bin
curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.37/mdbook-v0.4.37-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=bin
bin/mdbook build docs

cargo doc --no-deps
cp -r target/doc docs/book/
