#/usr/bin/env bash

set -e

cp -r --no-preserve=mode,ownership result-2/ linux-generic
chmod +x linux-generic/bin/noita-utility-box
pushd linux-generic
tar -czf ../noita-utility-box-linux-generic.tar.gz *
popd

cp --no-preserve=mode,ownership result-1/bin/noita-utility-box.exe .

cp --no-preserve=mode,ownership result-3 noita-utility-box.deb
