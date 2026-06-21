# Spec delta — dependency-security

## ADDED Requirements

### Requirement: A target-scoped advisory gate enforces the Rust dependency security posture

The repository SHALL define a `cargo-deny` configuration at `src-tauri/deny.toml` whose advisory graph is
scoped via `[graph].targets` to the **shipped Apple targets** (`aarch64-apple-darwin`,
`x86_64-apple-darwin`). The advisory gate (`cargo deny check advisories`) run against this configuration
SHALL exit cleanly. The configuration SHALL NOT blanket-ignore the Linux-backend gtk3-rs/`glib` advisories
by ID, SHALL change no shipped crate (`git diff` against the base for `Cargo.toml` and `Cargo.lock` SHALL be
empty), and SHALL change no source, schema, IPC, capability, or `tauri.conf.json` content.

#### Scenario: Scoped advisory check passes on the shipped target

- **WHEN** `cargo deny check advisories` runs against `src-tauri/deny.toml` with the graph scoped to the
  Apple targets
- **THEN** the check exits 0 with no advisory errors
- **AND** the gtk3-rs chain (`glib`, `gtk`, `gdk`, `atk`, `gtk3-macros`) does not appear in the evaluated
  dependency graph, because those crates are Linux/BSD-gated and not compiled into the macOS artifact
- **AND** `Cargo.toml` and `Cargo.lock` are unchanged relative to the base branch.

#### Scenario: Adding a Linux target re-surfaces the deferred advisories (tripwire)

- **WHEN** a Linux triple (e.g. `x86_64-unknown-linux-gnu`) is added to `[graph].targets`
- **THEN** the advisory check re-reports `glib` RUSTSEC-2024-0429 and the gtk3-rs "no longer maintained"
  cluster
- **AND** the posture must be re-evaluated (real upstream fix or an explicit, fresh acceptance) rather than
  the advisories being silently suppressed.

### Requirement: The deferred Linux-only advisories are documented as an auditable risk acceptance

The change SHALL record, in `deny.toml` comments and in the change's security review, the deferred
Linux-only advisory IDs (at minimum `glib` RUSTSEC-2024-0429, `proc-macro-error` RUSTSEC-2024-0370, and the
gtk-rs GTK3 unmaintained cluster), each with the rationale that it is **not reachable on the macOS-only
shipped target** and has **no upstream fix** (gtk3-rs end-of-life; `glib` soundness fixed only in the gtk4
ecosystem). The deferred IDs SHALL be generated from an actual advisory scan, not hand-copied.

#### Scenario: Risk acceptance is traceable to a scan

- **WHEN** a reviewer inspects the change
- **THEN** the deferred advisory IDs are present with per-ID rationale and were produced from a real
  `cargo deny`/`cargo audit` run
- **AND** the documented review trigger states that the acceptance is revisited if a Linux build target is
  added or if Tauri's Linux backend migrates off GTK3.
