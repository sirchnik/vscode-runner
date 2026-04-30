#!/bin/bash

set -e

rm -rf build/*.tar.gz
rm -rf build/vscode_krunner/*

mkdir -p build/vscode_krunner

cargo build --release

cp ./target/release/vscode_krunner build/vscode_krunner/
cp ./package/* build/vscode_krunner/

tar -czvf build/vscode_krunner.tar.gz \
    -C build/vscode_krunner .
