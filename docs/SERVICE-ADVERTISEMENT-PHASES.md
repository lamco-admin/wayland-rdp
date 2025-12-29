# Service Advertisement Implementation Phases

## Phase Overview

| Phase | Focus | Deliverables | Dependencies |
|-------|-------|--------------|--------------|
| 1 | Core Registry | ServiceRegistry, service types, translation | Compositor probing |
| 2 | Capability Injection | Dynamic RDP capabilities | Phase 1, IronRDP |
| 3 | Runtime Negotiation | Service-aware frame processing | Phase 2, Premium features |
| 4 | Client Protocol | Custom RDP channel for discovery | Phase 3 |

---

## Phase 1: Service Registry Core

**Goal**: Create the foundational service registry that translates compositor capabilities into advertised services.

### Deliverables

1. **Module Structure**
   ```
   src/services/
   ├── mod.rs
   ├── registry.rs
   ├── service.rs
   ├── wayland_features.rs
   ├── rdp_capabilities.rs
   └── translation.rs
   ```

2. **Core Types**
   - `ServiceRegistry` struct
   - `AdvertisedService` struct
   - `ServiceId` enum (all known services)
   - `ServiceLevel` enum (Guaranteed/BestEffort/Degraded/Unavailable)
   - `WaylandFeature` enum
   - `RdpCapability` enum
   - `PerformanceHints` struct

3. **Translation Logic**
   - `ServiceRegistry::from_compositor(caps: CompositorCapabilities)`
   - Per-compositor profile generation
   - Quirk-aware service level assignment

4. **Query API**
   - `registry.has_service(id: ServiceId) -> bool`
   - `registry.service_level(id: ServiceId) -> ServiceLevel`
   - `registry.get_service(id: ServiceId) -> Option<&AdvertisedService>`
   - `registry.all_services() -> &[AdvertisedService]`

### Implementation Tasks

- [x] Create `src/services/mod.rs` with module structure
- [x] Define `ServiceId` enum with all known services
- [x] Define `ServiceLevel` enum with comparison traits
- [x] Define `WaylandFeature` enum covering detected features
- [x] Define `RdpCapability` enum for RDP-side mappings
- [x] Implement `AdvertisedService` struct
- [x] Implement `ServiceRegistry` struct
- [x] Implement `from_compositor()` translation for GNOME
- [x] Implement `from_compositor()` translation for KDE
- [x] Implement `from_compositor()` translation for Sway/wlroots
- [x] Implement `from_compositor()` translation for unknown compositors
- [x] Add logging for service advertisement
- [x] Add unit tests for each compositor profile
- [x] Wire into lib.rs exports

### Success Criteria

```rust
// This should work after Phase 1:
let caps = probe_capabilities().await?;
let registry = ServiceRegistry::from_compositor(caps);

for service in registry.all_services() {
    info!("Service: {} = {:?}", service.name, service.level);
}

assert!(registry.has_service(ServiceId::DamageTracking));
```

---

## Phase 2: Dynamic Capability Injection

**Goal**: Inject advertised services into RDP capability exchange so clients receive compositor-aware capabilities.

### Deliverables

1. **Capability Generation**
   - `registry.generate_rdp_capabilities() -> Vec<CapabilitySet>`
   - Dynamic EGFX codec selection based on DMA-BUF support
   - Cursor capability adjustment based on metadata support

2. **IronRDP Integration**
   - Modify server initialization to accept ServiceRegistry
   - Pass registry to capability generation
   - Store registry in session state

3. **EGFX Adaptation**
   - Enable/disable AVC444 based on compositor support
   - Adjust quality presets based on buffer type support

### Implementation Tasks

- [x] Add `ServiceRegistry` field to server session state
- [x] Create `generate_rdp_capabilities()` method (as `recommended_codecs()`)
- [x] Implement EGFX codec selection logic (`should_enable_avc444()`)
- [x] Implement cursor capability injection (`should_use_predictive_cursor()`)
- [x] Add service-aware logging at startup
- [ ] Modify IronRDP `capabilities()` function to accept registry (future)
- [ ] Add desktop composition capability based on multi-mon support (future)
- [ ] Add integration tests (future)

### Success Criteria

```rust
// Clients should receive different capabilities based on compositor
// GNOME: AVC420 preferred, MemFd buffers
// Sway: AVC444 enabled, DMA-BUF zero-copy
```

---

## Phase 3: Runtime Negotiation

**Goal**: Use service registry at runtime to make intelligent decisions about encoding, frame rate, and feature usage.

### Deliverables

1. **Adaptive FPS Integration**
   - Check `DamageTracking` service level before enabling activity detection
   - Adjust min/max FPS based on compositor profile

2. **Latency Governor Integration**
   - Use `ExplicitSync` service to choose frame pacing strategy
   - Adjust encoding decisions based on buffer type

3. **Cursor Strategy Integration**
   - Check `MetadataCursor` service level
   - Fall back to painted cursor if degraded

4. **Encoder Integration**
   - Zero-copy path when `DmaBufZeroCopy` is Guaranteed
   - Quality adjustments based on performance hints

### Implementation Tasks

- [ ] Pass registry to display handler
- [ ] Integrate with AdaptiveFpsController
- [ ] Integrate with LatencyGovernor
- [ ] Integrate with CursorStrategy
- [ ] Add service-based encoder configuration
- [ ] Add runtime service level logging
- [ ] Add performance metrics per service

### Success Criteria

```rust
// Logs should show service-aware decisions:
// "DamageTracking service: Guaranteed, enabling activity detection"
// "MetadataCursor service: Degraded, using painted cursor fallback"
```

---

## Phase 4: Client Protocol (Future)

**Goal**: Create a custom RDP channel for Wayland-specific service discovery, enabling smart clients to query available features.

### Deliverables

1. **Custom DVC Channel**
   - `wayland-services` dynamic virtual channel
   - Protocol for service enumeration
   - Protocol for feature queries

2. **Service Discovery Messages**
   - `ListServices` request/response
   - `QueryService` request/response
   - `ServiceChanged` notification

3. **Client SDK**
   - Documentation for client implementers
   - Reference implementation for FreeRDP

### Protocol Definition

```
Channel: "wayland-services"
Version: 1

Messages:
  ListServicesRequest { }
  ListServicesResponse { services: Vec<ServiceInfo> }

  QueryServiceRequest { service_id: u32 }
  QueryServiceResponse { service: ServiceDetails }

  ServiceChangedNotification { service_id: u32, new_level: u8 }
```

### Implementation Tasks

- [ ] Define wire protocol
- [ ] Implement DVC channel handler
- [ ] Implement service enumeration
- [ ] Implement query handling
- [ ] Add notification for dynamic changes
- [ ] Document protocol for clients
- [ ] Test with custom FreeRDP build

---

## Timeline Estimates

| Phase | Estimated Effort | Prerequisites |
|-------|-----------------|---------------|
| Phase 1 | 2-3 days | Compositor probing complete |
| Phase 2 | 2-3 days | Phase 1 complete |
| Phase 3 | 3-4 days | Phase 2 + Premium features |
| Phase 4 | 5-7 days | Phase 3 + Protocol design |

## Risk Mitigation

1. **Phase 1 Risk**: Translation logic complexity
   - Mitigation: Start with GNOME profile, iterate

2. **Phase 2 Risk**: IronRDP integration challenges
   - Mitigation: Minimal changes to IronRDP, adapt on our side

3. **Phase 3 Risk**: Performance overhead
   - Mitigation: Registry queries are O(1), cached at startup

4. **Phase 4 Risk**: Client adoption
   - Mitigation: Protocol is optional, graceful degradation
