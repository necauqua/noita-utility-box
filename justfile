
_default:
    @just -l

check:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo doc --no-deps --all-features
    cargo test --all-features

build target:
    nix build . ".#{{target}}"

build-all:
    nix build . ".#windows" ".#linux" ".#deb"

release version: check
    #!/usr/bin/env bash
    set -euo pipefail

    # Extract the unreleased changelog section to be the tag annotation
    annotation="Release v{{version}} $(echo; sed -n '/## \[Unreleased\]/,/## \[/{/## \[/!p;}' CHANGELOG.md)"
    echo $annotation

    # Update the unreleased section to be a release with todays date
    sed -i "/## \[Unreleased\]/a\\\\n## [v{{version}}] $(date +1%F)
    ;/\[unreleased\]/{s%compare/.*%compare/v{{version}}...HEAD%;n;h;s/v[^]]*/v{{version}}/g;p;g}" CHANGELOG.md

    # Update Cargo.toml and Cargo.lock
    sed -i "/^version/s/\".[^\"]*\"/\"{{version}}\"/" Cargo.toml macros/Cargo.toml
    cargo update -q --offline

    # Make a commit and the annotated tag
    jj ci -m "release: {{version}}" && jj bookmark set main -r @-
    git tag --cleanup=whitespace -m "$annotation" "v{{version}}"

    read -p $'Push it? [y/N]\n' -n 1 -r
    if [[ "$REPLY" =~ ^[Yy]$ ]]; then jj git push && git push --tags; fi

play:
    cargo test --workspace --package playground::test -- --ignored --nocapture
