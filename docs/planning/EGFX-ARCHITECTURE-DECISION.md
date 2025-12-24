# EGFX Architecture Decision Record

**Date**: 2024-12-24
**Status**: Accepted
**Decision**: Option E (Hybrid) - Arc<Mutex<>> Bridge + ServerEvent Pattern

---

## Context

We need to send H.264 video frames via EGFX (MS-RDPEGFX) from our display handler to RDP clients. The challenge is that `GraphicsPipelineServer` is owned by `DrdynvcServer` after channel attachment, making it inaccessible for proactive frame sending.

## Decision

We will implement **Option E (Hybrid)** which combines:

1. **GfxDvcBridge wrapper** - Holds `Arc<Mutex<GraphicsPipelineServer>>` and implements `DvcProcessor`
2. **Factory provides shared access** - `WrdGfxFactory` returns `Arc<Mutex<GraphicsPipelineServer>>`
3. **ServerEvent::Egfx** - Routes encoded DVC messages through the server event loop to the wire

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           HYBRID ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  WrdGfxFactory                                                              │
│       │                                                                     │
│       ├─► Creates Arc<Mutex<GraphicsPipelineServer>>                       │
│       │                                                                     │
│       ├─► Returns Arc clone to display handler                             │
│       │                                                                     │
│       └─► Wraps in GfxDvcBridge for DrdynvcServer                          │
│                                                                             │
│  ┌─────────────────────────┐     ┌─────────────────────────┐               │
│  │   WrdDisplayHandler     │     │     DrdynvcServer       │               │
│  │                         │     │                         │               │
│  │  gfx_server: Arc<Mutex< │     │  GfxDvcBridge {         │               │
│  │    GraphicsPipelineServer     │    inner: Arc<Mutex<    │               │
│  │  >>                     │     │      GraphicsPipeline   │               │
│  │                         │     │      Server>>           │               │
│  │  [calls send_avc420_    │     │  }                      │               │
│  │   frame() directly]     │     │                         │               │
│  └───────────┬─────────────┘     │  [handles client msgs]  │               │
│              │                   └─────────────────────────┘               │
│              │                                                             │
│              │ drain_output() → Vec<DvcMessage>                            │
│              │                                                             │
│              ▼                                                             │
│  ┌─────────────────────────┐                                               │
│  │ ServerEvent::Egfx(msgs) │ ──► Server event loop ──► Wire ──► Client    │
│  └─────────────────────────┘                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Rationale

1. **Follows IronRDP patterns** - Uses `ServerEvent` for proactive sending (like Clipboard/Rdpsnd)
2. **Preserves GraphicsPipelineServer benefits** - Frame tracking, QoE metrics, surface management
3. **Minimal upstream changes** - Bridge is implementation detail, ServerEvent is familiar
4. **Future-proof** - Bridge isolates us from internal changes

## Consequences

### Positive
- Full access to GraphicsPipelineServer methods
- Efficient frame sending without data copying through channels
- Works with existing IronRDP event dispatch pattern
- Testable in isolation

### Negative
- Introduces Arc<Mutex<>> pattern (not used elsewhere in IronRDP)
- Mutex contention possible (mitigated by async Mutex)
- Slightly more complex than pure ServerEvent approach

### Neutral
- Requires upstream PR for ServerEvent::Egfx (can be done after initial EGFX PR merged)

## Implementation Plan

### Phase 1: Local Implementation (wrd-server-specs + local IronRDP fork)

1. Create `GfxDvcBridge` in ironrdp-server (local fork)
2. Update `WrdGfxFactory` to create and share `Arc<Mutex<GraphicsPipelineServer>>`
3. Add `ServerEvent::Egfx` variant to ironrdp-server
4. Implement frame sending in `WrdDisplayHandler`
5. Test with real H.264 frames

### Phase 2: Upstream PRs (after EGFX merged)

See `EGFX-UPSTREAM-STRATEGY.md` for detailed PR plan.

## Alternatives Considered

| Option | Why Not Chosen |
|--------|---------------|
| A (Pure ServerEvent) | Frame data copying through event channel adds latency |
| B (Arc only) | No clear output routing pattern |
| C (PDU Injection) | Loses GraphicsPipelineServer benefits, duplicates logic |
| D (Callback) | Complicates handler trait significantly |

## References

- [EGFX-COMPREHENSIVE-ANALYSIS.md](./EGFX-COMPREHENSIVE-ANALYSIS.md) - Full analysis
- [EGFX-ARCHITECTURE-DEEP-DIVE.md](./EGFX-ARCHITECTURE-DEEP-DIVE.md) - Technical details
- IronRDP FIXME(#61) - Acknowledged gap in DVC proactive messaging
