
_default:
    @just -l

check:
    cargo fmt --check
    cargo clippy --no-deps --all-features --workspace -- -D warnings
    cargo clippy --no-deps --all-features --workspace --target x86_64-pc-windows-gnu -- -D warnings
    cargo doc --no-deps --all-features --workspace
    cargo doc --no-deps --all-features --workspace --target x86_64-pc-windows-gnu
    cargo test --all-features --workspace

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
    sed -i "/^version/s/\".[^\"]*\"/\"{{version}}\"/" Cargo.toml noita-engine-reader{,-macros}/Cargo.toml
    cargo update -q --offline

    # Make a commit and the annotated tag
    git commit -am "release: {{version}}"
    git branch -f main HEAD # jj tug lmao
    git branch -f release HEAD
    git tag --cleanup=whitespace -m "$annotation" "v{{version}}"

    read -p $'Push it? [y/N]\n' -n 1 -r
    if [[ "$REPLY" =~ ^[Yy]$ ]]; then
        for remote in $(git remote); do
            git push $remote refs/heads/main refs/heads/release refs/tags/v{{version}} --force
        done
    fi
