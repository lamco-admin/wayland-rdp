# Wayland RDP Clipboard Bridge

GNOME Shell extension that exposes clipboard changes via D-Bus for [wayland-rdp-server](https://github.com/anthropics/wayland-rdp-server) and other applications.

## Why This Extension Exists

GNOME's Portal implementation does not emit `SelectionOwnerChanged` signals, making it impossible for external applications to detect when a user copies something on the Linux side. This extension bridges that gap by:

1. Monitoring GNOME's internal clipboard (`St.Clipboard`)
2. Exposing changes via a D-Bus interface
3. Allowing external applications to subscribe to clipboard events

## Requirements

- GNOME Shell 45, 46, 47, or 48
- Wayland session (X11 works but defeats the purpose)

## Installation

### From Source (Development)

```bash
# Clone or navigate to the extension directory
cd extension/

# Compile the GSettings schema
glib-compile-schemas schemas/

# Create extension directory
mkdir -p ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io

# Copy files
cp extension.js metadata.json ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io/
cp -r schemas ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io/

# Enable the extension
gnome-extensions enable wayland-rdp-clipboard@wayland-rdp.io

# Restart GNOME Shell (Wayland: log out/in, X11: Alt+F2 -> r -> Enter)
```

### From extensions.gnome.org (Coming Soon)

Visit [extensions.gnome.org](https://extensions.gnome.org) and search for "Wayland RDP Clipboard".

## D-Bus Interface

**Service**: `org.wayland_rdp.Clipboard`
**Object Path**: `/org/wayland_rdp/Clipboard`
**Interface**: `org.wayland_rdp.Clipboard`

### Signals

| Signal | Arguments | Description |
|--------|-----------|-------------|
| `ClipboardChanged` | `(as mime_types, s content_hash)` | Emitted when CLIPBOARD selection changes |
| `PrimaryChanged` | `(as mime_types, s content_hash)` | Emitted when PRIMARY selection changes |

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `GetText()` | `() -> s` | Get current clipboard text |
| `GetPrimaryText()` | `() -> s` | Get current primary selection text |
| `GetMimeTypes()` | `() -> as` | Get supported MIME types |
| `GetVersion()` | `() -> s` | Get extension version |
| `GetSettings()` | `() -> a{sv}` | Get current settings |
| `Ping(s)` | `(s) -> s` | Test connectivity |

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `PollInterval` | `u` | Current polling interval in ms |
| `IsMonitoring` | `b` | Whether monitoring is active |

## Configuration

Settings are stored in GSettings and can be modified with `gsettings` or `dconf`:

```bash
# View all settings
gsettings list-recursively org.gnome.shell.extensions.wayland-rdp-clipboard

# Set poll interval (100-5000ms)
gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard poll-interval 500

# Disable PRIMARY selection monitoring
gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard monitor-primary false

# Enable debug logging
gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard log-level 'debug'
```

### Available Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `poll-interval` | uint | 500 | Clipboard check interval (ms) |
| `monitor-clipboard` | bool | true | Monitor CLIPBOARD selection |
| `monitor-primary` | bool | true | Monitor PRIMARY selection |
| `log-level` | string | 'info' | Logging: none/error/info/debug |
| `emit-on-empty` | bool | false | Emit signals for empty clipboard |
| `deduplicate-window` | uint | 100 | Ignore rapid changes within this window (ms) |

## Testing

### Monitor D-Bus Signals

```bash
# Watch for clipboard changes
gdbus monitor --session --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard

# Copy some text and watch the signal appear
```

### Call D-Bus Methods

```bash
# Ping the extension
gdbus call --session --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.Ping "hello"

# Get clipboard text
gdbus call --session --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.GetText

# Get version
gdbus call --session --dest org.wayland_rdp.Clipboard \
  --object-path /org/wayland_rdp/Clipboard \
  --method org.wayland_rdp.Clipboard.GetVersion
```

### View Logs

```bash
# Follow GNOME Shell logs
journalctl -f -o cat /usr/bin/gnome-shell | grep wayland-rdp-clipboard
```

## Integration with wayland-rdp-server

The `wayland-rdp-server` automatically connects to this extension when available:

1. On startup, checks if `org.wayland_rdp.Clipboard` is available on the session bus
2. Subscribes to `ClipboardChanged` signal
3. When signal received, reads clipboard via Portal API and sends to RDP client

If the extension is not installed, Linux-to-Windows clipboard will not work (Windows-to-Linux still works via Portal).

## Troubleshooting

### Extension not appearing

```bash
# Check if extension is recognized
gnome-extensions list | grep wayland-rdp

# Check for errors
gnome-extensions info wayland-rdp-clipboard@wayland-rdp.io
```

### D-Bus service not available

```bash
# Check if name is registered
busctl --user list | grep wayland_rdp

# If not, check extension is enabled and GNOME Shell logs for errors
```

### Schema errors

```bash
# Recompile schemas
cd ~/.local/share/gnome-shell/extensions/wayland-rdp-clipboard@wayland-rdp.io
glib-compile-schemas schemas/
```

## Development

### File Structure

```
extension/
├── extension.js                    # Main extension code
├── metadata.json                   # Extension metadata
├── schemas/
│   └── org.gnome.shell.extensions.wayland-rdp-clipboard.gschema.xml
└── README.md
```

### Building for Release

```bash
# Create distributable zip
cd extension
zip -r wayland-rdp-clipboard@wayland-rdp.io.zip \
  extension.js metadata.json schemas/
```

## License

MIT OR Apache-2.0 (same as wayland-rdp-server)

## Related Projects

- [wayland-rdp-server](https://github.com/anthropics/wayland-rdp-server) - The main RDP server this extension supports
- [GNOME Shell Extensions](https://extensions.gnome.org) - Official extension repository
