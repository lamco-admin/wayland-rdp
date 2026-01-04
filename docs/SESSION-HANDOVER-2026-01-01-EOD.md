# Session Handover - End of Day 2026-01-01

**Date:** 2026-01-01
**Focus:** RHEL 9 testing + OBS build server setup
**Status:** Environment discovered, build server in progress

---

## What We Accomplished

### RHEL 9 Environment Discovery ✅
**VM:** 192.168.10.6 (user: greg, pass: Bibi4189)

**Critical Findings:**
- ✅ GNOME Shell 40.10
- ✅ Mutter D-Bus services PRESENT and available
  - `org.gnome.Mutter.ScreenCast` version 4
  - `org.gnome.Mutter.RemoteDesktop` version 1
- ✅ Portal version 4 (RESTORE TOKENS SUPPORTED!)
  - xdg-desktop-portal 1.12.6
  - xdg-desktop-portal-gnome 41.2
- ✅ gnome-remote-desktop 40.0-11.el9_6 installed

**Documentation:** `docs/RHEL9-GNOME40-FINDINGS.md`

**Key Discovery:** RHEL 9 is BETTER than expected
- Initial assumption: "Portal v3, no tokens" ❌ WRONG
- Reality: Portal v4 WITH restore tokens ✅
- Mutter services available ✅
- Should support zero-dialog operation via BOTH strategies

### openSUSE Tumbleweed Build Server Setup ✅
**VM:** 192.168.10.7 (VM 102, user: greg, pass: Bibi4189)

**Status: HEALTHY**
- OS: openSUSE Tumbleweed 20251230 (latest)
- Disk: 486GB free (97% free)
- RAM: 14GB available
- No failed services, fully updated

**Build Environment Installed:**
- ✅ Rust 1.92.0
- ✅ Cargo 1.92.0
- ✅ GCC 15.2.1
- ✅ Ruby 3.4 + Rails 8.0 + Bundler
- ✅ MariaDB 11.8.5 + PostgreSQL libraries (libpq5)
- ✅ Apache 2.4.66
- ✅ Node.js 24, npm, yarn
- ✅ Perl JSON::XS, YAML::LibYAML
- ✅ createrepo_c, memcached
- ✅ pipewire-devel, dbus-devel, glib2-devel, openssl-devel

**OBS Source:**
- ✅ Cloned from https://github.com/openSUSE/open-build-service.git
- Location: `~/open-build-service`

---

## What Remains

### Critical: Test on RHEL 9 (Blocks Enterprise Launch)

**The Question:** Does Mutter API actually work on GNOME 40?

**Why Critical:**
- D-Bus services exist ✅
- But does the API function correctly?
- If YES: Zero-dialog operation on RHEL 9/Ubuntu 22.04 LTS ✅
- If NO: Portal v4 fallback (1 dialog first run, 0 after) ⚠️

**Blocker:** Need lamco-rdp-server binary that runs on RHEL 9

**Build Issue:**
- glibc version mismatch (Ubuntu 24.04 binary needs 2.38+, RHEL 9 has 2.34)
- Can't static link (pipewire, dbus are system libs)
- Can't cross-compile easily (system dependency complications)

**Solution Options:**
1. Build on RHEL 9 directly (copy all source + workspaces)
2. Build on openSUSE with glibc 2.34 compatibility
3. Set up proper multi-distro build pipeline

### OBS Build Server Setup

**Current State:**
- Dependencies installed ✅
- OBS source cloned ✅
- Need to actually build/install OBS server components

**OBS Components Needed:**
- Backend (Perl) - build scheduler and workers
- Frontend (Rails) - web UI and API
- Database (PostgreSQL preferred by user, MariaDB also installed)

**Next Steps:**
1. Configure database (PostgreSQL)
2. Build/install OBS backend
3. Build/install OBS frontend
4. Run setup wizard
5. Configure build workers
6. Create lamco-rdp-server build project

**Documentation:** OBS setup guide needed

---

## Key Decisions Made

### Database Choice
**User preference:** PostgreSQL over MariaDB

OBS supports both. PostgreSQL already available (libpq5 installed).

### Build Strategy
**User requirement:** NO containers/Docker

Must use native builds or OBS (which builds in clean VMs natively).

