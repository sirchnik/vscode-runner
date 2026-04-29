#!/bin/bash

set -e

rm -rf build/*.tar.gz
rm -rf build/vscode_runner/*

mkdir -p build/vscode_runner

cargo build --release

cp ./target/release/vscode_runner build/vscode_runner/
cp ./package/* build/vscode_runner/

tar -czvf build/vscode_runner.tar.gz \
    -C build/vscode_runner .
