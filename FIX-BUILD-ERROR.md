# Fix: "stdbool.h file not found" Build Error

**Error:** `unable to generate bindings clang diagnostic stdbool.h file not found`
**Cause:** Missing libclang and C headers needed for bindgen (used by PipeWire bindings)
**Solution:** Install clang and LLVM development packages

---

## Quick Fix (Run on VM)

```bash
# Install clang and LLVM
sudo apt install -y \
    clang \
    llvm-dev \
    libclang-dev \
    libc6-dev

# Verify installation
clang --version
# Should show: clang version 14.0 or higher

# Set environment variable for bindgen
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
# OR
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu

# Try build again
cd ~/wayland-rdp
cargo clean
cargo build --release
```

---

## If That Doesn't Work

### Find libclang location:

```bash
# Find libclang
find /usr/lib -name "libclang.so*" 2>/dev/null

# Example output:
# /usr/lib/x86_64-linux-gnu/libclang-14.so.1
# /usr/lib/llvm-14/lib/libclang.so

# Set environment to the directory (without the filename)
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu

# Or add to your shell profile
echo 'export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu' >> ~/.bashrc
source ~/.bashrc
```

---

## Alternative: Install All Build Dependencies

```bash
# Comprehensive package install
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
    ninja-build \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    libdbus-1-dev \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome

# Clean and rebuild
cd ~/wayland-rdp
cargo clean
cargo build --release
```

---

## Updated Setup Script

Here's the corrected setup script with clang included:

```bash
#!/bin/bash
set -e

echo "Installing dependencies..."
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
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    libdbus-1-dev \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome

# Set libclang path
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu
echo 'export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu' >> ~/.bashrc

echo "Installing Rust..."
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "Cloning repository..."
cd ~
if [ ! -d "wayland-rdp" ]; then
    git clone https://github.com/lamco-admin/wayland-rdp.git
fi
cd wayland-rdp

echo "Building (this takes 5-10 minutes)..."
cargo build --release

echo "Setup complete!"
```

---

## After Fix

Once build succeeds, you'll see:

```
   Compiling wrd-server v0.1.0
   Finished `release` profile [optimized] target(s) in 1m 07s
```

Then continue with:
- Generate certificates
- Create config
- Run server
- Test connection

---

## Quick Command Summary

```bash
# Fix the error
sudo apt install -y clang llvm-dev libclang-dev libc6-dev
export LIBCLANG_PATH=/usr/lib/x86_64-linux-gnu

# Rebuild
cd ~/wayland-rdp
cargo clean
cargo build --release

# Should succeed now!
```

