#!/bin/bash

# Unique identifier, ideally a reverse-domain identifier.
identifier=sirchnik.vscode_runner

# Name of this runner.
name=vscode_runner

# Stop the runner process.
kill "$(pidof $name)" &> /dev/null || true

# Ensure our working directory is the scripts directory.
cd "$(dirname "$0")" || exit

# Check for install location.
if [[ -n "$XDG_DATA_HOME" ]]; then
    dataHome="$XDG_DATA_HOME"
else
    dataHome=~/.local/share
fi

# Remove the executable & plugin files.
rm -f ~/.local/bin/$name
rm -f "$dataHome"/krunner/dbusplugins/plasma-runner-$name.desktop
rm -f "$dataHome"/dbus-1/services/$identifier.service
