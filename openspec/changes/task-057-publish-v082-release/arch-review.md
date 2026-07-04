# Architecture Review — TASK-057 publish v0.8.2 release

- **Tier:** L2 · **Component:** Vire desktop app + release-ops · **Verdict:** **PASS** (no split, no
  blocking BA escalation)
- **Reviewed against:** `artifacts/ba/03_architecture_plan.md` (update-check / release path), DEC-029
  (U-lite update check, Phase-1), TASK-052 arch-review (ownership/ordering precedent, proposed
  DEC-031), TASK-053/054 (DMG asset + unsigned-launch guidance), and the TASK-056 merge state
  (PR #43 → `main` @ `df79423`).

## 1. Scope alignment — proportional, single boundary

TASK-057 moves **version strings** + a **changelog/README doc update**, then performs one
**release-ops publication** (build aarch64 DMG from `main`, publish full `v0.8.2` Release with the
asset + unsigned-launch notes). No source logic, schema, IPC, CSP, capability, or dependency change;
`src-tauri/src/update_check/` is untouched. This crosses **no** component boundary in
`03_architecture_plan.md`. The update-check capability is already correct on `main`; TASK-057 only
supplies the two real-world inputs a *new* detectable version needs — matching compiled metadata and
a published Release newer than `v0.8.1`.

## 2. Split decision — DO NOT split (keep focused)

For `v0.8.1`, the equivalent work fragmented reactively across three tasks: TASK-052 published
notes-only (assets deferred), TASK-053 later attached the DMG, TASK-054 fixed the unsigned-launch
docs. That fragmentation was **discovery-driven**, not inherent: the DMG build path, the
`Vire_<v>_aarch64.dmg` naming, the single-arch (aarch64-only) choice, and the validated Plan-B xattr
guidance are now all **known and settled**. Folding them into one task removes the 052→053 window
where a published version had no downloadable binary. The architectural separation that *does* matter
— "code change through gates" vs "outward-facing publication" — is preserved **inside** this task as
the Part A / Part B ownership+ordering boundary (below), which is sufficient. A separate
code-bump-vs-release-asset task split would add coordination cost with **no** boundary or risk
benefit. **One task, no split.**

## 3. Ownership & ordering (the key architectural constraint) — inherited from DEC-031 (proposed)

**The public GitHub Release + DMG asset is owned by the Release Manager (SW-6) and published only
*after* the Part-A code change merges to `main`. The version-metadata change is owned by SW-2 and
clears the normal code gates first.** Sequenced, not concurrent. Rationale (same as TASK-052 §3):
publishing a public Release + binary is an outward-facing, hard-to-reverse action that must reference
the exact merged `main` SHA; the tag is authoritative for `/releases/latest`; and the server-side tag
via `gh release create --target <sha>` avoids the recurring local SSH-key passphrase blocker that
deferred every prior signed task-tag. The DMG **must be built from the merged `main` head that
contains TASK-056** — otherwise the shipped binary omits the feature this release exists to surface.

## 4. Correctness of the check against `v0.8.2` (logic unchanged)

`compare_release` (`src-tauri/src/update_check/mod.rs`) strips a leading `v`, parses both sides with
`semver`, returns `UpdateAvailable` iff `latest > current`, else `UpToDate`:

- `current 0.8.1` vs `tag v0.8.2` → `0.8.2 > 0.8.1` → **UpdateAvailable** (real `html_url`). ✓
- `current 0.8.2` vs `tag v0.8.2` → equal → **UpToDate** (never downgrade). ✓

Tag MUST be parseable semver after `v`-stripping (`v0.8.2` → `0.8.2`); a non-semver tag fail-softs to
`Unknown`. No logic change — both arms exercised by the two builds.

## 5. xattr / quarantine — reaffirming Plan B stays user-run (Janne's question)

Confirmed architecturally sound to **not** embed any quarantine bypass in the DMG/installer:
`com.apple.quarantine` is stamped by Gatekeeper on the user's filesystem at download time, is absent
from the uploaded artifact, and cannot be pre-cleared from it; a drag-install DMG runs no script, and
any embedded `xattr`/`spctl` bypass would be both circular (blocked by the same Gatekeeper) and an
anti-pattern (globally weakening the user's Gatekeeper). The **only** artifact-side removal of the
user step is a valid Developer ID signature + Apple notarization. Absent that (TASK-028), Plan B
remains the documented, user-run, per-app remedy — the exact posture validated on a physical Mac in
TASK-054. Fully detailed in `design.md` §xattr.

## 6. Version-tier note (non-blocking)

Bump is `0.8.1 → 0.8.2` (**patch**) per the task directive. Observation for BA/product awareness:
TASK-056 added a user-facing feature (Settings UI), which strict SemVer would treat as minor
(→`0.9.0`), and TASK-056's RELEASE.md suggested a future `0.9.0`. A patch bump is a valid pre-1.0
(`0.x`) choice and fully satisfies the release's goal (any increment `> 0.8.1` triggers
`UpdateAvailable`). Not a blocker; surfaced in the handoff so Janne can choose `0.9.0` instead if
preferred — that would change only the three version strings + tag/notes, no design impact.

## 7. feedback_to_ba (non-blocking)

- **DEC-031 (still proposed, carry-forward):** formalize release-publication ownership/ordering (code
  gates → merge → SW-6 publishes a full Release; server-side tag via `gh release create --target`;
  `/releases/latest` requires non-draft/non-prerelease). TASK-057 relies on it operationally; worth
  pinning in `07_decision_log.md`.
- **FB-054-1 (open):** BA to decide TASK-028 Developer ID signing + notarization vs. accept the
  unsigned/xattr interim. TASK-057 continues the unsigned interim; it does not resolve FB-054-1.

## 8. Verdict

**PASS.** Single component, proportional scope, no boundary crossing, no redesign, no split.
Architecture constraints for implementation: (1) keep `update_check/` untouched, no
`tauri-plugin-updater`/auto-install; (2) version triple (`Cargo.toml` / `tauri.conf.json` /
`Cargo.lock`) must agree at `0.8.2`; (3) Release is non-draft, non-prerelease, `v`-prefixed semver
tag on the merged `main` SHA that includes TASK-056, published by SW-6 after gates; (4) DMG built
from that `main` head, attached, unsigned-launch notes with user-run xattr Plan B and no embedded
bypass.