### BUSL License Implications
**Finding:** BUSL-1.1 is NOT open source
- Can't use public build.opensuse.org free tier
- Must run own OBS instance (which we're doing)
- GitHub Actions alternative also viable

---

## Technical Challenges Identified

### Cargo.toml Dependency Structure

**Current:**
```toml
# Local path dependencies:
lamco-portal = { path = "../lamco-wayland/crates/lamco-portal" }  # v0.3.0 unpublished
IronRDP = { path = "/home/greg/wayland/IronRDP/..." }  # forked with patches
lamco-pipewire = { path = "../lamco-rdp-workspace/..." }  # may have unpublished fixes
```

**Problem:** Can't build without these repos present

**Solutions:**
1. **Vendor dependencies:** `cargo vendor` creates standalone build
2. **Publish all forks:** Push IronRDP fork, lamco-portal v0.3.0, etc.
3. **Git submodules:** Include dependencies in repo
4. **OBS multi-package:** Build each dependency separately

### glibc Compatibility

**Issue:**
- Ubuntu 24.04: glibc 2.39
- RHEL 9: glibc 2.34
- openSUSE Tumbleweed: glibc 2.41

**Solution:** Build on target platform or oldest supported platform

---

## VMs Available

| VM | IP | OS | Status | Purpose |
|----|----|----|--------|---------|
| VM1 | 192.168.10.205 | Ubuntu 24.04 / GNOME 46 | Tested ✅ | Development |
| VM ? | 192.168.10.6 | RHEL 9 / GNOME 40 | Ready ⏳ | Critical test target |
| VM 102 | 192.168.10.7 | openSUSE Tumbleweed | Ready ✅ | Build server |

---

## Next Session Priorities

### PRIORITY 1: Get OBS Running (4-8 hours)

**VM:** 192.168.10.7 (openSUSE Tumbleweed)
**Goal:** Full OBS server operational

**Steps:**
1. Configure PostgreSQL database for OBS
2. Build OBS backend from source (Perl components)
3. Build OBS frontend from source (Rails app)
4. Run OBS setup wizard (`dist/setup-appliance.sh`)
5. Start OBS services (scheduler, dispatcher, workers)
6. Configure at least one build worker
7. Create test project to verify OBS works
8. Create lamco-rdp-server build project with multi-distro targets

**Expected Deliverable:**
- Working OBS instance accessible via web UI
- Capable of building packages for RHEL 9, Ubuntu 22.04, Ubuntu 24.04

### PRIORITY 2: RHEL 9 Testing (Deferred)

**Only after OBS is operational.**

Test lamco-rdp-server on RHEL 9 to answer: Does Mutter work on GNOME 40?

This validates the enterprise value proposition but is NOT the immediate priority.

---

## Open Questions

1. **How to build OBS from source?** Need to follow proper build procedure for backend and frontend
2. **PostgreSQL configuration for OBS?** User prefers PostgreSQL over MariaDB
3. **How to configure OBS for multi-distro builds?** Need build targets for RHEL 9, Ubuntu 22.04, Ubuntu 24.04
4. **How to handle lamco-rdp-server local dependencies in OBS?** Vendor? Submodules? Multi-package?
5. **Does Mutter API work on GNOME 40?** (Deferred - test after OBS is running)

---

## Files Created This Session

- `docs/RHEL9-GNOME40-FINDINGS.md` - RHEL 9 environment analysis
- `docs/SESSION-HANDOVER-2026-01-01-EOD.md` - This file

---

## Commands for Tomorrow

### Priority 1: OBS Setup on openSUSE
```bash
# SSH to openSUSE:
ssh greg@192.168.10.7

# Install PostgreSQL:
sudo zypper install postgresql-server postgresql-contrib

# Initialize and start PostgreSQL:
sudo systemctl enable postgresql
sudo systemctl start postgresql

# Create OBS database and user:
sudo -u postgres createuser -P obsadmin
sudo -u postgres createdb -O obsadmin obs_api_production

# Build OBS backend:
cd ~/open-build-service/src/backend
# Follow build/install steps (TBD - research needed)

# Build OBS frontend:
cd ~/open-build-service/src/api
bundle install
# Configure database.yml for PostgreSQL
# Run migrations
# Start services

# Run OBS setup wizard:
# (command TBD - need to research proper procedure)
```

### Priority 2: RHEL 9 Test (Deferred)
```bash
# Only after OBS is working - use OBS to build the binary
# Then deploy to RHEL 9 for testing
```

---

## User Priorities

**Priority 1: OBS Setup**
- Get Open Build Service running on openSUSE Tumbleweed VM (192.168.10.7)
- Use PostgreSQL as database
- Configure for multi-distro builds (RHEL 9, Ubuntu 22.04, 24.04)
- NO containerized solutions

**Priority 2: Everything Else (Deferred)**
- RHEL 9 testing waits until OBS is operational
- Will use OBS to build binaries for testing

---

## Session Summary

**Time:** ~3 hours
**Progress:**
- ✅ Discovered RHEL 9 has better support than expected (both Mutter and Portal v4)
- ✅ Set up openSUSE Tumbleweed build VM (192.168.10.7)
- ✅ Installed all OBS build dependencies (Ruby, Rails, PostgreSQL, Perl, Apache)
- ✅ Cloned OBS source code
- ⏳ OBS not yet built/configured

**Next Step:** Build and configure Open Build Service from source

**User Priority:** Get OBS operational first. Everything else (RHEL 9 testing) is deferred until OBS is running.

---

*Ready for tomorrow's session. Priority: Complete OBS installation and configuration.*
