#!/usr/bin/env bash
set -euo pipefail

echo "==> Installing system build & runtime dependencies for Ubuntu / Linux Mint..."
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  pkg-config \
  libx11-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libxkbcommon-dev \
  libwayland-dev \
  libgl1-mesa-dev \
  libssl-dev \
  fwupd \
  lm-sensors \
  policykit-1

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Installing Rust toolchain via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

echo "==> Building release binary (myasus-rs)..."
cargo build --release

echo "==> Installing binary, desktop entry, and udev rules..."
sudo install -Dm755 target/release/myasus-rs /usr/local/bin/myasus-rs
sudo install -Dm644 packaging/myasus-rs.desktop /usr/share/applications/myasus-rs.desktop
sudo install -Dm644 packaging/99-myasus-rs.rules /etc/udev/rules.d/99-myasus-rs.rules

# Apply permissions immediately for current boot
sudo usermod -aG plugdev,video "${SUDO_USER:-$USER}" || true
sudo udevadm control --reload-rules
sudo udevadm trigger

if [ -e /sys/firmware/acpi/platform_profile ]; then
  sudo chgrp plugdev /sys/firmware/acpi/platform_profile || true
  sudo chmod 0664 /sys/firmware/acpi/platform_profile || true
fi

echo ""
echo "Done! Launch 'MyASUS (Rust)' from your Linux Mint / Ubuntu application menu"
echo "or run 'myasus-rs' (GUI) / 'myasus-rs status' (CLI)."
echo "Note: Log out and back in once so your 'plugdev' group membership takes effect."
