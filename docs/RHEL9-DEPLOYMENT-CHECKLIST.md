# RHEL 9 Deployment Checklist - Production Quality

**Purpose:** Ensure clean, reproducible deployments to RHEL 9 test VM
**VM:** greg@192.168.10.6 (RHEL 9.7, GNOME 40.10)
**Build Requirement:** Must build on RHEL 9 (glibc 2.34 compatibility)

---

## Pre-Deployment Checklist

**Local:**
- [ ] All changes committed (`git status` clean)
- [ ] Code compiles locally (`cargo build --release`)
- [ ] All tests pass (`cargo test`)
- [ ] No TODO/FIXME in critical paths

**RHEL 9:**
- [ ] Old deployment completely removed
- [ ] Fresh directory structure
- [ ] Cargo.toml paths will be fixed
- [ ] Config with absolute paths deployed

---

## Step 1: Clean RHEL 9 Completely

```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "rm -rf ~/wayland-build ~/rhel9-test-*.log"
```

**Verify:**
```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "ls ~/wayland-build 2>&1"
# Should say: No such file or directory
```

---

## Step 2: Deploy Source

```bash
cd /home/greg/wayland/wrd-server-specs
./scripts/deploy-to-rhel9.sh
```

**What this does:**
- Rsyncs all 5 source directories
- Creates ~/wayland-build/ structure
- Excludes target/, .git/, logs

**Verify:**
```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "ls ~/wayland-build/"
# Should show: IronRDP lamco-rdp-workspace lamco-wayland openh264-rs wrd-server-specs build.sh
```

---

## Step 3: Fix Cargo.toml Paths

```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 'cd ~/wayland-build/wrd-server-specs && \
  sed -i "s|path = \"/home/greg/wayland/IronRDP|path = \"../IronRDP|g" Cargo.toml && \
  sed -i "s|path = \"/home/greg/IronRDP|path = \"../IronRDP|g" Cargo.toml && \
  sed -i "s|path = \"/home/greg/openh264-rs|path = \"../openh264-rs|g" Cargo.toml && \
  echo "Cargo.toml paths fixed"'
```

**Verify:**
```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "grep 'path.*IronRDP' ~/wayland-build/wrd-server-specs/Cargo.toml | head -3"
# Should show: path = "../IronRDP/crates/ironrdp"
```

---

## Step 4: Deploy Config and Scripts

```bash
# Create proper config
cat > /tmp/rhel9-config.toml << 'EOF'
[server]
listen_addr = "0.0.0.0:3389"

[security]
cert_path = "certs/test-cert.pem"  # Relative to working directory
key_path = "certs/test-key.pem"
enable_nla = false
auth_method = "none"
require_tls_13 = false

[egfx]
h264_bitrate = 8000
qp_min = 10
qp_max = 28
qp_default = 20
avc444_enable_aux_omission = false

[advanced_video]
scene_change_threshold = 0.4
intra_refresh_interval = 150
EOF

# Create proper run script
cat > /tmp/run-server.sh << 'EOF'
#!/bin/bash
# RHEL 9 Test Script
# Working directory: ~/wayland-build/wrd-server-specs

cd ~/wayland-build/wrd-server-specs

LOGFILE=~/rhel9-test-$(date +%Y%m%d-%H%M%S).log

echo "════════════════════════════════════════════════════════════"
echo "  lamco-rdp-server Test - RHEL 9 GNOME 40"
echo "════════════════════════════════════════════════════════════"
echo "Working dir: $(pwd)"
echo "Binary: $(pwd)/target/release/lamco-rdp-server"
echo "Config: $(pwd)/rhel9-config.toml"
echo "Certs: $(pwd)/certs/test-cert.pem"
echo "Log: $LOGFILE"
echo ""

# Verify files exist
if [ ! -f target/release/lamco-rdp-server ]; then
    echo "❌ Binary not found: target/release/lamco-rdp-server"
    exit 1
fi

if [ ! -f rhel9-config.toml ]; then
    echo "❌ Config not found: rhel9-config.toml"
    exit 1
fi

if [ ! -f certs/test-cert.pem ]; then
    echo "❌ Cert not found: certs/test-cert.pem"
    exit 1
fi

echo "✅ All files found"
echo "Starting server..."
echo ""

./target/release/lamco-rdp-server \
    -c rhel9-config.toml \
    -p 3389 \
    -vvv \
    2>&1 | tee "$LOGFILE"
EOF

# Deploy both
sshpass -p 'Bibi4189' scp /tmp/rhel9-config.toml /tmp/run-server.sh greg@192.168.10.6:~/wayland-build/wrd-server-specs/
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "chmod +x ~/wayland-build/wrd-server-specs/run-server.sh"
```

