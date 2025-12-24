# EGFX Upstream PR Strategy

**Date**: 2024-12-24
**Status**: Planning
**Timeline**: After initial EGFX PR (#1057) is merged

---

## Overview

Our Hybrid architecture requires changes to `ironrdp-server` that should eventually be upstreamed. This document outlines the PR strategy and timing.

## Current State

- **PR #1057**: EGFX server support (ironrdp-egfx) - pending review
- **Local fork**: Contains EGFX + file transfer changes
- **wrd-server-specs**: Uses local fork for development

## Upstream PR Plan

### Timing

```
Timeline:
─────────────────────────────────────────────────────────────────────────────

NOW                         EGFX PR Merged            Our PRs
 │                               │                       │
 │  Development with             │  Clean up and         │  Submit upstream
 │  local fork                   │  prepare PRs          │  PRs
 │                               │                       │
 ▼                               ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Local Development    │ Phase 2: Prep         │ Phase 3: Submit │
│ - Implement Hybrid approach   │ - Extract changes     │ - PR #A         │
│ - Validate with real frames   │ - Write tests         │ - PR #B         │
│ - Iterate on design           │ - Document            │ - PR #C         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Why Wait?

1. **Avoid PR conflicts** - EGFX PR (#1057) changes overlapping files
2. **Validate design** - Ensure our approach works before proposing upstream
3. **Clean extraction** - Separate our changes from file transfer work
4. **Better reception** - Show working implementation as evidence

---

## Proposed PRs

### PR #A: DVC Processor Access Method (Small, Core-adjacent)

**Target**: `ironrdp-dvc`
**Size**: ~20 lines
**Risk**: Low

Add method to access DVC processor by type:

```rust
impl DrdynvcServer {
    /// Get a dynamic channel processor by type
    pub fn get_dvc_processor<T: DvcProcessor + 'static>(&self) -> Option<&T> {
        for (_, channel) in self.dynamic_channels.iter() {
            if let Some(processor) = channel.processor.as_any().downcast_ref::<T>() {
                return Some(processor);
            }
        }
        None
    }

    /// Get mutable access to a dynamic channel processor by type
    pub fn get_dvc_processor_mut<T: DvcProcessor + 'static>(&mut self) -> Option<&mut T> {
        for (_, channel) in self.dynamic_channels.iter_mut() {
            if let Some(processor) = channel.processor.as_any_mut().downcast_mut::<T>() {
                return Some(processor);
            }
        }
        None
    }
}
```

**Justification**: Mirrors existing `get_svc_processor<T>()` pattern in ironrdp-server.

---

### PR #B: ServerEvent::Egfx (Medium, Community Tier)

**Target**: `ironrdp-server`
**Size**: ~100 lines
**Risk**: Medium

Add EGFX event variant and dispatch logic:

```rust
// In server.rs
pub enum ServerEvent {
    Quit(String),
    Clipboard(ClipboardMessage),
    Rdpsnd(RdpsndServerMessage),
    SetCredentials(Credentials),
    GetLocalAddr(oneshot::Sender<Option<SocketAddr>>),
    #[cfg(feature = "egfx")]
    Egfx(EgfxServerMessage),  // NEW
}

#[cfg(feature = "egfx")]
pub enum EgfxServerMessage {
    /// Pre-encoded DVC messages ready to send
    SendMessages {
        channel_id: u32,
        messages: Vec<SvcMessage>,
    },
}

// In dispatch_server_events:
#[cfg(feature = "egfx")]
ServerEvent::Egfx(msg) => {
    match msg {
        EgfxServerMessage::SendMessages { channel_id, messages } => {
            let data = server_encode_svc_messages(
                messages,
                drdynvc_channel_id,
                user_channel_id
            )?;
            writer.write_all(&data).await?;
        }
    }
}
```

**Justification**: Follows Clipboard/Rdpsnd pattern exactly. Enables proactive EGFX frame sending.

---

### PR #C: GfxDvcBridge Helper (Optional, Community Tier)

**Target**: `ironrdp-server`
**Size**: ~50 lines
**Risk**: Low

Add bridge wrapper for shared GraphicsPipelineServer access:

```rust
#[cfg(feature = "egfx")]
pub struct GfxDvcBridge {
    inner: Arc<Mutex<GraphicsPipelineServer>>,
}

#[cfg(feature = "egfx")]
impl GfxDvcBridge {
    pub fn new(server: Arc<Mutex<GraphicsPipelineServer>>) -> Self {
        Self { inner: server }
    }

    pub fn server(&self) -> &Arc<Mutex<GraphicsPipelineServer>> {
        &self.inner
    }
}

#[cfg(feature = "egfx")]
impl DvcProcessor for GfxDvcBridge {
    fn channel_name(&self) -> &str {
        ironrdp_egfx::CHANNEL_NAME
    }

    fn start(&mut self, channel_id: u32) -> PduResult<Vec<DvcMessage>> {
        self.inner.blocking_lock().start(channel_id)
    }

    fn process(&mut self, channel_id: u32, payload: &[u8]) -> PduResult<Vec<DvcMessage>> {
        self.inner.blocking_lock().process(channel_id, payload)
    }

    fn close(&mut self, channel_id: u32) {
        self.inner.blocking_lock().close(channel_id)
    }
}
```

**Note**: This PR is optional. The bridge could remain in wrd-server-specs if IronRDP prefers not to add shared ownership patterns.

---

## PR Submission Strategy

### Approach: Incremental, Well-Documented

1. **Start with PR #A** (smallest, least controversial)
   - Reference FIXME(#61) in description
   - Show it enables proactive DVC messaging
   - Wait for approval before proceeding

2. **Submit PR #B** (main functionality)
   - Reference PR #A
   - Show real-world use case (our EGFX implementation)
   - Include tests

3. **Offer PR #C** (if desired)
   - Only if IronRDP wants the bridge upstream
   - Otherwise, keep in wrd-server-specs

### PR Description Template

```markdown
## Summary

[Brief description of what this PR adds]

## Motivation

This addresses the proactive DVC messaging gap noted in FIXME(#61). Currently,
DVC processors can only send data in response to client messages. For video
streaming via EGFX, we need to push frames proactively.

## Changes

- [List of changes]

## Testing

- [How this was tested]

## Related

- Closes #61 (partially)
- Related to PR #1057 (EGFX support)
```

---

## Fallback Plans

### If PR #A is Rejected

Use `AsAny` pattern to access processor directly:

```rust
// Less clean, but works
let dvc = static_channels.get_by_type_mut::<DrdynvcServer>()?;
// Can't easily get to inner GraphicsPipelineServer without upstream changes
```

### If PR #B is Rejected

Keep `ServerEvent::Egfx` in our local fork permanently, or use direct DVC encoding:

```rust
// Encode and send without ServerEvent
let messages = encode_dvc_messages(channel_id, dvc_msgs, flags)?;
let data = server_encode_svc_messages(messages, drdynvc_id, user_id)?;
// Send directly in our own event loop
```

### If PR #C is Rejected

Keep `GfxDvcBridge` in wrd-server-specs. This is the expected outcome - the bridge is implementation-specific.

---

## Success Criteria

- [ ] PR #A merged: Can access DVC processor by type
- [ ] PR #B merged: Can route EGFX messages through ServerEvent
- [ ] Real H.264 streaming works end-to-end
- [ ] No performance regression vs. RemoteFX path

---

## Notes

- @mihneabuz is the Community Tier maintainer for ironrdp-server
- FIXME(#61) provides good justification for these changes
- The client already has `RdpInputEvent::SendDvcMessages` (precedent)
