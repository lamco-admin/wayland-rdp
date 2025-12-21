# Installation Guide

This guide covers installing and setting up `lamco-rdp-server` on Linux systems with Wayland.

## System Requirements

### Operating System

- **Linux distribution** with Wayland compositor support
- **Supported desktops**:
  - GNOME (3.38+) - Recommended
  - KDE Plasma (5.20+)
  - Other Wayland compositors with XDG Desktop Portal support

### Runtime Requirements

- **Wayland compositor** running (not X11)
- **PipeWire** (0.3.50+) for screen capture
- **XDG Desktop Portal** with ScreenCast support
- **D-Bus session bus**

### Build Requirements

- **Rust** 1.77 or later
- **pkg-config**
- **System libraries**:
  - libpipewire-0.3-dev
  - libspa-0.2-dev
  - libasound2-dev (for PipeWire)
  - libssl-dev (for TLS support)

## Installation Methods

### Method 1: From Crates.io (Recommended)

```bash
# Install via cargo
cargo install lamco-rdp-server

# Verify installation
lamco-rdp-server --version
```

### Method 2: From Source

```bash
# Clone repository
git clone https://github.com/lamco-admin/lamco-rdp-server.git
cd lamco-rdp-server

# Install system dependencies (Debian/Ubuntu)
sudo apt update
sudo apt install -y \
    pkg-config \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libasound2-dev \
    libssl-dev

# Install system dependencies (Fedora/RHEL)
sudo dnf install -y \
    pkg-config \
    pipewire-devel \
    alsa-lib-devel \
    openssl-devel

# Install system dependencies (Arch Linux)
sudo pacman -S --needed \
    pkg-config \
    pipewire \
    alsa-lib \
    openssl

# Build and install
cargo build --release
sudo install -m 755 target/release/lamco-rdp-server /usr/local/bin/

# Verify installation
lamco-rdp-server --version
```

### Method 3: Pre-built Binary (Coming Soon)

Pre-built binaries will be available on the GitHub releases page.

## Initial Setup

### 1. Create Configuration Directory

```bash
# For system-wide installation
sudo mkdir -p /etc/lamco-rdp-server
sudo cp config.toml.example /etc/lamco-rdp-server/config.toml

# For user installation
mkdir -p ~/.config/lamco-rdp-server
cp config.toml.example ~/.config/lamco-rdp-server/config.toml
```

### 2. Generate TLS Certificates

For production use, use certificates from a trusted CA. For testing:

```bash
# Create certs directory
sudo mkdir -p /etc/lamco-rdp-server/certs

# Generate self-signed certificate (testing only)
openssl req -x509 -newkey rsa:4096 \
    -keyout /tmp/key.pem \
    -out /tmp/cert.pem \
    -days 365 -nodes \
    -subj "/CN=localhost"

# Install certificates
sudo mv /tmp/cert.pem /etc/lamco-rdp-server/certs/
sudo mv /tmp/key.pem /etc/lamco-rdp-server/certs/
sudo chmod 600 /etc/lamco-rdp-server/certs/key.pem
```

### 3. Configure Firewall

```bash
# Allow RDP port (default: 3389)
sudo firewall-cmd --add-port=3389/tcp --permanent
sudo firewall-cmd --reload

# Or with ufw
sudo ufw allow 3389/tcp
```

### 4. Set Up D-Bus Access (SSH Only)

If running via SSH, you need access to the user's D-Bus session:

```bash
# Add to ~/.bashrc or ~/.profile
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
```

### 5. Verify XDG Portal Support

Check that your desktop provides the required portal:

```bash
# Check for ScreenCast portal
busctl --user tree org.freedesktop.portal.Desktop

# You should see:
# /org/freedesktop/portal/desktop
#   org.freedesktop.portal.ScreenCast
#   org.freedesktop.portal.RemoteDesktop
```

## Desktop-Specific Setup

### GNOME

GNOME 40+ includes full portal support out of the box.

For clipboard support on older GNOME versions, install the D-Bus clipboard extension:

```bash
# Extension is included in the repository
cp -r extension/dbus-clipboard@lamco.io ~/.local/share/gnome-shell/extensions/
gnome-extensions enable dbus-clipboard@lamco.io

# Log out and back in to activate
```

### KDE Plasma

KDE Plasma 5.20+ includes portal support. Ensure `xdg-desktop-portal-kde` is installed:

```bash
# Debian/Ubuntu
sudo apt install xdg-desktop-portal-kde

# Fedora
sudo dnf install xdg-desktop-portal-kde

# Arch Linux
sudo pacman -S xdg-desktop-portal-kde
```

### Other Desktops

For other Wayland compositors, ensure you have a compatible portal backend:

- **wlroots-based** (Sway, River, etc.): Install `xdg-desktop-portal-wlr`
- **Hyprland**: Install `xdg-desktop-portal-hyprland`

## Running as a Service

### systemd User Service

Create `/etc/systemd/user/lamco-rdp-server.service`:

```ini
[Unit]
Description=Lamco RDP Server
After=pipewire.service

[Service]
Type=simple
ExecStart=/usr/local/bin/lamco-rdp-server -c /etc/lamco-rdp-server/config.toml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
```

Enable and start:

```bash
systemctl --user enable lamco-rdp-server
systemctl --user start lamco-rdp-server
systemctl --user status lamco-rdp-server
```

### systemd System Service

For system-wide installation, create `/etc/systemd/system/lamco-rdp-server@.service`:

```ini
[Unit]
Description=Lamco RDP Server for user %i
After=network.target

[Service]
Type=simple
User=%i
ExecStart=/usr/local/bin/lamco-rdp-server -c /etc/lamco-rdp-server/config.toml
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Enable for a user:

```bash
sudo systemctl enable lamco-rdp-server@username
sudo systemctl start lamco-rdp-server@username
```

## Verification

### Test Connection

1. Start the server:
```bash
lamco-rdp-server -c /etc/lamco-rdp-server/config.toml -vv
```

2. Connect from an RDP client:
```bash
# From another machine
xfreerdp /v:your-server-ip:3389 /u:yourusername
```

3. Check the logs for successful connection messages

### Troubleshooting

**Portal permission denied**:
```bash
# Grant portal access (GNOME)
# A dialog should appear when connecting - click "Share"
```

**PipeWire not found**:
```bash
# Check PipeWire is running
systemctl --user status pipewire

# Start if needed
systemctl --user start pipewire
```

**D-Bus connection failed** (via SSH):
```bash
# Export D-Bus session address
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
```

**Certificate errors**:
```bash
# Check certificate permissions
ls -l /etc/lamco-rdp-server/certs/
# key.pem should be mode 600
```

## Next Steps

- See [CONFIGURATION.md](CONFIGURATION.md) for configuration options
- See [README.md](README.md) for usage examples
- See [CONTRIBUTING.md](CONTRIBUTING.md) to contribute

## License Note

This software is licensed under the Business Source License 1.1. See [LICENSE](LICENSE) for free use conditions or visit https://lamco.ai for commercial licensing.
