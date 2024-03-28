#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPT_DIR/..

NEW_VERSION="${1}"

echo "Bumping version: ${NEW_VERSION}"
perl -pi -e "s/^version = \".*?\"/version = \"$NEW_VERSION\"/" insta/Cargo.toml
perl -pi -e "s/^version = \".*?\"/version = \"$NEW_VERSION\"/" cargo-insta/Cargo.toml
perl -pi -e "s/^(insta.*?)version = \".*?\"/\$1version = \"=$NEW_VERSION\"/" cargo-insta/Cargo.toml

cargo check -p insta
cargo check -p cargo-insta
