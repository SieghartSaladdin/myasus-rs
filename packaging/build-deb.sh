#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.0"
ARCH="$(dpkg --print-architecture)"
PKG_DIR="target/deb/myasus-rs_${VERSION}_${ARCH}"

echo "==> Building release binary..."
cargo build --release

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN" \
         "$PKG_DIR/usr/local/bin" \
         "$PKG_DIR/usr/share/applications" \
         "$PKG_DIR/etc/udev/rules.d"

install -m755 target/release/myasus-rs "$PKG_DIR/usr/local/bin/myasus-rs"
install -m644 packaging/myasus-rs.desktop "$PKG_DIR/usr/share/applications/myasus-rs.desktop"
install -m644 packaging/99-myasus-rs.rules "$PKG_DIR/etc/udev/rules.d/99-myasus-rs.rules"

cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: myasus-rs
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: libc6, libgl1, libx11-6, libxkbcommon0, policykit-1
Recommends: fwupd, lm-sensors
Maintainer: myasus-rs contributors
Description: MyASUS-inspired Rust hardware control utility for Ubuntu and Linux Mint
 Pure-Rust native desktop & CLI tool providing ASUS laptop telemetry,
 Battery Health Charging thresholds, Fan/Thermal profiles, Splendid Eye Care,
 6-category System Diagnosis, and LVFS/APT update checks via asus-nb-wmi.
EOF

cat > "$PKG_DIR/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
udevadm control --reload-rules || true
udevadm trigger || true
EOF
chmod 755 "$PKG_DIR/DEBIAN/postinst"

dpkg-deb --build "$PKG_DIR"
echo "==> Package built: ${PKG_DIR}.deb"
echo "Install with: sudo apt install ./${PKG_DIR}.deb"
