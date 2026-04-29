#!/bin/sh
# Shared check functions

check_cmd() {
    cmd="$1"
    name="${2:-$cmd}"
    get_version="${3:-$cmd --version 2>&1}"
    min_version="${4:-}"

    if ! command -v "$cmd" >/dev/null 2>&1; then
        fail "$name is not installed. Please install $name."
        return 1
    fi

    version=$($get_version 2>&1 | head -1)
    pass "$name — $version"
    return 0
}
