# Build and Deployment Guide

## Overview

This guide provides comprehensive instructions for building and deploying the WRD (Wayland Remote Desktop) system with both the headless compositor and direct login service.

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 22.04+, Debian 12+, Fedora 39+, or Arch Linux)
- **Kernel**: Linux 6.0+ (for optimal DMA-BUF support)
- **Architecture**: x86_64 or aarch64

### Required System Packages

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config cmake clang libclang-dev \
    libwayland-dev wayland-protocols \
    libpipewire-0.3-dev libspa-0.2-dev \
    libpam0g-dev libssl-dev libdbus-1-dev \
    libxkbcommon-dev libudev-dev \
    libinput-dev libgbm-dev libdrm-dev libseat-dev \
    systemd libsystemd-dev \
    git curl
```

#### Fedora/RHEL

```bash
sudo dnf install -y \
    gcc gcc-c++ make cmake pkg-config clang clang-devel \
    wayland-devel wayland-protocols-devel \
    pipewire-devel \
    pam-devel openssl-devel dbus-devel \
    libxkbcommon-devel libinput-devel \
    mesa-libgbm-devel libdrm-devel libseat-devel \
    systemd-devel \
    git curl
```

#### Arch Linux

```bash
sudo pacman -S --needed \
    base-devel cmake clang \
    wayland wayland-protocols \
    pipewire \
    pam openssl dbus \
    libxkbcommon libinput \
    mesa libdrm seatd \
    systemd \
    git curl
```

### Rust Toolchain

```bash
# Install rustup if not present
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Set stable as default
rustup default stable

# Update to latest
rustup update stable

# Install components
rustup component add clippy rustfmt
```

## Building

### Option 1: Standard Build (Portal Mode)

Build the standard server that uses xdg-desktop-portal:

```bash
# Clone repository
git clone https://github.com/lamco-admin/wayland-rdp.git
cd wayland-rdp

# Build
cargo build --release

# Binary will be at: target/release/wrd-server
```

### Option 2: Headless Compositor Build

Build with the custom Smithay compositor:

```bash
# Build with headless compositor
cargo build --release --features headless-compositor,pam-auth

# Binaries:
# - target/release/wrd-server (with compositor)
# - Or create separate binaries for compositor and login service
```

### Option 3: Separate Binaries

For production deployment, you may want separate binaries:

**Compositor Binary:**
```bash
cargo build --release --bin wrd-compositor --features headless-compositor
```

**Login Service Binary:**
```bash
cargo build --release --bin wrd-login-daemon --features headless-compositor,pam-auth
```

## Configuration

### 1. Create Configuration Directory

```bash
sudo mkdir -p /etc/wrd-login
sudo mkdir -p /etc/wrd-login/certs
sudo mkdir -p /var/log/wrd-login
```

### 2. Generate TLS Certificates

```bash
# Self-signed certificate (for testing)
openssl req -x509 -newkey rsa:4096 \
    -keyout /etc/wrd-login/certs/server.key \
    -out /etc/wrd-login/certs/server.crt \
    -days 365 -nodes \
    -subj "/CN=wrd-server"

# Set permissions
sudo chmod 600 /etc/wrd-login/certs/server.key
sudo chmod 644 /etc/wrd-login/certs/server.crt
```

### 3. Create Configuration File

Create `/etc/wrd-login/config.toml`:

```toml
[network]
bind_address = "0.0.0.0"
port = 3389
max_connections = 100
connection_timeout = 300

[security]
enable_lockout = true
max_failed_attempts = 5
lockout_duration = 300
require_strong_passwords = true
audit_logging = true

[session]
timeout = 0  # 0 = no timeout
enable_xwayland = false
auto_start_apps = []

[paths]
compositor_path = "/usr/bin/wrd-compositor"
cert_path = "/etc/wrd-login/certs/server.crt"
key_path = "/etc/wrd-login/certs/server.key"
pam_service = "wrd-login"
log_dir = "/var/log/wrd-login"

[limits]
max_memory_mb = 2048
cpu_shares = 1024
max_processes = 256
max_open_files = 1024
```

### 4. Configure PAM

Create `/etc/pam.d/wrd-login`:

```
#%PAM-1.0
auth       required     pam_env.so
auth       required     pam_unix.so
account    required     pam_unix.so
session    required     pam_unix.so
session    required     pam_systemd.so
```

## Installation

### 1. Install Binaries

```bash
# Install to /usr/bin
sudo install -m 755 target/release/wrd-server /usr/bin/
sudo install -m 755 target/release/wrd-compositor /usr/bin/
sudo install -m 755 target/release/wrd-login-daemon /usr/bin/
```

### 2. Install systemd Units

```bash
# Install service files
sudo cp systemd/wrd-login.service /etc/systemd/system/
sudo cp systemd/wrd-compositor@.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload
```

### 3. Enable Services

```bash
# Enable login service
sudo systemctl enable wrd-login.service

# Optionally start now
sudo systemctl start wrd-login.service
```

## Verification

### 1. Check Service Status

```bash
# Check login service
sudo systemctl status wrd-login.service

# View logs
sudo journalctl -u wrd-login.service -f
```

### 2. Test RDP Connection

From a Windows machine or Linux with RDP client:

```bash
# Linux
xfreerdp /v:YOUR_SERVER_IP:3389 /u:YOUR_USERNAME

