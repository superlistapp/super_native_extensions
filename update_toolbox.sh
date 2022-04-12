#!/bin/sh

# Pub doesn't support sub modules (no recursive checkout) so we have to use git subtree instead.
# This script will update the super_data_transfer subtree
git subtree pull --prefix super_data_transfer/toolbox git@github.com:nativeshell/toolbox.git main --squash
