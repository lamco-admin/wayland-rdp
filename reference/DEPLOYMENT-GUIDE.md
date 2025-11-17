# DEPLOYMENT GUIDE
**Document:** DEPLOYMENT-GUIDE.md
**Version:** 1.0

## PRODUCTION DEPLOYMENT GUIDE

### PREREQUISITES

#### System Requirements
- Linux kernel 6.0+
- Wayland compositor (GNOME 45+, KDE 6+, or Sway 1.8+)
- PipeWire 0.3.77+
- 2GB RAM minimum (4GB recommended)
- GPU with VA-API support (optional but recommended)

#### Network Requirements
- TCP port 3389 accessible
- TLS 1.3 support
- Firewall configured

### INSTALLATION

#### 1. Install System Dependencies
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    libwayland-client0 \
    libpipewire-0.3-0 \
    libva2 \
    libpam0g \
    xdg-desktop-portal \
    xdg-desktop-portal-gtk  # or -kde, -wlr depending on compositor

# Verify PipeWire
systemctl --user status pipewire

# Verify portal
systemctl --user status xdg-desktop-portal
```

#### 2. Install Binary
```bash
# Option A: From release
wget https://github.com/your-org/wrd-server/releases/latest/wrd-server
sudo install -m 755 wrd-server /usr/local/bin/

# Option B: Build from source
git clone https://github.com/your-org/wrd-server
cd wrd-server
cargo build --release
sudo install -m 755 target/release/wrd-server /usr/local/bin/
```

#### 3. Create User and Directories
```bash
# Create dedicated user
sudo useradd -r -s /bin/false wrd-server

# Create directories
sudo mkdir -p /etc/wrd-server
sudo mkdir -p /var/log/wrd-server
sudo chown wrd-server:wrd-server /var/log/wrd-server
```

#### 4. Generate Certificates
```bash
# Production: Use real certificates
sudo openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout /etc/wrd-server/key.pem \
    -out /etc/wrd-server/cert.pem \
    -days 365 \
    -subj "/CN=your-server.example.com"

