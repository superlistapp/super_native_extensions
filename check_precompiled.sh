#!/bin/sh

BASEDIR=$(dirname "$0")

cd $BASEDIR/super_native_extensions/cargokit/build_tool

# Check whether the precompiled binaries ara available for each architecture.
# Note: aaarch64-unknown-linux-gnu... is meant to be missing as there is no
# cross-compilation available currently.

dart run build_tool verify-binaries --manifest-dir=../../rust