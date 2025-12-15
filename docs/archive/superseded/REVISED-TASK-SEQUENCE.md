# REVISED TASK SEQUENCE - PRACTICAL IMPLEMENTATION ORDER
**Document:** REVISED-TASK-SEQUENCE.md
**Version:** 2.0
**Date:** 2025-01-18
**Reason:** Feedback from CCW - tasks need better scoping and dependency management

---

## PROBLEM IDENTIFIED

**CCW Feedback:** TASK-P1-03 is too complex, mixes concerns, and has unclear IronRDP integration.

**Issues:**
1. Task combines RDP protocol + Server infrastructure + Portal dependencies
2. IronRDP integration details unclear
3. Portal manager needed before RDP server can work
4. Task is 10-14 days (too large for single AI session)

---

## REVISED APPROACH

### Strategy: Break into smaller, focused, executable tasks

**Old Sequence:**
```
P1-01 Foundation → P1-02 Security → P1-03 RDP (huge) → P1-04 Portal
```

**New Sequence:**
```
P1-01 Foundation → P1-02 Security → P1-03 Portal → P1-04 PipeWire →
P1-05 Basic Server → P1-06 Encoders → P1-07 Video Pipeline →
P1-08 RDP Integration → ...
```

---

## NEW TASK BREAKDOWN

### TASK-P1-03-REVISED: Portal Integration (FIRST)
**Duration:** 5-7 days
**Dependencies:** P1-01, P1-02
**Reason:** Portal is needed for everything else

**Includes:**
- Complete PortalManager implementation
- ScreenCast portal
- RemoteDesktop portal
- Clipboard portal
- Session management
- **Outputs:** PipeWire FD, stream info, input injection methods

**This is achievable and testable standalone**

---

### TASK-P1-04-REVISED: PipeWire Integration
**Duration:** 5-7 days
**Dependencies:** P1-03
**Reason:** Get video frames flowing before worrying about RDP

**Includes:**
- PipeWire stream connection
- Frame reception
- Format negotiation
- **Output:** VideoFrame structs ready for encoding

**This is achievable and testable (save frames to disk)**

---

### TASK-P1-05-REVISED: Basic Network Server
**Duration:** 3-5 days
**Dependencies:** P1-02
**Reason:** Get server accepting connections first

**Includes:**
- TCP listener
- TLS wrapping (using P1-02)
- Connection manager
- Session tracking
- **Output:** Accepts TLS connections, doesn't do RDP yet

**This is achievable and testable**

---

### TASK-P1-06-REVISED: Video Encoders
**Duration:** 7-10 days
**Dependencies:** P1-04 (needs VideoFrame definition)
**Reason:** Get encoding working independently

**Includes:**
- Encoder trait
- OpenH264 implementation
- VA-API implementation (if available)
- **Output:** Takes VideoFrame, outputs H.264 NAL units

**This is achievable and testable (encode and save to file)**

---

### TASK-P1-07-REVISED: Video Pipeline
**Duration:** 5-7 days
**Dependencies:** P1-04, P1-06
**Reason:** Connect PipeWire to Encoder

**Includes:**
- Pipeline orchestrator
- Damage tracking
- Cursor extraction
- **Output:** End-to-end video processing

**This is achievable and testable**

---

### TASK-P1-08-REVISED: IronRDP Server Integration
**Duration:** 10-14 days
**Dependencies:** P1-05, P1-07
**Reason:** NOW we add RDP protocol with working components

**Includes:**
- Study IronRDP server API
- Implement RDP protocol handler
- Capability negotiation
- Channel setup
- Integrate video pipeline → RDP graphics channel
- **Output:** Working RDP server showing video

**This is the complex integration task, but now has all pieces ready**

---

### TASK-P1-09-REVISED: Input Handling
**Duration:** 5-7 days
**Dependencies:** P1-03, P1-08
**Reason:** Add input after video works

---

### TASK-P1-10-REVISED: Clipboard
**Duration:** 5-7 days
**Dependencies:** P1-03, P1-08

---

### TASK-P1-11-REVISED: Multi-Monitor
**Duration:** 5-7 days
**Dependencies:** P1-04, P1-07

---

### TASK-P1-12-REVISED: Integration & Testing
**Duration:** 7-10 days
**Dependencies:** All previous

---

## NEXT TASK FOR CCW

Since CCW has completed P1-01 and P1-02, the next logical task is:

**TASK-P1-03-REVISED: Portal Integration**

This is achievable because:
- ✅ Foundation is complete (P1-01)
- ✅ Security is complete (P1-02)
- ✅ ashpd is a mature library with examples
- ✅ Can be tested standalone (creates portal session, gets PipeWire FD)
- ✅ No RDP complexity yet
- ✅ Clear deliverables

---

## RECOMMENDATION

I will now create:
1. **TASK-P1-03-REVISED.md** - Portal Integration (practical, achievable)
2. **CCW-SESSION-PROMPT.md** - Ready-to-use prompt for next CCW session

This will unblock your progress and give you a clear path forward.

---

**Status:** REVISIONS IN PROGRESS
