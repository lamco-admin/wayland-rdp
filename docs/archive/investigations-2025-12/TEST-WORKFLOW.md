# KDE VM Testing Workflow

**Test VM:** 192.168.10.205 (KDE Plasma + Wayland)
**Scripts Ready:** ✅

---

## Quick Start (Easiest)

### One-Command Deploy and Run

```bash
./quick-deploy.sh
```

This will:
1. Build the server
2. Deploy to KDE VM
3. SSH into VM and start server
4. Show live logs

**That's it!** Server will be running on the VM.

---

## Step-by-Step (More Control)

### 1. Deploy to KDE VM

```bash
./test-kde.sh deploy
```

This builds and copies everything to the VM.

### 2. SSH into VM

```bash
./test-kde.sh ssh
```

Or manually:
```bash
ssh greg@192.168.10.205
```

### 3. Run Server on VM

```bash
cd lamco-rdp-server-test
./run-server.sh
```

### 4. Test from RDP Client

From your workstation or another machine:

**Linux (Remmina):**
```bash
remmina -c rdp://192.168.10.205:3389
```

**Linux (xfreerdp):**
```bash
xfreerdp /v:192.168.10.205:3389 /u:greg
```

**Windows:**
- Remote Desktop Connection
- Computer: `192.168.10.205:3389`

---

## Script Reference

### test-kde.sh Commands

```bash
./test-kde.sh build    # Build locally
./test-kde.sh run      # Run locally
./test-kde.sh deploy   # Deploy to VM
./test-kde.sh ssh      # SSH to VM
./test-kde.sh logs     # Fetch logs from VM
```

### run-server.sh (on VM)

Automatically:
- ✅ Checks D-Bus session
- ✅ Starts PipeWire if needed
- ✅ Validates config and certs
- ✅ Runs with comprehensive logging
- ✅ Saves to timestamped log file

---

## What to Watch For

### Successful Startup

Look for these log lines:

```
INFO lamco_portal: Portal manager created
INFO lamco_portal: Portal session created successfully
INFO lamco_pipewire: PipeWire thread manager created
INFO lamco_pipewire: Stream 42 created successfully
INFO lamco_rdp_server::server: WRD Server initialized successfully
INFO lamco_rdp_server::server: Server is ready and listening
INFO lamco_rdp_server::server: Waiting for clients to connect...
```

### Portal Permission Dialog

**IMPORTANT:** On first run, KDE will show a permission dialog:

```
"lamco-rdp-server wants to share your screen"
[Deny] [Share]
```

**Click "Share"!**

If you see this in logs:
```
ERROR lamco_portal: Failed to create portal session
ERROR lamco_portal: User denied screen sharing permission
```

Re-run and grant permission.

### Client Connection

When client connects:
```
INFO ironrdp_server: New connection from 192.168.1.100:54321
INFO ironrdp_server: TLS handshake successful
INFO ironrdp_server: Client connected successfully
INFO lamco_rdp_server::server::display_handler: Processing frame 1
```

---

## Troubleshooting

### Can't SSH to VM

```bash
ping 192.168.10.205
```

If no response, VM might be down or IP changed.

### "Permission denied" errors

Make sure you're logged into the KDE desktop session on the VM (not just SSH).
Portal requires an active Wayland session.

### "PipeWire connection failed"

On the VM:
```bash
systemctl --user status pipewire
systemctl --user start pipewire
```

### "Portal permission denied"

Grant permission when dialog appears, or check:
```bash
# On VM
qdbus org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop
```

---

## Log Files

Logs are saved on the VM:
```
~/lamco-rdp-server-test/kde-test-YYYYMMDD-HHMMSS.log
~/lamco-rdp-server-test/console-output.log (latest)
```

Fetch logs:
```bash
./test-kde.sh logs
```

---

## After Testing

### Stop Server

On VM (in SSH session):
- Press `Ctrl+C`

### Fetch Logs for Analysis

```bash
./test-kde.sh logs
```

### Clean Up VM

```bash
ssh greg@192.168.10.205
rm -rf lamco-rdp-server-test
```

---

## Full Test Checklist

- [ ] Deploy to VM: `./test-kde.sh deploy`
- [ ] SSH to VM: `./test-kde.sh ssh`
- [ ] Start server: `cd lamco-rdp-server-test && ./run-server.sh`
- [ ] Watch for "Server is ready and listening"
- [ ] Grant Portal permission if dialog appears
- [ ] Connect from RDP client
- [ ] Watch for "Client connected successfully"
- [ ] Test video: Move windows, see if visible
- [ ] Test input: Type in text editor, click buttons
- [ ] Test clipboard: Copy/paste between client and server
- [ ] Stop server: Ctrl+C
- [ ] Fetch logs: `./test-kde.sh logs`
- [ ] Analyze logs for errors

---

**Ready to test!**

Use `./quick-deploy.sh` for fastest path to running server.