# Windows
mstsc.exe
# Enter: YOUR_SERVER_IP:3389
```

### 3. Verify Session Creation

After successful login:

```bash
# List sessions
loginctl list-sessions

# Check user session
loginctl show-session SESSION_ID

# Verify compositor is running
ps aux | grep wrd-compositor
```

## Troubleshooting

### Login Service Won't Start

**Check systemd-logind:**
```bash
systemctl status systemd-logind.service
```

**Check D-Bus:**
```bash
systemctl status dbus.service
```

**Verify Certificates:**
```bash
ls -la /etc/wrd-login/certs/
openssl x509 -in /etc/wrd-login/certs/server.crt -text -noout
```

### Connection Refused

**Check Port:**
```bash
sudo netstat -tlnp | grep 3389
# or
sudo ss -tlnp | grep 3389
```

**Check Firewall:**
```bash
# UFW
sudo ufw allow 3389/tcp

# firewalld
sudo firewall-cmd --permanent --add-port=3389/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 3389 -j ACCEPT
```

### Compositor Won't Start

**Check Runtime Directory:**
```bash
ls -la /run/user/$(id -u)/
```

**Check Permissions:**
```bash
# Runtime dir should be owned by user
sudo chown -R $USER:$USER /run/user/$(id -u)/
```

**Check Dependencies:**
```bash
# Verify libraries are installed
ldd /usr/bin/wrd-compositor
```

### Authentication Fails

**Test PAM Configuration:**
```bash
# Check PAM service file exists
cat /etc/pam.d/wrd-login

# Test with pamtester (if available)
sudo pamtester wrd-login $USER authenticate
```

**Check Audit Log:**
```bash
sudo cat /var/log/wrd-login/audit.log
```

### Performance Issues

**Monitor Resources:**
```bash
# Check memory usage
systemctl status wrd-compositor@USERNAME.service

# Check cgroup limits
cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/memory.max
cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/cpu.weight
```

**Adjust Limits:**
Edit `/etc/wrd-login/config.toml` and modify `[limits]` section.

## Development

### Running in Development

```bash
# Set environment
export RUST_LOG=wrd_server=debug,info
export RUST_BACKTRACE=1

# Run directly (requires root for port 3389)
sudo -E cargo run --features headless-compositor,pam-auth
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests (requires D-Bus and systemd)
cargo test --features headless-compositor,pam-auth

# Specific module
cargo test --package wrd-server --lib login::logind
```

### Code Quality

```bash
# Clippy (linter)
cargo clippy --all-features

# Format
cargo fmt

# Check without building
cargo check --all-features
```

## Security Hardening

### 1. SELinux/AppArmor

Create appropriate profiles for the services.

### 2. Limit Network Access

```bash
# Bind only to specific interface
# In config.toml:
bind_address = "192.168.1.10"  # Replace with your IP
```

### 3. Use Strong Certificates

Replace self-signed certificates with proper CA-signed certificates:

```bash
# Copy your certificates
sudo cp your-cert.crt /etc/wrd-login/certs/server.crt
sudo cp your-key.key /etc/wrd-login/certs/server.key
sudo chmod 600 /etc/wrd-login/certs/server.key
```

### 4. Enable Audit Logging

Ensure `audit_logging = true` in config and monitor:

```bash
sudo tail -f /var/log/wrd-login/audit.log
```

### 5. Resource Limits

Adjust cgroup limits in config.toml based on your system.

## Monitoring

### Systemd Journal

```bash
# Follow all WRD logs
sudo journalctl -u 'wrd-*' -f

# Export logs
sudo journalctl -u wrd-login.service --since today > wrd-login.log
```

### Metrics (Future)

Integration with Prometheus planned for monitoring:
- Active sessions
- Frame rates
- Memory usage per user
- Authentication failures

## Updating

```bash
# Pull latest code
git pull origin main

# Rebuild
cargo build --release --features headless-compositor,pam-auth

# Stop services
sudo systemctl stop wrd-login.service

# Update binaries
sudo install -m 755 target/release/wrd-login-daemon /usr/bin/
sudo install -m 755 target/release/wrd-compositor /usr/bin/

# Restart services
sudo systemctl start wrd-login.service
```

## Uninstall

```bash
# Stop and disable services
sudo systemctl stop wrd-login.service
sudo systemctl disable wrd-login.service

# Remove binaries
sudo rm /usr/bin/wrd-server
sudo rm /usr/bin/wrd-compositor
sudo rm /usr/bin/wrd-login-daemon

# Remove systemd units
sudo rm /etc/systemd/system/wrd-login.service
sudo rm /etc/systemd/system/wrd-compositor@.service
sudo systemctl daemon-reload

# Optionally remove configuration
sudo rm -rf /etc/wrd-login
sudo rm -rf /var/log/wrd-login
```

## Further Reading

- [Architecture Documentation](HEADLESS-COMPOSITOR-ARCHITECTURE.md)
- [Implementation Status](IMPLEMENTATION-STATUS.md)
- [Systemd Units README](systemd/README.md)

## Support

For issues, please check:
1. System logs: `journalctl -xe`
2. Service status: `systemctl status wrd-login.service`
3. Audit log: `/var/log/wrd-login/audit.log`
4. GitHub Issues: https://github.com/lamco-admin/wayland-rdp/issues
