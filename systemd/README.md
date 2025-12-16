# Systemd Service Units

This directory contains systemd service units for the WRD system.

## Services

### wrd-login.service

Main login service daemon that:
- Listens on port 3389 for RDP connections
- Authenticates users via PAM
- Creates systemd-logind sessions
- Spawns per-user compositors

**Installation:**
```bash
sudo cp wrd-login.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable wrd-login.service
sudo systemctl start wrd-login.service
```

### wrd-compositor@.service

Template service for per-user compositors.

This service is typically started automatically by the login service,
but can also be started manually:

```bash
sudo systemctl start wrd-compositor@username.service
```

## Configuration

Create `/etc/wrd-login/config.toml` with appropriate settings.
See `docs/configuration.md` for details.

## Logs

View logs with:
```bash
# Login service logs
sudo journalctl -u wrd-login.service -f

# Compositor logs for specific user
sudo journalctl -u wrd-compositor@username.service -f
```

## Security

The services are configured with security hardening:
- `NoNewPrivileges=yes` - prevent privilege escalation
- `PrivateTmp=yes` - private /tmp
- `ProtectSystem=strict` - read-only system directories
- Resource limits - memory, CPU, tasks

## Troubleshooting

If services fail to start, check:
1. Configuration file exists and is valid
2. Certificates are in place
3. systemd-logind is running
4. Port 3389 is available

Check status:
```bash
sudo systemctl status wrd-login.service
```

View detailed errors:
```bash
sudo journalctl -xeu wrd-login.service
```
