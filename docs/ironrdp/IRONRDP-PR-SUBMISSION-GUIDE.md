# IronRDP PR Submission Guide
## Server Clipboard Ownership Fix
**Date:** 2025-12-11
**Branch:** fix/server-clipboard-announce
**Target:** Devolutions/IronRDP

**Note:** This guide is based on analysis of 50 recent IronRDP PRs. See IRONRDP-PR-PATTERNS-ANALYSIS.md for detailed patterns. This submission matches successful external contributors (ymarcus93, rhammonds) exactly.

**Previous attempt (PR #1037):** Closed as duplicate. Had excessive markdown formatting (8 sections, emojis, checklists, bold text). This version is plain text only.

---

## PR TITLE

```
fix(cliprdr): allow servers to announce clipboard ownership
```

---

## PR DESCRIPTION

Copy this text exactly into the GitHub PR description field:

```
Servers can now send Format List PDU via initiate_copy() regardless of internal state. The existing state machine was designed for clients where clipboard initialization must complete before announcing ownership.

MS-RDPECLIP Section 2.2.3.1 specifies that Format List PDU is sent by either client or server when the local clipboard is updated. Servers should be able to announce clipboard changes immediately after channel negotiation.

This change enables RDP servers to properly announce clipboard ownership by bypassing the Initialization/Ready state check when R::is_server() is true. Client behavior remains unchanged.
```

---

## SUBMISSION INSTRUCTIONS

### Step 1: Navigate to GitHub

1. Open browser to: https://github.com/Devolutions/IronRDP
2. Click "Pull requests" tab
3. Click green "New pull request" button

### Step 2: Select Branches

**Base repository:** `Devolutions/IronRDP`
**Base branch:** `master`

**Head repository:** `glamberson/IronRDP`
**Head branch:** `fix/server-clipboard-announce`

Click "Create pull request"

### Step 3: Fill PR Form

**Title field:**
```
fix(cliprdr): allow servers to announce clipboard ownership
```

**Description field:**
Paste the PR DESCRIPTION text from above (the entire block)

### Step 4: Submit

1. Review the diff shown (should be 1 file, +26/-20 lines)
2. Verify commit message is clean (no extra commits)
3. Click "Create pull request"

---

## EXPECTED RESPONSE SCENARIOS

### Scenario A: Quick Acceptance (Ideal)
- Maintainer reviews and merges within 1-3 days
- Minimal or no change requests
- **Your action:** None needed, update wrd-server to track upstream

### Scenario B: Change Requests
- Maintainer requests modifications (tests, documentation, etc.)
- **Your action:** Decide if worth engaging or maintaining fork

### Scenario C: Rejection - Design Disagreement
- They don't want servers to bypass state machine
- They consider it client-only crate
- **Your action:** Politely acknowledge, maintain permanent fork

### Scenario D: Rejection - Community Tier
- "This is community tier, we don't maintain server features"
- **Your action:** Maintain fork, minimal upstream engagement

### Scenario E: Silence (No Response)
- No response after 7 days
- **Your action:** Ping once, then assume permanent fork

---

## RESPONSE TEMPLATES

### If They Request Changes You Disagree With

```
Thanks for reviewing. I'll consider the feedback, but this change is critical
for our server implementation to comply with MS-RDPECLIP Section 2.2.3.1.

If this doesn't align with IronRDP's direction, I'm happy to maintain our fork
and contribute bug fixes separately.
```

### If They Ask for Tests

```
The existing fuzzer for initiate_copy() should cover both client and server
paths. I can add an explicit test case for server clipboard announcement if
you'd like - let me know the preferred approach.
```

### If They Ask for Documentation

```
Happy to add doc comments. Would you prefer documentation on the function
itself, or in a module-level comment explaining server vs client behavior?
```

### If Rejected Due to Community Tier

```
Understood. We'll maintain our fork for server-specific features. Would you
be interested in bug fixes for other parts of the codebase, or prefer we
minimize upstream PRs?
```

---

## POST-SUBMISSION ACTIONS

### Immediate (After Submitting PR)

1. **Do NOT respond immediately to comments**
   - Wait at least 2 hours before replying
   - Read all comments carefully before responding
   - Keep responses technical and brief

2. **Do NOT make additional commits to the branch**
   - Wait for their review first
   - Only push changes if explicitly requested

3. **Monitor notifications**
   - Check GitHub once per day
   - Don't appear over-eager or impatient

### If PR Gets Merged

1. Delete your fork branch: `fix/server-clipboard-announce`
2. Update wrd-server Cargo.toml to use upstream:
   ```toml
   ironrdp-cliprdr = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
   ```
3. Test wrd-server still works with upstream
4. Delete `from-devolutions-clean` branch (no longer needed)
5. Archive fork or delete it

### If PR Gets Rejected or Ignored (After 7+ Days)

1. Keep `fix/server-clipboard-announce` branch
2. Update wrd-server Cargo.toml to point to this branch:
   ```toml
   ironrdp-cliprdr = { git = "https://github.com/glamberson/IronRDP", branch = "fix/server-clipboard-announce" }
   ```
3. Set up monthly rebase schedule:
   - 1st of every month: `git fetch devolutions master && git rebase devolutions/master`
4. Document decision in fork strategy
5. Move forward with your product

---

## WHAT TO AVOID

### DON'T DO THIS:

1. **Don't explain too much**
   - No long paragraphs about your use case
   - No defensive justifications
   - Keep it technical and brief

2. **Don't mention:**
   - That you had difficulties with them before
   - That this is for a commercial product
   - Your timeline or urgency
   - How long you spent on this
   - Alternative approaches you considered

3. **Don't add:**
   - Emojis or casual language
   - Multiple follow-up comments
   - Links to your fork or issues
   - Requests for expedited review

4. **Don't argue about:**
   - Whether servers "should" work differently
   - IronRDP's architecture decisions
   - Community tier vs core tier policies
   - Their maintenance priorities

### DO THIS:

1. **Be technical**
   - Reference the spec (MS-RDPECLIP)
   - Explain the change clearly
   - Show it's minimal and safe

2. **Be patient**
   - Give them time to review
   - Accept their decision gracefully
   - Have your fallback ready (maintain fork)

3. **Be professional**
   - Thank them for their time
   - Acknowledge their expertise
   - Keep responses brief

---

## DECISION TREE AFTER SUBMISSION

```
                    PR SUBMITTED
                         |
         +---------------+---------------+
         |                               |
    ACCEPTED (1-7 days)            REJECTED/SILENT (7+ days)
         |                               |
         |                               |
    Use upstream                    Maintain fork
    Delete fork                     Monthly rebase
    Move forward                    Move forward
         |                               |
         +---------------+---------------+
                         |
                  Product continues
                  (either way works)
```

---

## FINAL NOTES

**Remember:**
- This PR is not critical to your product's success
- You can maintain the fork indefinitely with minimal overhead
- The goal is to TEST their receptivity, not to convince them
- If rejected: you lose nothing, you already have a working solution
- If accepted: slight convenience, less maintenance

**Your position is strong:**
- Your patch is spec-compliant
- Your code is clean
- Your fork works
- You don't need them more than they need server contributors

**Maintain this mindset:**
- Professional courtesy, not pleading
- Technical discussion, not emotional investment
- Take it or leave it, not negotiation

---

## SUBMISSION CHECKLIST

Before clicking "Create pull request":

- [ ] Title is exactly: `fix(cliprdr): allow servers to announce clipboard ownership`
- [ ] Description is copied from this document (no modifications)
- [ ] Base is `Devolutions/IronRDP:master`
- [ ] Head is `glamberson/IronRDP:fix/server-clipboard-announce`
- [ ] Diff shows only 1 file changed (+26/-20)
- [ ] Commit message is clean (no debug commits, no extra commits)
- [ ] You're prepared to walk away if rejected

---

**END OF SUBMISSION GUIDE**

You are ready. Submit when you're ready to test their receptivity.