**Verify:**
```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "cat ~/wayland-build/wrd-server-specs/rhel9-config.toml | grep cert_path"
# Should show: cert_path = "certs/test-cert.pem"
```

---

## Step 5: Build on RHEL 9

```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "cd ~/wayland-build/wrd-server-specs && cargo build --release 2>&1 | tail -20"
```

**Expected:**
```
Compiling lamco-rdp-server v0.1.0
Finished `release` profile [optimized] target(s) in Xm XXs
```

**Verify:**
```bash
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "ls -lh ~/wayland-build/wrd-server-specs/target/release/lamco-rdp-server"
# Should show: ~23MB binary
```

---

## Step 6: Pre-Flight Checks

```bash
# Verify binary runs
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "~/wayland-build/wrd-server-specs/target/release/lamco-rdp-server --help | head -5"

# Verify config loads
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "cd ~/wayland-build/wrd-server-specs && ~/wayland-build/wrd-server-specs/target/release/lamco-rdp-server --config rhel9-config.toml --show-capabilities 2>&1 | grep 'Portal\|Strategy'"

# Verify certs accessible
sshpass -p 'Bibi4189' ssh greg@192.168.10.6 "cd ~/wayland-build/wrd-server-specs && ls -la certs/"
```

---

## Step 7: Test

**On RHEL 9 console (not SSH):**
```bash
cd ~/wayland-build/wrd-server-specs
./run-server.sh
```

**Expected behavior:**
1. Script runs pre-flight checks
2. All files found
3. Server starts
4. Permission dialog appears (approve it)
5. Server listening on 3389

---

## Known Issues (Non-Blocking)

**1. Two Permission Dialogs**
- Cause: Hybrid mode code in server/mod.rs lines 303-365
- Impact: Annoying but functional
- Fix: Requires careful editing (not urgent for testing)
- Status: Deferred until working version confirmed

**2. EGFX 7-Second Delay**
- Cause: DVC channel negotiation
- Impact: Poor quality for first 7 seconds (RemoteFX bitmaps)
- Fix: Investigate DVC initialization
- Status: Known issue, doesn't prevent testing

---

## Common Failure Modes

### "No such file or directory" (certs)

**Cause:** Config paths relative, but working directory wrong

**Fix:** Use relative paths from working directory:
```toml
cert_path = "certs/test-cert.pem"  # NOT absolute
```

**And ensure run-server.sh does:**
```bash
cd ~/wayland-build/wrd-server-specs  # Set working directory
./target/release/lamco-rdp-server -c rhel9-config.toml  # Relative paths
```

### "Failed to load config"

**Cause:** Config file not found

**Fix:** Use relative path to config when in working directory:
```bash
cd ~/wayland-build/wrd-server-specs
./target/release/lamco-rdp-server -c rhel9-config.toml  # NOT absolute
```

### Build fails with IronRDP path errors

**Cause:** Cargo.toml has absolute paths from local machine

**Fix:** Always run path fixup after deployment (Step 3 above)

---

## Production Quality Checklist

**Before calling anything "ready":**

- [ ] Binary builds without errors
- [ ] Binary runs without crashes
- [ ] Config loads correctly
- [ ] Certs load correctly
- [ ] Server listens on port 3389
- [ ] Client can connect
- [ ] Video displays (any quality)
- [ ] Mouse works
- [ ] Keyboard works
- [ ] No repeated failures/crashes
- [ ] Log file created and accessible
- [ ] Error messages helpful (not generic)

**NOT required for initial test:**
- [ ] One dialog (known issue, deferred)
- [ ] Perfect video quality (tuning in progress)
- [ ] Zero errors in log (warnings OK)

---

## Current Status

**Working:**
- ✅ Mutter disabled (Portal-only strategy)
- ✅ Clipboard won't crash (Optional handling)
- ✅ Source deployed to RHEL 9
- ✅ Binary built (23MB)
- ✅ Config exists
- ✅ Certs exist

**Not Working:**
- ❌ Certs not found at runtime (path resolution issue)
- ❌ Two dialogs (hybrid mode bug)

**Next Actions:**
1. Fix cert path resolution (use relative paths + ensure working directory)
2. Test if it works (accept two dialogs for now)
3. Then fix two dialogs issue carefully

---

*This checklist should be followed for every RHEL 9 deployment to ensure consistency.*
