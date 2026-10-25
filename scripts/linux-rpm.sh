#!/bin/sh
# RPM generates versioned ELF Requires from the same release binary.
set -eu
cd "$(dirname "$0")/.."
. scripts/linux-common.sh
linux_init x86_64
linux_require rpmbuild rpm desktop-file-validate strip
linux_stage "$work/payload"
strip --strip-unneeded "$work/payload/usr/bin/ztty"
desktop-file-validate "$work/payload/usr/share/applications/$id.desktop"
mkdir -p "$work/rpmbuild/SPECS" "$work/rpmbuild/SOURCES"
mkdir "$work/rpmdb"
rpm --dbpath "$work/rpmdb" --initdb
tar -C "$work/payload" -czf "$work/rpmbuild/SOURCES/payload.tar.gz" .
cat > "$work/rpmbuild/SPECS/ztty.spec" <<EOF
Name: ztty
Version: $version
Release: 1
Summary: Terminal on GPUI and Ely
License: MIT AND OFL-1.1 AND FTL
URL: https://github.com/zzqxvii/ztty
Source0: payload.tar.gz
Requires: libwayland-client.so.0()(64bit)
Requires: libwayland-egl.so.1()(64bit)
Requires: libEGL.so.1()(64bit)
Requires: libvulkan.so.1()(64bit)
Requires: git, curl, xdg-utils

%description
A GPU-rendered terminal with projects and worktrees, terminals, Git and file
panels, shell integration and configurable appearance.

%prep

%build

%install
mkdir -p %{buildroot}
tar -xzf %{SOURCE0} -C %{buildroot}

%files
/usr/bin/ztty
/usr/share/applications/$id.desktop
/usr/share/icons/hicolor/scalable/apps/$id.svg
/usr/share/man/man1/ztty.1.gz
%license /usr/share/doc/ztty
EOF
rpmbuild -bb --target x86_64 --define "_topdir $work/rpmbuild" \
    --define "_dbpath $work/rpmdb" \
    --define '_build_id_links none' --define 'debug_package %{nil}' \
    --define '__os_install_post %{nil}' "$work/rpmbuild/SPECS/ztty.spec"
cp "$work/rpmbuild/RPMS/x86_64/ztty-$version-1.x86_64.rpm" "$out/"
echo "linux-rpm: $out/ztty-$version-1.x86_64.rpm"
