# WRD Server Headless Deployment Guide

**Complete guide for deploying WRD Server in headless mode for multi-user VDI/RDP hosting**

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [System Requirements](#system-requirements)
4. [Installation](#installation)
5. [Configuration](#configuration)
6. [User Management](#user-management)
7. [Resource Management](#resource-management)
8. [Monitoring & Maintenance](#monitoring--maintenance)
9. [Security](#security)
10. [Troubleshooting](#troubleshooting)
11. [Production Deployment](#production-deployment)

---

## Introduction

WRD Server's headless mode enables enterprise-grade multi-user RDP deployment without requiring a desktop environment. This makes it ideal for:

- **Cloud VPS hosting** - Run on $5-20/month VPS instances
- **Enterprise VDI** - Replace Citrix/VMware at fraction of cost
- **Container deployment** - Docker/Kubernetes ready
- **Multi-tenant hosting** - Isolate users with cgroups
- **CI/CD with GUI** - Automated GUI testing in pipelines

### Key Features

✅ **Zero GUI dependency** - Runs on minimal Ubuntu Server/Debian
✅ **Multi-user sessions** - Concurrent isolated user sessions
✅ **Direct RDP login** - No local login required (RDP-as-display-manager)
✅ **Resource isolation** - cgroups v2 limits (CPU, memory, processes)
✅ **PAM authentication** - Integrates with existing user management
✅ **systemd integration** - Full lifecycle management
✅ **Embedded portal backend** - No GUI permission dialogs
✅ **Session persistence** - Reconnection support

---

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────────┐
│  Headless Linux Server (No Desktop Environment)              │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  WRD Server (Main Process)                             │ │
│  │  • Login service (listens on TCP:3389)                 │ │
│  │  • PAM authentication                                  │ │
│  │  • Session manager                                     │ │
│  │  • Resource manager (cgroups)                          │ │
│  └────────────┬───────────────────────────────────────────┘ │
│               │                                              │
│               │ Creates per-user sessions                    │
│               ▼                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  User Session #1 (user: alice, port: 3389)             │ │
│  │  ┌──────────────────────────────────────────────────┐  │ │
│  │  │  Headless Compositor (Smithay)                   │  │ │
│  │  │  • Virtual Wayland display                       │  │ │
│  │  │  • Software rendering (llvmpipe)                 │  │ │
│  │  └──────────────┬───────────────────────────────────┘  │ │
│  │                 │                                        │ │
│  │  ┌──────────────▼───────────────────────────────────┐  │ │
│  │  │  Embedded Portal Backend                         │  │ │
│  │  │  • Auto-grants permissions                       │  │ │
│  │  └──────────────┬───────────────────────────────────┘  │ │
│  │                 │                                        │ │
│  │  ┌──────────────▼───────────────────────────────────┐  │ │
│  │  │  PipeWire (Headless)                             │  │ │
│  │  │  • Captures compositor framebuffer               │  │ │
│  │  └──────────────┬───────────────────────────────────┘  │ │
│  │                 │                                        │ │
│  │  ┌──────────────▼───────────────────────────────────┐  │ │
│  │  │  RDP Server (IronRDP)                            │  │ │
│  │  │  • Encodes video stream                          │  │ │
│  │  │  • Handles input injection                       │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  │                                                          │ │
│  │  Resource Limits: 2GB RAM, 2 CPUs, 256 processes        │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                              │
│  [User Session #2...] [User Session #N...]                   │
└─────────────────────────────────────────────────────────────┘
```

---

## System Requirements

### Minimum Requirements (1-2 users)

- **OS:** Ubuntu 22.04+ / Debian 12+ / RHEL 9+ / Arch Linux
- **CPU:** 2 cores (x86_64 or ARM64)
- **RAM:** 2GB (512MB base + 512MB per session)
- **Disk:** 10GB
- **Network:** 10 Mbps upload per session

### Recommended Requirements (5-10 users)

- **CPU:** 8 cores
- **RAM:** 16GB
- **Disk:** 50GB SSD
- **Network:** 100 Mbps
- **GPU:** Optional (reduces CPU usage by 50%)

### Software Requirements

- **Kernel:** 5.10+ (for cgroups v2)
- **systemd:** 245+
- **PipeWire:** 0.3.50+
- **Rust:** 1.70+ (for compilation)

---

## Installation

### Option 1: Automated Installation (Recommended)

```bash
# Clone repository
git clone https://github.com/lamco-admin/wayland-rdp.git
cd wayland-rdp

# Build with headless features
cargo build --release --features full-headless

# Run installation script
sudo deploy/install-headless.sh
```

### Option 2: Manual Installation

#### Step 1: Install Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y \
    pipewire libpipewire-0.3-dev \
    xdg-desktop-portal \
    libpam0g-dev libsystemd-dev \
    build-essential pkg-config libssl-dev \
    cgroup-tools systemd-container
```

**Fedora/RHEL:**
```bash
sudo dnf install -y \
    pipewire pipewire-devel \
    xdg-desktop-portal \
    pam-devel systemd-devel \
    gcc pkg-config openssl-devel \
    libcgroup-tools
```

#### Step 2: Create System User

```bash
sudo groupadd --system wrd-server
sudo useradd --system \
    --home-dir /var/lib/wrd-server \
    --shell /bin/false \
    --gid wrd-server \
    --groups video,render,input \
    --create-home \
    wrd-server
```

#### Step 3: Build and Install

```bash
cargo build --release --features full-headless
sudo cp target/release/wrd-server /usr/local/bin/
sudo chmod 755 /usr/local/bin/wrd-server
```

#### Step 4: Create Configuration

```bash
sudo mkdir -p /etc/wrd-server
sudo cp examples/headless.toml /etc/wrd-server/
sudo chown root:wrd-server /etc/wrd-server/headless.toml
sudo chmod 640 /etc/wrd-server/headless.toml
```

#### Step 5: Install systemd Service

```bash
sudo cp deploy/systemd/wrd-server-headless.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable wrd-server-headless
```

---

## Configuration

### Basic Configuration (/etc/wrd-server/headless.toml)

```toml
# Network configuration
listen_address = "0.0.0.0:3389"

# Multi-user settings
[multiuser]
enabled = true
max_sessions = 10                # System-wide limit
max_sessions_per_user = 2        # Per-user limit
enable_reconnection = true
idle_timeout = 3600              # 1 hour

# Authentication
[authentication]
provider = "pam"
pam_service = "wrd-server"
max_failed_attempts = 5
lockout_duration = 900           # 15 minutes

# Resource limits (per session)
[resources.session_limits]
max_memory = 2048                # MB
cpu_quota = 200                  # 200% = 2 full cores
max_processes = 256
max_files = 4096

# Compositor settings
[compositor]
default_resolution = [1920, 1080]
refresh_rate = 60
render_backend = "llvmpipe"      # Software rendering
```

### Advanced Configuration Options

See [Configuration Reference](CONFIG-REFERENCE.md) for complete options.

---

## User Management

### Adding Users

Users are managed through standard Linux user accounts:

```bash
# Create user
sudo useradd -m -s /bin/bash alice

# Set password
sudo passwd alice

# Grant RDP access (optional group-based)
sudo usermod -aG rdp-users alice
```

### Per-User Policies

Configure per-user permissions in `/etc/wrd-server/headless.toml`:

```toml
[portal.permission_policy.user_policies.alice]
allow_screencast = true
allow_remote_desktop = true
allow_clipboard = true
allowed_apps = ["firefox", "libreoffice"]
```

### Quota Management

Set per-user disk quotas:

```bash
sudo setquota -u alice 5G 10G 0 0 /home
```

---

## Resource Management

### cgroups v2 Isolation

Each user session is isolated using cgroups v2:

- **Memory limits** - Prevent OOM killing other sessions
- **CPU quotas** - Fair CPU scheduling
- **Process limits** - Prevent fork bombs
- **I/O limits** - Prevent disk thrashing

### Monitoring Resource Usage

```bash
# View all sessions
systemctl status wrd-server-headless

# View per-session resources
systemd-cgtop

# Check memory usage
cat /sys/fs/cgroup/wrd-server/session-alice-*/memory.current
```

### Adjusting Limits

Edit `/etc/wrd-server/headless.toml`:

```toml
[resources.session_limits]
max_memory = 4096      # Increase to 4GB
cpu_quota = 400        # Increase to 4 cores
```

Then restart:
```bash
sudo systemctl restart wrd-server-headless
```

---

## Monitoring & Maintenance

### Logging

Logs are written to journald:

```bash
# View all logs
sudo journalctl -u wrd-server-headless -f

# View session logs
sudo journalctl -u wrd-server-headless@alice -f

# Filter by priority
sudo journalctl -u wrd-server-headless -p err
```

### Metrics

WRD Server exposes Prometheus metrics on port 9090:

```bash
curl http://localhost:9090/metrics
```

Key metrics:
- `wrd_active_sessions` - Current session count
- `wrd_total_connections` - Total RDP connections
- `wrd_session_memory_bytes` - Memory usage per session
- `wrd_session_cpu_seconds` - CPU usage per session

### Health Checks

```bash
# Check service health
systemctl is-active wrd-server-headless

# Check RDP port
nc -zv localhost 3389

# Check session count
systemctl list-units 'wrd-server-headless@*' | grep running | wc -l
```

---

## Security

### TLS Configuration

Generate self-signed certificate:

```bash
openssl req -x509 -newkey rsa:4096 \
    -keyout /etc/wrd-server/key.pem \
    -out /etc/wrd-server/cert.pem \
    -days 365 -nodes \
    -subj "/CN=rdp.example.com"
```

Or use Let's Encrypt:

```bash
sudo certbot certonly --standalone -d rdp.example.com
ln -s /etc/letsencrypt/live/rdp.example.com/fullchain.pem /etc/wrd-server/cert.pem
ln -s /etc/letsencrypt/live/rdp.example.com/privkey.pem /etc/wrd-server/key.pem
```

### Firewall Rules

```bash
# Allow RDP
sudo ufw allow 3389/tcp

# Allow user session ports
sudo ufw allow 3389:3489/tcp

# Enable firewall
sudo ufw enable
```

### PAM Configuration

Configure authentication in `/etc/pam.d/wrd-server`:

```pam
auth       required     pam_env.so
auth       required     pam_unix.so
auth       required     pam_faillock.so preauth
account    required     pam_unix.so
account    required     pam_faillock.so
session    required     pam_unix.so
session    required     pam_limits.so
```

---

## Troubleshooting

### Service Won't Start

```bash
# Check service status
sudo systemctl status wrd-server-headless

# Check logs
sudo journalctl -u wrd-server-headless -n 50

# Verify configuration
wrd-server --config /etc/wrd-server/headless.toml --check-config
```

### Cannot Connect

```bash
# Check if port is listening
sudo netstat -tlnp | grep 3389

# Check firewall
sudo ufw status

# Test locally
xfreerdp /v:localhost:3389 /u:alice /p:password
```

### Session Crashes

```bash
# View crash logs
sudo journalctl -u 'wrd-server-headless@*' -p err

# Check resource limits
systemd-cgtop

# Increase memory limit if needed
```

### Performance Issues

```bash
# Check CPU usage
top

# Check memory pressure
cat /sys/fs/cgroup/wrd-server/memory.pressure

# Enable hardware acceleration (if GPU available)
# Edit /etc/wrd-server/headless.toml:
# render_backend = "virgl"
```

---

## Production Deployment

### High Availability Setup

Use load balancer (HAProxy/Nginx) for multiple WRD servers:

```haproxy
frontend rdp_frontend
    bind *:3389
    mode tcp
    default_backend rdp_servers

backend rdp_servers
    mode tcp
    balance leastconn
    server wrd1 10.0.1.10:3389 check
    server wrd2 10.0.1.11:3389 check
    server wrd3 10.0.1.12:3389 check
```

### Auto-Scaling

Use systemd service templates for dynamic session creation.

### Backup Strategy

```bash
# Backup configuration
tar -czf wrd-server-backup.tar.gz /etc/wrd-server/

# Backup user data
tar -czf user-data-backup.tar.gz /var/lib/wrd-server/

# Backup to remote storage
rclone sync /etc/wrd-server/ remote:wrd-backups/
```

### Monitoring Integration

- **Prometheus + Grafana** - Metrics and dashboards
- **ELK Stack** - Log aggregation
- **Netdata** - Real-time monitoring

---

## Support & Resources

- **Documentation:** https://github.com/lamco-admin/wayland-rdp/docs
- **Issues:** https://github.com/lamco-admin/wayland-rdp/issues
- **Community:** Discord/Matrix channels
- **Commercial Support:** Available for enterprise deployments

---

**Version:** 1.0
**Last Updated:** 2025-11-19
**License:** MIT OR Apache-2.0
