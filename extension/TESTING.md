# Testing the Wayland RDP Clipboard Extension

## Prerequisites

- GNOME Shell 45+ on Wayland
- Test VM: 192.168.10.205 (Ubuntu 24.04.3 + GNOME 46.2)

## Installation

```bash
# On the GNOME VM
cd /home/greg/wayland/wrd-server-specs/extension

# Run install script
./install.sh

# Enable the extension
gnome-extensions enable wayland-rdp-clipboard@wayland-rdp.io

# Restart GNOME Shell
# Wayland: Log out and log back in
# X11: Alt+F2 -> r -> Enter
```

## Verify Installation

```bash
# Check extension is listed
gnome-extensions list | grep wayland-rdp

# Check extension info
gnome-extensions info wayland-rdp-clipboard@wayland-rdp.io

# Check D-Bus service is available
busctl --user list | grep wayland_rdp
```

## Test D-Bus Interface

### Monitor Clipboard Signals

Open a terminal and run:

```bash
gdbus monitor --session \
  --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard
```

Then copy text in any application. You should see:

```
/org/wayland_rdp/Clipboard: org.wayland_rdp.Clipboard.ClipboardChanged (['text/plain', 'text/plain;charset=utf-8', 'UTF8_STRING', 'STRING'], 'a1b2c3d4')
```

### Test Methods

```bash
# Ping test
gdbus call --session \
  --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.Ping "hello"
# Expected: ('pong: hello',)

# Get version
gdbus call --session \
  --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.GetVersion
# Expected: ('1.0.0',)

# Get clipboard text (copy something first)
gdbus call --session \
  --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.GetText
# Expected: ('your copied text',)

# Get settings
gdbus call --session \
  --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.GetSettings
```

## Test Configuration

```bash
# View current settings
gsettings list-recursively org.gnome.shell.extensions.wayland-rdp-clipboard

# Change poll interval
gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard poll-interval 250

# Enable debug logging
gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard log-level 'debug'

# View logs
journalctl -f -o cat /usr/bin/gnome-shell | grep wayland-rdp-clipboard
```

## Test PRIMARY Selection

1. Start monitoring:
   ```bash
   gdbus monitor --session --dest org.wayland_rdp.Clipboard --object-path /org/wayland_rdp/Clipboard
   ```

2. Select text with mouse (don't Ctrl+C, just highlight)

3. Should see `PrimaryChanged` signal

4. Disable if not needed:
   ```bash
   gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard monitor-primary false
   ```

## Test MIME Type Detection

Copy different content types and verify MIME detection:

| Content | Expected MIME Types |
|---------|---------------------|
| Plain text | `text/plain`, `UTF8_STRING` |
| `<div>HTML</div>` | Above + `text/html` |
| `https://example.com` | Above + `text/uri-list` |
| `/home/user/file.txt` | Above + `text/uri-list` |

## Troubleshooting

### Extension not loading

```bash
# Check for errors in GNOME Shell logs
journalctl -f -o cat /usr/bin/gnome-shell 2>&1 | grep -i error

# Try reinstalling
gnome-extensions disable wayland-rdp-clipboard@wayland-rdp.io
rm -rf ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io
./install.sh
gnome-extensions enable wayland-rdp-clipboard@wayland-rdp.io
# Log out/in
```

### Schema not found

```bash
# Recompile schemas
cd ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io
glib-compile-schemas schemas/
```

### D-Bus name not appearing

```bash
# Check if extension is actually enabled
gnome-extensions show wayland-rdp-clipboard@wayland-rdp.io

# Check logs for D-Bus errors
journalctl -f -o cat /usr/bin/gnome-shell | grep -i dbus
```

## Integration Test with wrd-server

Once the extension is working, test with the RDP server:

1. Start wrd-server with clipboard enabled
2. Connect from Windows RDP client
3. Copy text on Linux side
4. Paste on Windows side - should work now!

## Success Criteria

- [ ] Extension installs without errors
- [ ] D-Bus service `org.wayland_rdp.Clipboard` appears on session bus
- [ ] `ClipboardChanged` signal emits when copying text
- [ ] `PrimaryChanged` signal emits when selecting text
- [ ] `GetText` method returns clipboard contents
- [ ] Settings changes take effect (poll interval, log level)
- [ ] No errors in GNOME Shell logs
