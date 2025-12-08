# WRD-Server Quick Reference

## TL;DR - Run the Server

```bash
# Local development (from project root)
./target/release/wrd-server -c config.toml

# Via SSH (GNOME VM)
ssh greg@192.168.10.205 'export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus" && cd ~/wayland/wrd-server-specs && ./target/release/wrd-server -c config.toml'
```

## Why `-c config.toml`?

The default config path is `/etc/wrd-server/config.toml` (for production).
During development, use `-c config.toml` to load the local config file which has:
- Relative cert paths: `certs/cert.pem`, `certs/key.pem`
- NLA disabled for testing
- Debug-friendly settings

## Setup Checklist

1. **Certs exist?**
   ```bash
   ls certs/cert.pem certs/key.pem
   # If missing:
   cp certs/test-cert.pem certs/cert.pem
   cp certs/test-key.pem certs/key.pem
   ```

2. **Built?**
   ```bash
   cargo build --release
   ```

3. **GNOME extension installed?** (for clipboard)
   ```bash
   cd extension && ./install.sh
   # Then log out/in to activate
   ```

4. **D-Bus available?** (for SSH sessions)
   ```bash
   export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
   ```

## Common Issues

| Issue | Solution |
|-------|----------|
| TLS permission denied | Use `-c config.toml` (not default path) |
| Portal not responding | Export DBUS_SESSION_BUS_ADDRESS |
| Clipboard not working | Install & enable GNOME extension, log out/in |
| Connection refused | Accept the screen share dialog on VM |

## Config File Locations

| Environment | Config Path |
|-------------|-------------|
| Development | `./config.toml` (use `-c config.toml`) |
| Production | `/etc/wrd-server/config.toml` (default) |
| Certs (dev) | `./certs/cert.pem`, `./certs/key.pem` |
| Certs (prod) | `/etc/wrd-server/cert.pem`, `/etc/wrd-server/key.pem` |

## VM Deployment

The test VM (192.168.10.205) has a git clone at `~/wayland-rdp/`.

### Deploy New Code

```bash
# 1. From local machine - commit and push changes
git add -A && git commit -m "your message" && git push origin feature/gnome-clipboard-extension

# 2. On VM - pull and build (run these on the VM, not via SSH for long builds)
cd ~/wayland-rdp
git pull origin feature/gnome-clipboard-extension
cargo build --release

# 3. Run the server
pkill -f wrd-server
export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
./target/release/wrd-server -c config.toml
```

### Quick SSH Commands (for short operations only)

```bash
# Check current branch/commit
ssh greg@192.168.10.205 'cd ~/wayland-rdp && git log -1 --oneline'

# Pull latest
ssh greg@192.168.10.205 'cd ~/wayland-rdp && git pull'

# NOTE: Don't run cargo build via SSH - it times out. Use tmux/screen on VM instead.
```

### VM Directory Structure

| Path | Purpose |
|------|---------|
| `~/wayland-rdp/` | Git clone (use for deployment) |
| `~/wayland/wrd-server-specs/` | Old rsync copy (deprecated, don't use) |
