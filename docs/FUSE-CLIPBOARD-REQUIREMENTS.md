# FUSE Clipboard Requirements

This document describes the requirements for FUSE-based clipboard file transfer in lamco-rdp-server.

## Overview

The FUSE clipboard feature provides on-demand file transfer from Windows RDP clients to Linux. Instead of downloading entire files upfront, files are transferred as the user accesses them through a virtual FUSE filesystem.

## Deployment Modes

### Flatpak (Staging Fallback)

**FUSE is not supported in Flatpak sandboxes.** The application gracefully falls back to staging mode:

- Files are downloaded upfront to a staging directory
- Full functionality is preserved, just less bandwidth-efficient
- No configuration required

This is expected behavior and documented in the Flatpak manifest.

### Native/Systemd Deployment (Full FUSE Support)

For native installations, FUSE provides on-demand file streaming.

#### Requirements

1. **FUSE3 Library**: libfuse3 must be installed
   ```bash
   # Debian/Ubuntu
   sudo apt install libfuse3-3 fuse3

   # Fedora/RHEL
   sudo dnf install fuse3 fuse3-libs

   # Arch Linux
   sudo pacman -S fuse3
   ```

2. **FUSE Configuration**: `/etc/fuse.conf` must enable `user_allow_other`
   ```ini
   # /etc/fuse.conf
   # Allow non-root users to use allow_other mount option
   user_allow_other

   # Optional: increase mount limit if needed
   # mount_max = 1000
   ```

3. **fusermount3 Permissions**: The `fusermount3` binary must be setuid root or have capabilities
   ```bash
   # Verify permissions
   ls -la /usr/bin/fusermount3
   # Expected: -rwsr-xr-x 1 root root ... /usr/bin/fusermount3
   #           ^ setuid bit

   # If not setuid, fix with:
   sudo chmod u+s /usr/bin/fusermount3
   ```

4. **User Group Membership** (some distributions):
   ```bash
   # Add user to fuse group if required
   sudo usermod -a -G fuse $USER
   # Log out and back in for group changes to take effect
   ```

#### Verification

Test FUSE access:
```bash
# Check FUSE module is loaded
lsmod | grep fuse

# Check fusermount3 is accessible
which fusermount3

# Check config
cat /etc/fuse.conf | grep user_allow_other
```

## How It Works

1. **Mount Point**: Virtual filesystem mounted at `$XDG_RUNTIME_DIR/wrd-clipboard-fuse/`

2. **File Listing**: When Windows client copies files, file metadata appears in the mount

3. **On-Demand Transfer**: File contents are fetched from Windows client only when read

4. **Automatic Unmount**: FUSE filesystem unmounts cleanly when session ends

## Troubleshooting

### "Failed to mount FUSE: No such file or directory"

**Cause**: fusermount3 not found in PATH or /dev/fuse not accessible

**Solution**:
```bash
# Ensure fuse3 is installed
which fusermount3

# Ensure FUSE kernel module is loaded
sudo modprobe fuse
```

### "option allow_other only allowed if 'user_allow_other' is set"

**Cause**: /etc/fuse.conf missing `user_allow_other` directive

**Solution**: Add `user_allow_other` to /etc/fuse.conf (see above)

### "Permission denied" on fusermount3

**Cause**: fusermount3 missing setuid bit or user not in fuse group

**Solution**:
```bash
sudo chmod u+s /usr/bin/fusermount3
# or
sudo usermod -a -G fuse $USER
```

### Staging Mode Fallback

If FUSE mount fails, the server automatically falls back to staging mode with this log message:
```
WARN: Failed to mount FUSE clipboard filesystem: ...
WARN: File clipboard will use staging fallback (download files upfront)
```

This is safe and provides full functionality with slightly more bandwidth usage.

## Security Considerations

- FUSE mounts are user-specific and not accessible to other users
- The `allow_other` option is used to allow file managers to access clipboard files
- Files are read-only in the FUSE mount
- Mount automatically unmounts when the RDP session ends

## AppArmor/SELinux

If running with AppArmor or SELinux, ensure the profile allows:
- Access to `/dev/fuse`
- Execution of `fusermount3`
- Access to `/run/mount/utab` (for fusermount3 bookkeeping)

For AppArmor on Ubuntu, the default `fusermount3` profile should work. If issues occur:
```bash
# Check for denials
sudo dmesg | grep -i apparmor | grep fusermount

# Temporarily disable for testing
sudo aa-complain /etc/apparmor.d/fusermount3
```
