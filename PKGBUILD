# Maintainer: shake_11 <shake-11@users.noreply.github.com>
pkgname=yandex-music-tui-git
pkgver=r2.ce8329b
pkgrel=1
pkgdesc="A keyboard-first, unofficial Yandex Music client for the terminal"
arch=('x86_64' 'aarch64')
url="https://github.com/shake-11/yandex-music-tui"
license=('MIT')
depends=('alsa-lib' 'openssl' 'gcc-libs' 'glibc')
makedepends=('git' 'rust' 'pkg-config')
provides=('yandex-music-tui')
conflicts=('yandex-music-tui')
source=("git+${url}.git")
sha256sums=('SKIP')

pkgver() {
  cd "${srcdir}/${pkgname%-git}"
  printf 'r%s.%s' "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
  cd "${srcdir}/${pkgname%-git}"
  export CARGO_HOME="${srcdir}/cargo"
  cargo build --release --locked
}

package() {
  cd "${srcdir}/${pkgname%-git}"
  install -Dm755 target/release/yandex-music-tui "${pkgdir}/usr/bin/yandex-music-tui"
  install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
  install -Dm644 README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"
  install -Dm644 config.example.toml "${pkgdir}/usr/share/doc/${pkgname}/config.example.toml"
}
