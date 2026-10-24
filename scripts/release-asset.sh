#!/bin/sh
# release-asset.sh FILE: puts FILE on the GitHub release of this version, a draft too, in place of one of its name.
set -eu
cd "$(dirname "$0")/.."
file=$1
[ -f "$file" ] || { echo "release-asset: no file $file" >&2; exit 1; }
version=$(cargo pkgid)
version=${version##*[#@]}
gh release upload "v$version" "$file" --clobber -R zzqxvii/ztty
echo "release-asset: $(basename "$file") on release v$version"
