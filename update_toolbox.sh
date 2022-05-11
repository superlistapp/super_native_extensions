#!/bin/sh

# Pub doesn't support sub modules (no recursive checkout) so we have to use git subtree instead.
# This script will update the super_native_extensions subtree
git subtree pull --prefix super_native_extensions/toolbox git@github.com:nativeshell/toolbox.git main --squash
