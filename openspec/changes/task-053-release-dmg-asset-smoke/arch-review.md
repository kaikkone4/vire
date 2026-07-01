# Architecture review — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

**Verdict:** PASS. Single-surface, minimal release-distribution/docs change. No component boundary
crossed, no source/schema/IPC/capability change, no split required, no BA escalation.

## Scope validated

The task's stated scope (validate/build the aarch64 DMG, attach it to Release `v0.8.1`, add README
download docs + a release smoke checklist, verify install/open/update-check) is coherent and
proportional. It is confined to:

1. the **v0.8.1 GitHub Release** (one uploaded asset), and
2. repo **install docs** (`README.md` one subsection + example name bump; `RELEASE.md` one checklist
   section).

No change to the Vire app binary or any file under `src-tauri/src`. Confirmed exclusions per
TASK-042/050 lineage: **no** `tauri-plugin-updater` / auto-install, **no** code-signing /
notarization / Developer ID, **no** release-feed automation or CI release workflow.

## Component-boundary check (against `artifacts/ba/03_architecture_plan.md`)

The architecture plan's components are the desktop app's internal modules (capture, langfuse
import, update-check, storage/schema, frontend). This change touches **none** of them — it operates
on the *distribution surface* (a GitHub Release artifact) and *documentation*, which sit outside the
app's runtime component graph. No boundary is crossed and nothing needs splitting. I did not need to
redesign any component; there is no architecture divergence to file back to BA (`feedback_to_ba[]`
empty).

## Key architectural note — asset-independence of the update-check path

`check_for_update` (`src-tauri/src/update_check/mod.rs:26`) performs a read-only GET to
`api.github.com/repos/kaikkone4/vire/releases/latest` and consumes **only** `tag_name` + `html_url`
(`GithubRelease` struct, mod.rs:47-51). It does **not** read, require, or download release **assets**.

Consequence for this task: attaching the DMG changes the **human download/install** experience; it
does **not** change update-check behavior. With `v0.8.1` as the latest release and the app running
`0.8.1` (single-sourced: `tauri.conf.json:4`, `Cargo.toml`, `env!("CARGO_PKG_VERSION")`), the check
already returns `UpToDate` today. The smoke checklist's "Check for updates → up to date" step
therefore verifies the *version/update wiring end-to-end*, not the asset. This is why the change is
docs+ops only and requires no touch to `update_check`.

## Naming / packaging — verified, not assumed

Tauri v2's macOS DMG target names artifacts `{productName}_{version}_{arch}.dmg`. This is confirmed
empirically, not inferred: the on-disk bundle is
`src-tauri/target/release/bundle/dmg/Vire_0.8.1_aarch64.dmg` (5.5 MB, sha256
`e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`), the packaged `Vire.app` inside
reports `CFBundleShortVersionString = 0.8.1`, and README §Artifact location documents the same
pattern. No official-doc lookup was required — the toolchain output is authoritative here.

## Outward-facing action — authorization gate (FB-note for Pi-Assistant)

Task step 2 (`gh release upload v0.8.1 …`) **mutates the public GitHub Release** — an outward-facing
publish. The architect deliberately does **not** perform it in this SW-1 design pass. The dev/ops
role must obtain Janne's explicit go-ahead before uploading (the asset becomes publicly downloadable
and is effectively permanent). The authenticated account (`kaikkone4`) owns `kaikkone4/vire` and has
permission; the only gate is authorization, not access.

## Risks / constraints for the dev/ops role

- **Stale-artifact risk (A1).** The DMG is under gitignored `target/`. Embedded version is `0.8.1`,
  but rebuild-from-tagged-commit **or** a recorded sha256 in `RELEASE.md` is required so the shipped
  bytes are traceable. Do not upload an un-fingerprinted artifact.
- **Single-arch (A2).** aarch64 only; Intel Macs are out of scope for v0.8.1. If an x86_64/universal
  build is later wanted, that is a new task (would touch build targets, not this change's scope).
- **Unsigned (A3).** Gatekeeper first-launch must be **documented** (right-click → Open), never
  bypassed via `spctl`/quarantine-stripping instructions — that would contradict the security
  posture and is out of scope.

## Handoff

Route to the **dev/ops (release) role**. No design-level questions outstanding; the only pre-upload
gate is Janne's authorization for the outward-facing publish.