# Set permissions
sudo chmod 644 /etc/wrd-server/cert.pem
sudo chmod 600 /etc/wrd-server/key.pem
sudo chown wrd-server:wrd-server /etc/wrd-server/*.pem
```

#### 5. Create Configuration
```bash
sudo tee /etc/wrd-server/config.toml << EOF
[server]
listen_addr = "0.0.0.0:3389"
max_connections = 10

[security]
cert_path = "/etc/wrd-server/cert.pem"
key_path = "/etc/wrd-server/key.pem"
enable_nla = true
auth_method = "pam"

[video]
encoder = "auto"
target_fps = 30
bitrate = 4000

[logging]
level = "info"
log_dir = "/var/log/wrd-server"
EOF
```

#### 6. Configure PAM
```bash
sudo tee /etc/pam.d/wrd-server << EOF
auth    required    pam_unix.so
account required    pam_unix.so
EOF
```

#### 7. Install Systemd Service
```bash
sudo tee /etc/systemd/system/wrd-server.service << EOF
[Unit]
Description=Wayland Remote Desktop Server
After=network.target graphical.target
Requires=dbus.service

[Service]
Type=simple
User=wrd-server
Group=wrd-server
ExecStart=/usr/local/bin/wrd-server --config /etc/wrd-server/config.toml
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/wrd-server

# Environment
Environment="RUST_LOG=info"
Environment="PIPEWIRE_LATENCY=512/48000"

[Install]
WantedBy=multi-user.target
EOF

# Reload and enable
sudo systemctl daemon-reload
sudo systemctl enable wrd-server
```

#### 8. Configure Firewall
```bash
# UFW
sudo ufw allow 3389/tcp

# firewalld
sudo firewall-cmd --permanent --add-port=3389/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 3389 -j ACCEPT
```

#### 9. Start Service
```bash
sudo systemctl start wrd-server
sudo systemctl status wrd-server
```

### VERIFICATION

```bash
# Check service status
systemctl status wrd-server

# View logs
journalctl -u wrd-server -f

# Test connection
# From Windows: mstsc /v:server-ip:3389

# Check listening port
ss -tlnp | grep 3389
```

### MONITORING

#### Log Management
```bash
# View logs
sudo journalctl -u wrd-server -f

# Log rotation (automatic with journald)
# Or configure logrotate
sudo tee /etc/logrotate.d/wrd-server << EOF
/var/log/wrd-server/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    create 0644 wrd-server wrd-server
}
EOF
```

#### Performance Monitoring
```bash
# CPU/Memory usage
top -p $(pgrep wrd-server)

# GPU usage (Intel)
intel_gpu_top

# GPU usage (AMD)
radeontop
```

### SECURITY HARDENING

#### 1. Use Strong Certificates
```bash
# Use Let's Encrypt or organization CA
# Update cert_path and key_path in config
```

#### 2. Enable NLA
```toml
[security]
enable_nla = true
```

#### 3. Configure Fail2ban
```bash
sudo tee /etc/fail2ban/filter.d/wrd-server.conf << EOF
[Definition]
failregex = ^.*Authentication failed for user.*<HOST>.*$
ignoreregex =
EOF

sudo tee /etc/fail2ban/jail.d/wrd-server.conf << EOF
[wrd-server]
enabled = true
port = 3389
filter = wrd-server
logpath = /var/log/wrd-server/*.log
maxretry = 3
bantime = 3600
EOF

sudo systemctl restart fail2ban
```

#### 4. SELinux/AppArmor
```bash
# SELinux
# Create policy (example)
# AppArmor
# Create profile in /etc/apparmor.d/usr.local.bin.wrd-server
```

### TROUBLESHOOTING

#### Service won't start
```bash
# Check logs
journalctl -u wrd-server -n 50

# Common issues:
# - Certificate files not found
# - Port already in use
# - PipeWire not running
# - Portal not available
```

#### Client can't connect
```bash
# Check firewall
sudo ufw status

# Check service listening
ss -tlnp | grep 3389

# Test TLS
openssl s_client -connect localhost:3389 -tls1_3
```

#### Poor performance
```bash
# Check encoder
# Add to config:
[video]
encoder = "vaapi"  # Force hardware encoding

# Check GPU
vainfo

# Monitor performance
top -p $(pgrep wrd-server)
```

### BACKUP AND RESTORE

#### Backup
```bash
# Backup configuration
sudo tar czf wrd-server-backup.tar.gz \
    /etc/wrd-server/ \
    /etc/systemd/system/wrd-server.service
```

#### Restore
```bash
sudo tar xzf wrd-server-backup.tar.gz -C /
sudo systemctl daemon-reload
sudo systemctl restart wrd-server
```

### UPGRADING

```bash
# Stop service
sudo systemctl stop wrd-server

# Backup
sudo cp /usr/local/bin/wrd-server /usr/local/bin/wrd-server.bak

# Install new version
sudo install -m 755 wrd-server /usr/local/bin/

# Restart
sudo systemctl start wrd-server

# Verify
sudo systemctl status wrd-server
```

### HIGH AVAILABILITY

#### Load Balancing
- Use HAProxy or nginx stream module
- Round-robin to multiple wrd-server instances
- Each instance on different physical machine

#### Failover
- Primary/secondary setup
- Keepalived for VIP management
- Automatic failover on primary failure

### DOCKER DEPLOYMENT

```dockerfile
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    libwayland-client0 \
    libpipewire-0.3-0 \
    libva2 \
    libpam0g

COPY wrd-server /usr/local/bin/
COPY config.toml /etc/wrd-server/

EXPOSE 3389

ENTRYPOINT ["/usr/local/bin/wrd-server"]
CMD ["--config", "/etc/wrd-server/config.toml"]
```

```bash
docker build -t wrd-server .
docker run -d \
    --name wrd-server \
    --network host \
    -v /etc/wrd-server:/etc/wrd-server:ro \
    wrd-server
```

### KUBERNETES DEPLOYMENT

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wrd-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: wrd-server
  template:
    metadata:
      labels:
        app: wrd-server
    spec:
      containers:
      - name: wrd-server
        image: wrd-server:latest
        ports:
        - containerPort: 3389
        volumeMounts:
        - name: config
          mountPath: /etc/wrd-server
      volumes:
      - name: config
        configMap:
          name: wrd-server-config
---
apiVersion: v1
kind: Service
metadata:
  name: wrd-server
spec:
  type: LoadBalancer
  ports:
  - port: 3389
    targetPort: 3389
  selector:
    app: wrd-server
```

## END OF DEPLOYMENT GUIDE
