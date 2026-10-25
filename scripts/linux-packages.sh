#!/bin/sh
# One build, four packages; tools stay in the container.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
case $(uname -m) in
    x86_64) ;;
    *) echo "linux-packages: DEB, RPM and AppImage require x86_64; use linux-tar.sh for ARM64" >&2; exit 1 ;;
esac
scripts/linux-build.sh
linux_run -i \
    -e LINUX_BINARY=/src/target/release/ztty ztty-release sh -eu <<'EOF'
scripts/linux-tar.sh
scripts/linux-deb.sh
scripts/linux-appimage.sh
scripts/linux-rpm.sh
. scripts/linux-common.sh
linux_version
cd target/release
sha256sum "Ztty-$version-linux-x86_64.tar.gz" \
    "ztty_${version}_amd64.deb" \
    "Ztty-$version-linux-x86_64.AppImage" \
    "ztty-$version-1.x86_64.rpm" \
    > "Ztty-$version-linux-x86_64.sha256"
EOF
