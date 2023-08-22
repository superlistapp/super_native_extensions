#!/bin/sh

set -e

new_version=$1
# Ensure consistent version for all packages
melos version -Vsuper_clipboard:$new_version -Vsuper_drag_and_drop:$new_version -Vsuper_context_menu:$new_version -Vsuper_native_extensions:$new_version -Vsuper_hot_key:$new_version -Vsuper_keyboard_layout:$new_version
