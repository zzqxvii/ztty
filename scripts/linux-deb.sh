#!/bin/sh
# dpkg-shlibdeps reads the release ELF, not the build dependency list.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
linux_init x86_64
linux_require dpkg-deb dpkg-shlibdeps desktop-file-validate strip
package=$work/debian/ztty
linux_stage "$package"
strip --strip-unneeded "$package/usr/bin/ztty"
desktop-file-validate "$package/usr/share/applications/$id.desktop"
install -Dm644 packaging/linux/ztty.lintian-overrides \
    "$package/usr/share/lintian/overrides/ztty"
mkdir -p "$package/DEBIAN"
cat > "$work/debian/control" <<EOF
Source: ztty
Section: utils
Priority: optional
Maintainer: Ztty contributors <noreply@github.com>
Build-Depends: debhelper-compat (= 13)

Package: ztty
Architecture: amd64
Description: Terminal on GPUI and Ely
EOF
depends=$(cd "$work" && dpkg-shlibdeps -O -edebian/ztty/usr/bin/ztty)
depends=${depends#shlibs:Depends=}
[ -n "$depends" ] || { echo "linux-deb: dpkg-shlibdeps returned no dependencies" >&2; exit 1; }
cat > "$work/changelog" <<EOF
ztty ($version) unstable; urgency=medium

  * Package the upstream release for Debian and Ubuntu.

 -- Ztty contributors <noreply@github.com>  $(date -R -u)
EOF
gzip -9 -n -c "$work/changelog" > "$package/usr/share/doc/ztty/changelog.gz"
size=$(du -sk "$package/usr" | cut -f1)
cat > "$package/DEBIAN/control" <<EOF
Package: ztty
Version: $version
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Ztty contributors <noreply@github.com>
Installed-Size: $size
Depends: $depends, libwayland-client0, libwayland-egl1, libegl1, libvulkan1, git, curl, xdg-utils
Homepage: https://github.com/zzqxvii/ztty
Description: Terminal on GPUI and Ely
 A GPU-rendered terminal with projects and worktrees, terminals, Git and file
 panels, shell integration and configurable appearance.
EOF
(cd "$package" && find usr -type f -exec md5sum {} +) > "$package/DEBIAN/md5sums"
dpkg-deb --root-owner-group --build "$package" "$out/ztty_${version}_amd64.deb"
