#!/bin/bash
# Complete WRD-Server Setup for Ubuntu 24.04
# Run this on VM at 192.168.10.205
# User: greg

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║     WRD-Server Complete Setup for Ubuntu 24.04             ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# 1. Install ALL dependencies including clang
echo "[1/9] Installing dependencies (this may take a few minutes)..."
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    git \
    curl \
    clang \
    llvm-dev \
    libclang-dev \
    libc6-dev \
    cmake \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    libdbus-1-dev \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome

echo "✓ Dependencies installed"

# 2. Set libclang path
echo "[2/9] Configuring libclang..."
export LIBCLANG_PATH=/usr/lib/llvm-18/lib
echo 'export LIBCLANG_PATH=/usr/lib/llvm-18/lib' >> ~/.bashrc

# Verify libclang
if [ -f "$LIBCLANG_PATH/libclang.so" ] || [ -f "$LIBCLANG_PATH/libclang.so.1" ]; then
    echo "✓ libclang found at $LIBCLANG_PATH"
else
    echo "! libclang not at /usr/lib/llvm-18/lib, trying alternative..."
    export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
    echo 'export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu' >> ~/.bashrc
    echo "✓ Using $LIBCLANG_PATH"
fi

# 3. Verify PipeWire
echo "[3/9] Setting up PipeWire..."
systemctl --user enable pipewire pipewire-pulse wireplumber 2>/dev/null || true
systemctl --user start pipewire pipewire-pulse wireplumber

sleep 2

if systemctl --user is-active --quiet pipewire; then
    PW_VER=$(pipewire --version 2>/dev/null || echo "unknown")
    echo "✓ PipeWire running: $PW_VER"
else
    echo "✗ PipeWire not running"
    exit 1
fi

# 4. Verify Portal
echo "[4/9] Verifying Portal..."
if busctl --user list 2>/dev/null | grep -q org.freedesktop.portal.Desktop; then
    echo "✓ Portal available"
else
    echo "Starting Portal..."
    systemctl --user start xdg-desktop-portal || true
    sleep 2
fi

# 5. Install Rust
echo "[5/9] Installing Rust..."
if command -v cargo &> /dev/null; then
    echo "✓ Rust already installed: $(rustc --version)"
else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "✓ Rust installed"
fi

# Make sure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# 6. Get code
echo "[6/9] Getting source code..."
cd ~
if [ -d "wayland-rdp" ]; then
    echo "✓ Directory exists"
    cd wayland-rdp
    git pull || true
else
    echo "Cloning repository..."
    git clone https://github.com/lamco-admin/wayland-rdp.git
    cd wayland-rdp
fi

# 7. Build
echo "[7/9] Building wrd-server (5-10 minutes)..."
echo "This will take a while. Please wait..."
cargo build --release

if [ -f "target/release/wrd-server" ]; then
    echo "✓ Build successful!"
    ls -lh target/release/wrd-server
else
    echo "✗ Build failed"
    exit 1
fi

# 8. Certificates
echo "[8/9] Generating certificates..."
mkdir -p certs
if [ ! -f certs/cert.pem ]; then
    openssl req -x509 -newkey rsa:4096 -nodes \
        -keyout certs/key.pem \
        -out certs/cert.pem \
        -days 365 \
        -subj "/CN=wrd-server/O=Testing/C=US"
    echo "✓ Certificates generated"
else
    echo "✓ Certificates exist"
fi

# 9. Config
echo "[9/9] Creating configuration..."
cat > config.toml <<'CONFIGEOF'
[server]
listen_addr = "0.0.0.0:3389"
max_connections = 5
session_timeout = 0
use_portals = true

[security]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
require_tls = true

[video]
max_fps = 30
enable_damage_tracking = true
CONFIGEOF

echo "✓ Config created"

# Firewall
if command -v ufw &> /dev/null; then
    sudo ufw allow 3389/tcp 2>/dev/null || true
    echo "✓ Firewall configured"
fi

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║              SETUP COMPLETE!                               ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "To start the server:"
echo ""
echo "  cd ~/wayland-rdp"
echo "  ./target/release/wrd-server -c config.toml -vv"
echo ""
echo "Then:"
echo "  1. Grant permission when dialog appears"
echo "  2. Connect from Windows: mstsc.exe → 192.168.10.205:3389"
echo ""
