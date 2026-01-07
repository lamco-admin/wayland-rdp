lamco-rdp-server Portable Test Package
======================================

This package contains everything needed to test lamco-rdp-server on multiple
VMs with compatible glibc (2.34+).

CONTENTS
--------
  config.toml       - Configuration (uses relative paths)
  run-server.sh     - Run script with preflight checks
  certs/            - Test TLS certificates
  logs/             - Log output directory (auto-created)
  lamco-rdp-server  - Binary (YOU MUST COPY THIS)

COMPATIBLE SYSTEMS
------------------
Binary built on RHEL 9 (glibc 2.34) runs on:
  - RHEL/CentOS 9+
  - Fedora 34+
  - Ubuntu 22.04+
  - Debian 12+
  - Arch Linux (current)

NOT compatible with:
  - RHEL/CentOS 8 (glibc 2.28)
  - Ubuntu 20.04 (glibc 2.31)
  - Debian 11 (glibc 2.31)

DEPLOYMENT
----------
1. Copy entire test-package directory to target VM:

   rsync -av test-package/ user@target-vm:~/test-package/

2. Copy the binary from the RHEL build machine:

   scp greg@192.168.10.6:lamco-rdp-server/target/release/lamco-rdp-server \
       user@target-vm:~/test-package/

3. On target VM, ensure you're in a Wayland graphical session (not SSH)

4. Run:

   cd ~/test-package
   ./run-server.sh

   Or to just check capabilities:

   ./run-server.sh --caps

REQUIREMENTS ON TARGET VM
-------------------------
- Wayland compositor (GNOME, KDE Plasma, etc.)
- XDG Desktop Portal (usually pre-installed)
- PipeWire (usually pre-installed on modern distros)
- User must be logged into graphical session

CONFIGURATION NOTES
-------------------
- AVC444 is DISABLED by default (for compatibility with older systems)
- To enable AVC444, edit config.toml: avc444_enabled = true
- Logging is set to "debug" level for testing
- Certificates are self-signed test certs (will show warning in RDP client)

CONNECTING
----------
From Windows: mstsc.exe /v:<vm-ip>:3389
From Linux:  xfreerdp /v:<vm-ip>:3389 /cert:ignore

LOG FILES
---------
Logs are saved to: ./logs/<hostname>-<timestamp>.log
Copy these back for analysis after testing.

TROUBLESHOOTING
---------------
"Not running in Wayland session"
  - You must run from a graphical terminal, not SSH
  - SSH to the machine, then: export DISPLAY=:0 won't work
  - Use VNC or direct console access to start the server

"Portal not available"
  - Install xdg-desktop-portal and compositor-specific backend
  - GNOME: xdg-desktop-portal-gnome
  - KDE: xdg-desktop-portal-kde

"PipeWire not running"
  - Most modern distros run PipeWire by default
  - Check: systemctl --user status pipewire

"Connection refused"
  - Check firewall: sudo firewall-cmd --add-port=3389/tcp
  - Or: sudo ufw allow 3389/tcp
