# Maintainer: Julius de Jeu <julius@voidcorp.nl>
pkgname=cadmium
pkgver=0.1.0
pkgrel=1
pkgdesc="A simple Display Manager"
makedepends=('cargo')
depends=('pam' 'libxcb' 'dbus')
arch=('i686' 'x86_64' 'armv6h' 'armv7h')
url=https://github.com/jonay2000/cadmium
license=('MIT')

build() {
    cargo build --release
}

package() {
    cd ..
    install -Dm755 "target/release/cadmium" "${pkgdir}/usr/bin/cadmium"
    install -Dm644 "cadmium.service" "${pkgdir}/usr/lib/systemd/system/cadmium.service"
}
