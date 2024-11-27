#!/usr/bin/env bash

vers="$1"
if [ -z "$vers" ]; then
    >&2 echo No version given
    exit 1
fi

set -e
IFS=""

# Extract the unreleased changelog section to be the tag annotation
annotation="Release v$vers $(echo; sed -n '/## \[Unreleased\]/,/## \[/{/## \[/!p;}' CHANGELOG.md)"
echo $annotation

# Update the unreleased section to be a release with todays date
sed -i "/## \[Unreleased\]/a\\\\n## [v$vers] $(date +1%F)
;/\[unreleased\]/{s%compare/.*%compare/v$vers...HEAD%;n;h;s/v[^]]*/v$vers/g;p;g}" CHANGELOG.md

# Update Cargo.toml and Cargo.lock
sed -i "/^version/s/\".[^\"]*\"/\"$vers\"/" Cargo.toml
cargo update -q --offline

# Make a commit and the annotated tag
jj commit -m "release: $vers" && jj bookmark set main -r @-
git tag --cleanup=whitespace -m "$annotation" "v$vers"

read -p $'Push it? [y/N]\n' -n 1 -r
if [[ "$REPLY" =~ ^[Yy]$ ]]; then
    jj git push
    git push --tags
fi
