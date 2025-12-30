# IronRDP Bug Submission - Minimal Approach
**Date:** 2025-12-10

---

## THEIR NORMS (From Research)

**README.md says:**
> Report bugs in the [issue tracker](https://github.com/Devolutions/IronRDP/issues)

**But recent PRs:** Submitted directly (no issue first)

**Style observed:**
- Conventional commits: `fix(scope): brief description`
- PR body: 1-2 sentences, technical only, no markdown
- Example: "Removes incorrect len() calls. Type doesn't implement len()."

**CI:** Run `cargo xtask ci` locally before submitting

---

## RECOMMENDATION: ISSUE FIRST (Gentlest)

**Why issue not PR for first contact:**
1. ✅ Their README explicitly says "report bugs in issue tracker"
2. ✅ Less commitment (they can ignore or fix themselves)
3. ✅ Tests reception without code review burden
4. ✅ Gives them choice (fix themselves or ask for PR)

**Issue Template (ULTRA-MINIMAL):**

Title:
```
ironrdp-server: SvcProcessorMessages.len() method doesn't exist
```

Body:
```
crates/ironrdp-server/src/server.rs lines 556, 572 call len() on SvcProcessorMessages type which doesn't implement the method.
```

**That's it. 19 words.**

---

## IF THEY RESPOND POSITIVELY

**They might:**
- Fix it themselves
- Ask for a PR
- Say "thanks" and merge their own fix

**If they ask for PR:**
Then submit minimal PR with fix.

---

## VERIFICATION NEEDED FIRST

**Check if bug exists on their master:**
```bash
cd /home/greg/repos/ironrdp-work/IronRDP
git checkout allan2/master
grep -n "\.len()" crates/ironrdp-server/src/server.rs | grep -v "^\s*//"
```

**If len() calls exist:** Bug confirmed → Submit issue
**If not:** Already fixed → No action needed

---

## NEXT STEP

**Option 1:** I verify bug exists, draft issue, show you, await approval
**Option 2:** You want to review their repo yourself first
**Option 3:** Skip issue, go straight to PR (less gentle but faster)

**Your direction?**
