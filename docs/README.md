# wayland-rdp Documentation

This directory contains organized documentation for the wayland-rdp project - a Wayland-native RDP server for Linux.

## Directory Structure

### Active Documentation

| Directory | Description | Files |
|-----------|-------------|-------|
| [specs/](specs/) | Core specifications and design documents | 8 |
| [strategy/](strategy/) | Strategic planning, roadmaps, and crate breakdown | 8 |
| [architecture/](architecture/) | Architecture research and implementation plans | 6 |
| [guides/](guides/) | Setup guides, testing, and deployment instructions | 9 |
| [ironrdp/](ironrdp/) | IronRDP integration documentation | 13 |

### Archived Documentation

| Directory | Description | Files |
|-----------|-------------|-------|
| [archive/sessions/](archive/sessions/) | Session handover and summary documents | 19 |
| [archive/fixes/](archive/fixes/) | Bug fix documentation and clipboard work | 33 |
| [archive/status-reports/](archive/status-reports/) | Completed work and status reports | 20 |
| [archive/ccw/](archive/ccw/) | Claude Code Worker session documents | 6 |
| [archive/superseded/](archive/superseded/) | Older planning and analysis documents | 23 |

## Quick Navigation

### Getting Started
- [QUICKSTART.md](guides/QUICKSTART.md) - Quick setup guide
- [MANUAL-SETUP-INSTRUCTIONS.md](guides/MANUAL-SETUP-INSTRUCTIONS.md) - Detailed setup

### Core Specifications
- [00-MASTER-SPECIFICATION.md](specs/00-MASTER-SPECIFICATION.md) - Master specification
- [01-ARCHITECTURE.md](specs/01-ARCHITECTURE.md) - Architecture overview
- [02-TECHNOLOGY-STACK.md](specs/02-TECHNOLOGY-STACK.md) - Technology stack
- [PHASE-1-SPECIFICATION.md](specs/PHASE-1-SPECIFICATION.md) - Phase 1 implementation
- [PHASE-2-SPECIFICATION.md](specs/PHASE-2-SPECIFICATION.md) - Phase 2 implementation

### Strategic Documents
- [STRATEGIC-FRAMEWORK.md](strategy/STRATEGIC-FRAMEWORK.md) - Project strategic framework
- [CRATE-BREAKDOWN-AND-OPEN-SOURCE-STRATEGY.md](strategy/CRATE-BREAKDOWN-AND-OPEN-SOURCE-STRATEGY.md) - Crate separation strategy
- [PRODUCTION-ROADMAP.md](strategy/PRODUCTION-ROADMAP.md) - Production roadmap

### Architecture
- [MULTI-BACKEND-ARCHITECTURE-RESEARCH.md](architecture/MULTI-BACKEND-ARCHITECTURE-RESEARCH.md) - Multi-backend research
- [CLIPBOARD-ARCHITECTURE-FINAL.md](architecture/CLIPBOARD-ARCHITECTURE-FINAL.md) - Clipboard architecture

### IronRDP Integration
- [IRONRDP-INTEGRATION-GUIDE.md](ironrdp/IRONRDP-INTEGRATION-GUIDE.md) - Integration guide
- [IRONRDP-QUICK-REFERENCE.md](ironrdp/IRONRDP-QUICK-REFERENCE.md) - Quick reference

### Premium Features & Build Guides
- [PREMIUM-FEATURES-DEVELOPMENT-PLAN.md](PREMIUM-FEATURES-DEVELOPMENT-PLAN.md) - Premium features overview and status
- [HARDWARE-ENCODING-BUILD-GUIDE.md](HARDWARE-ENCODING-BUILD-GUIDE.md) - **Comprehensive** GPU encoding build/distribution guide
- [HARDWARE-ENCODING-QUICKREF.md](HARDWARE-ENCODING-QUICKREF.md) - Quick reference for hardware encoding
- [AVC444-IMPLEMENTATION-STATUS.md](AVC444-IMPLEMENTATION-STATUS.md) - AVC444 codec implementation details
- [DAMAGE-TRACKING-STATUS.md](DAMAGE-TRACKING-STATUS.md) - Damage tracking implementation details

## Project Overview

wayland-rdp is a Wayland-native RDP server that enables remote desktop access to Linux systems running Wayland compositors. Key features:

- **Portal-based screen capture** using XDG Desktop Portals
- **PipeWire integration** for efficient video capture
- **IronRDP** for RDP protocol implementation
- **Bidirectional clipboard** with GNOME extension support
- **Multi-backend architecture** for compositor flexibility

## Related Repositories

- **lamco-rdp** - Published crates on crates.io:
  - `lamco-rdp` - Meta-crate with feature flags
  - `lamco-rdp-input` - uinput-based keyboard/mouse input
  - `lamco-clipboard-core` - Clipboard abstraction layer
  - `lamco-rdp-clipboard` - RDP clipboard channel integration

## Documentation Status

This documentation was reorganized on 2025-12-16. Superseded and historical documents have been preserved in the `archive/` directory. Active documentation reflects the current project direction following the separation of reusable crates into the lamco-rdp workspace.
