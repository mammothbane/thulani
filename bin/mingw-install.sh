#!/usr/bin/env bash

set -euo pipefail

tmp=$(mktemp -d)

cleanup() {
  status=$?

  rm -rf "$tmp"

  exit $status
}

trap cleanup EXIT
cd "$tmp"

echo -n "Downloading mysql lib... " >&2

wget \
  'https://downloads.mysql.com/archives/get/p/19/file/mysql-connector-c-6.1.11-winx64.zip' \
   -qO out.zip

echo "done" >&2

pacman -Sq --noconfirm --needed unzip

unzip -j -o out.zip -d unzipped

mv unzipped/libmysql.dll /mingw64/lib/libmysql.dll.a
mv unzipped/libmysql.lib /mingw64/lib/libmysql.a

echo "Installing required packages..." >&2

exec pacman -Sq --noconfirm --needed \
  mingw-w64-x86_64-toolchain \
  mingw-w64-x86_64-opus \
  mingw-w64-x86_64-sqlite3 \
  mingw-w64-x86_64-postgresql \
  mingw-w64-x86_64-openssl \
