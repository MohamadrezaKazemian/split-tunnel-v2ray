#!/usr/bin/env bash
set -euo pipefail

if (( $# != 1 )); then
  printf 'Usage: %s <version without v>\n' "$0" >&2
  exit 2
fi

pkgver="$1"
case "$pkgver" in
  *[!0-9.]*|'')
    printf 'Version must contain only digits and dots: %s\n' "$pkgver" >&2
    exit 2
    ;;
esac

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
pkgbuild="${script_dir}/split-tunnel-v2ray-bin/PKGBUILD"
release_base="https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/releases/download/v${pkgver}"
deb="split-tunnel-v2ray_${pkgver}_linux_amd64.deb"
checksums="$(curl --fail --location --silent --show-error "${release_base}/SHA256SUMS.txt")"

checksum_for() {
  local file="$1"
  awk -v file="$file" '$2 == file { print $1; exit }' <<<"${checksums}"
}

deb_sha="$(checksum_for "${deb}")"
license_sha="$(checksum_for LICENSE)"
notices_sha="$(checksum_for THIRD_PARTY_NOTICES.md)"
[[ -n "${deb_sha}" && -n "${license_sha}" && -n "${notices_sha}" ]] || {
  printf 'SHA256SUMS.txt is missing a required release asset.\n' >&2
  exit 1
}

sed -i \
  -e "s/^pkgver=.*/pkgver=${pkgver}/" \
  -e 's/^pkgrel=.*/pkgrel=1/' \
  "${pkgbuild}"

# Replace the checksum block without relying on a non-base packaging tool.
tmp_pkgbuild="${pkgbuild}.tmp"
awk -v deb_sha="${deb_sha}" -v license_sha="${license_sha}" -v notices_sha="${notices_sha}" '
  /^sha256sums=\(/ {
    print
    print "  '\''" deb_sha "'\''"
    print "  '\''" license_sha "'\''"
    print "  '\''" notices_sha "'\''"
    in_sums = 1
    next
  }
  in_sums {
    if ($0 == ")") {
      print
      in_sums = 0
    }
    next
  }
  { print }
' "${pkgbuild}" > "${tmp_pkgbuild}"
mv "${tmp_pkgbuild}" "${pkgbuild}"

(
  cd "${script_dir}/split-tunnel-v2ray-bin"
  makepkg --printsrcinfo > .SRCINFO
)

printf 'Updated %s and regenerated .SRCINFO for v%s.\n' "${pkgbuild}" "${pkgver}"
