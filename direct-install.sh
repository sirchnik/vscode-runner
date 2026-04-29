#!/bin/bash

set -e

./package.sh

cd build/vscode_runner

./install.sh