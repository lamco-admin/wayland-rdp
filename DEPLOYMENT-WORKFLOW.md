# Deployment Workflow for Test Server

**Test Server**: `greg@192.168.10.205`

## Standard Deployment Process

### 1. Build the Binary

```bash
cargo build --release
```

### 2. Deploy to Test Server

**IMPORTANT**: Follow these steps in order:

```bash
# Step 1: Delete old binary
ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server"

# Step 2: Copy new binary with correct name
scp target/release/lamco-rdp-server greg@192.168.10.205:~/lamco-rdp-server

# Step 3: Make executable and verify
ssh greg@192.168.10.205 "chmod +x ~/lamco-rdp-server && md5sum ~/lamco-rdp-server"
```

### 3. Verify Deployment

Compare MD5 hashes:
```bash
# Local
md5sum target/release/lamco-rdp-server

# Remote (should match)
ssh greg@192.168.10.205 "md5sum ~/lamco-rdp-server"
```

### 4. Run the Server

The test server uses `~/run-server.sh` which:
- Automatically creates a timestamped log file: `colorful-test-YYYYMMDD-HHMMSS.log`
- Sets logging level to TRACE (`-vvv`)
- Includes pre-flight checks (D-Bus, PipeWire, config.toml, certs)
- Displays the binary MD5 on startup

**SSH to test server and run:**
```bash
ssh greg@192.168.10.205
cd ~  # or wherever config.toml and certs/ are located
./run-server.sh
```

### 5. Test and Collect Logs

The log file is automatically timestamped and saved in the current directory.

**IMPORTANT**: After testing, ALWAYS copy the log file locally for analysis.

**DO NOT** SSH and grep remotely. **ALWAYS** copy the file and use `rg` (ripgrep) locally.

```bash
# Step 1: Find the latest log file
ssh greg@192.168.10.205 "ls -lt ~/colorful-test-*.log | head -1"

# Step 2: Copy the log file locally (replace TIMESTAMP with actual timestamp)
scp greg@192.168.10.205:~/colorful-test-TIMESTAMP.log ./

# Step 3: Analyze locally with ripgrep
rg "TEMPORAL" colorful-test-TIMESTAMP.log
rg "AUXILIARY VIEW" colorful-test-TIMESTAMP.log
rg "COLOR CONVERSION" colorful-test-TIMESTAMP.log
```

### 6. Analyze Results Locally

**Use ripgrep commands like:**
```bash
# Check temporal stability
rg "TEMPORAL" colorful-test-*.log | tail -20

# Check auxiliary buffer analysis
rg "AUXILIARY VIEW MULTI-POSITION" colorful-test-*.log -A 20

# Check frame hashes
rg "hash:" colorful-test-*.log
```

## Current Deployment (2025-12-29)

**Binary MD5**: `6bc1df27435452e7a622286de716862b`
**Changes**: ✅ STABLE - Reverted to committed stable (dual encoder, all-I workaround)
**Purpose**: Restore working state after temporal layers protocol errors
**Previous**: Temporal layers created empty Aux bitstreams → connection errors
**Config**: Dual encoder (original architecture), both forced to all-I frames
**Status**: STABLE AND WORKING - Perfect quality confirmed

## Important Notes

- **Binary Name**: Must be `lamco-rdp-server` (not `lamco-rdp-server-*` or any variant)
- **Location**: Home directory (`~/lamco-rdp-server`)
- **Run Script**: `~/run-server.sh` handles all logging configuration
- **Log Files**: Automatically timestamped for reference
- **Always delete old binary first** to avoid confusion about which version is running

## Quick Reference

```bash
# Complete deployment in one go:
cargo build --release && \
ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server" && \
scp target/release/lamco-rdp-server greg@192.168.10.205:~/lamco-rdp-server && \
ssh greg@192.168.10.205 "chmod +x ~/lamco-rdp-server && md5sum ~/lamco-rdp-server"
```
