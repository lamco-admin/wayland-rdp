# SPECIFICATION REMEDIATION PLAN
**Status:** APPROVED FOR EXECUTION
**Scope:** Complete revision to production-grade PRD/SRS
**Estimated Effort:** 40+ hours of specification work
**No Shortcuts:** ZERO TODOs, placeholders, or simplifications allowed

---

## REMEDIATION STRATEGY

### Phase 1: Create Missing Foundation Documents (Priority 1)
**Effort:** 8-10 hours

1. **04-DATA-STRUCTURES.md** (NEW - COMPLETE)
   - Define ALL data structures with exact fields, types, sizes
   - Include serialization specs
   - Include validation rules
   - ~800 lines

2. **05-PROTOCOL-SPECIFICATIONS.md** (NEW - COMPLETE)
   - Portal D-Bus protocol (all methods, signals, errors)
   - PipeWire protocol (negotiation, buffers, DMA-BUF)
   - IronRDP traits (complete API specifications)
   - ~1000 lines

3. **03-PROJECT-STRUCTURE.md** (NEW - COMPLETE)
   - Complete directory tree with all files
   - Module organization rules
   - Naming conventions
   - ~400 lines

4. **IRONRDP-ARCHITECTURE-AUTHORITATIVE.md** (NEW - COMPLETE)
   - IronRDP-based architecture (replacing old encoder approach)
   - RemoteFX codec specifications
   - BitmapUpdate conversion approach
   - Complete integration patterns
   - ~600 lines

### Phase 2: Update Core Specifications (Priority 1)
**Effort:** 6-8 hours

5. **Update 00-MASTER-SPECIFICATION.md**
   - Remove ALL H.264 references
   - Add RemoteFX specifications
   - Update technology table
   - Update milestones
   - Fix task file references
   - ~2 hours

6. **Update 01-ARCHITECTURE.md**
   - Revise video pipeline for BitmapUpdate approach
   - Remove encoder components
   - Add IronRDP integration architecture
   - Update data flows
   - ~3 hours

7. **Update 02-TECHNOLOGY-STACK.md**
   - Fix IronRDP dependencies (ironrdp-server = "0.9")
   - Remove OpenH264, VA-API
   - Add complete, correct Cargo.toml
   - ~2 hours

### Phase 3: Complete Critical Task Specifications (Priority 1)
**Effort:** 12-15 hours

8. **TASK-P1-05-PIPEWIRE.md** (COMPLETE REWRITE)
   - Full PipeWire integration specification
   - Format negotiation protocol
   - DMA-BUF zero-copy implementation
   - Buffer management
   - Multi-stream handling
   - Complete code examples (~500 lines of code)
   - Integration tests
   - Performance requirements
   - ~1200 lines total

9. **TASK-P1-08-BITMAP-CONVERSION.md** (NEW, replacing P1-08)
   - Complete PipeWire frame → BitmapUpdate conversion
   - Format conversion (all PipeWire formats → BGRA/XRGB)
   - Stride calculation
   - Buffer management
   - Complete implementation
   - ~800 lines

10. **TASK-P1-09-IRONRDP-SERVER-INTEGRATION.md** (COMPLETE REWRITE)
    - RdpServerInputHandler trait implementation (complete)
    - RdpServerDisplay trait implementation (complete)
    - IronRDP server builder configuration
    - Connection lifecycle
    - Error handling
    - Complete integration
    - ~1000 lines

### Phase 4: Revise Remaining Tasks (Priority 2)
**Effort:** 8-10 hours

11. **TASK-P1-10-INPUT-HANDLING.md** (MAJOR REVISION)
    - Add complete RDP scancode → Linux evdev mapping table (200+ mappings)
    - Add coordinate transformation formulas
    - Add multi-monitor coordinate handling
    - Update for RdpServerInputHandler trait
    - ~800 lines

12. **TASK-P1-11-CLIPBOARD.md** (MAJOR REVISION)
    - Add complete MIME type ↔ RDP format mapping table
    - Add image format conversion specs (BMP, PNG, DIB)
    - Add clipboard loop prevention algorithm
    - Update for IronRDP CliprdrServer
    - Complete Portal clipboard integration
    - ~700 lines

13. **TASK-P1-12-MULTIMONITOR.md** (COMPLETE)
    - Add layout calculation algorithm
    - Add coordinate system specification
    - Add monitor topology formulas
    - Add DisplayControl integration details
    - ~600 lines

14. **TASK-P1-13-TESTING.md** (COMPLETE)
    - Add detailed test specifications for all tests
    - Add test data specifications
    - Add acceptance criteria
    - Add performance test procedures
    - ~800 lines

### Phase 5: Create Consolidated Phase Documents (Priority 3)
**Effort:** 4-6 hours

15. **PHASE-1-SPECIFICATION.md** (NEW - COMPLETE)
    - Consolidate all Phase 1 tasks
    - Overall phase objectives
    - Phase acceptance criteria
    - Integration testing
    - ~500 lines

16. **PHASE-2-SPECIFICATION.md** (NEW - COMPLETE)
    - Audio capture specifications
    - Audio encoding (Opus)
    - RDP audio channels
    - A/V synchronization
    - ~600 lines

### Phase 6: Polish and Finalize (Priority 4)
**Effort:** 2-4 hours

17. **Update reference/DEPLOYMENT-GUIDE.md**
    - Fix systemd for session user
    - Add portal configuration
    - Add monitoring setup
    - ~200 lines addition

18. **Archive obsolete documents**
    - Move P1-06, P1-07, old P1-08, old P1-09 to archived/
    - Update index files
    - Update README

---

## EXECUTION PLAN

Due to the massive scope (estimated 6,000-8,000 new lines of specification), I will:

1. **Use Task agents** to create each major document in parallel
2. **Create documents in priority order** (foundations first)
3. **Each document will be COMPLETE** with zero TODOs or placeholders
4. **Verify completeness** of each before moving to next
5. **Integrate and cross-reference** all documents
6. **Final review pass** for consistency

---

## SUCCESS CRITERIA

Remediation is COMPLETE when:
- [ ] ALL 6 missing specification files created
- [ ] ALL core specs updated for IronRDP architecture
- [ ] ALL task specifications complete with full implementations
- [ ] ZERO TODOs, placeholders, or shortcuts remain
- [ ] ALL data structures fully defined
- [ ] ALL protocols fully specified
- [ ] ALL code examples are complete and runnable
- [ ] ALL tests are fully specified
- [ ] Specifications are internally consistent
- [ ] Production-grade PRD/SRS quality achieved

---

## ESTIMATED TIMELINE

**Total Effort:** 40-50 hours of specification work
**With AI Agents:** Can parallelize, complete in 2-3 days of intensive work
**Priority:** CRITICAL - Must complete before continuing implementation

**Status:** PLAN APPROVED, BEGINNING EXECUTION
