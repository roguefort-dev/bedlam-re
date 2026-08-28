# Bedlam (1996) — P7 Ports and Packaging: the Deliverable-Map Contract

**Scope:** the P7 opener per `docs/PLAN.md` §6 (P7). This document pins
(1) the P7 scope map VERBATIM from the plan, (2) the deliverable map
that splits that surface into ENGINEERING deliverables (what P7 gates
grade) versus EXTERNAL-CONDITIONAL items (recorded exclusions — the
runner, signing, and publication availability the plan itself names as
non-blocking), (3) the machine-checkable deliverable registry (schema
`p7-ports-map-v1`, a fenced TOML block inside §3), (4) the CDDA
user-supply + local-cache contract, (5) the SteamDeck stretch default
decision, and (6) the gate shape the phase closes on, with the first
P7 required gate (`p7-ports-scaffold`) landing in
`docs/required-gates.toml` BEFORE any packaging work it grades (the
D175 scaffold pattern; D200 and D216 are the immediate precedents).

**Provenance:** unit `p7-ports-scaffold` (D221). BOUNDS: scaffold only —
no engine change and no packaging build lands in this unit (no CI
change, no workflow edit, no installer byte, no engine code); decisions
+ contract artifacts only. Facts cited below: the corpus CDDA shape
(7 audio tracks, ~206–225 s each, 44.1 kHz 16-bit stereo, track 1 = the
data track hence numbering from 02) is VERIFIED
(docs/GROUNDWORK.md; docs/RESEARCH-8STREET.md — parsed with `wave`
against the corpus); the EXW drives CD audio through the MCI path
(VERIFIED, docs/RE-EXW-MAINLOOP.md — the `mciSendCommandA` trio); the
existing per-push CI legs are VERIFIED (.github/workflows/ci.yml:
`ubuntu-latest` + `windows-latest` on every push and pull request);
the scale surface (D215) and the responsive layout (D217) are VERIFIED
landed decisions with green gates.

**Confidence tags:** the §1 quote is VERBATIM plan text (sentence-intact,
whitespace-normalized binding; amend here whenever the plan text changes — the checker
matches it whitespace-normalized, the git history binds it). The
deliverable split, the registry format, the CDDA contract, the
SteamDeck default and the gate wiring are a DECISION (D221), not RE
claims.

---

## 1. The P7 scope map (VERBATIM from PLAN §6, P7)

The following is quoted VERBATIM from `docs/PLAN.md` §6
("P7 — Ports and packaging") — the words and punctuation are the
plan's own, re-wrapped only at sentence boundaries (the checker
matches them whitespace-normalized; the git history binds the
text). It is the whole plan text of the phase; D221 makes it
binding:

> Linux native + Flatpak; Windows installer; macOS universal2 through automated CI. Runner, signing, and publication availability are external conditions and do not block engineering completion.
> CI artifacts per push. CDDA: user-supplied original tracks (WAV/CD), optional local lossy cache generated on first run — never redistributed. SteamDeck defaults stretch.

Consequences D221 records as binding (all grounded in the quote above
and PLAN §3's CI posture — "CI: Linux every commit from the first
window; Windows weekly; automated scheduled macOS CI when a runner is
available. Runner availability is external and does not block other
technical gates"):

- **The three-OS surface is engineering.** Linux native + Flatpak,
  the Windows installer, and the macOS universal2 CI job are
  deliverables P7 gates grade (§2/§3).
- **Per-push CI artifacts are engineering.** "CI artifacts per push" is
  the distribution-facing half of the phase: every push produces
  build artifacts. The repo already runs `ubuntu-latest` +
  `windows-latest` on every push/PR (ci.yml); the P7 landing extends
  that with the artifact-upload jobs (and the macOS leg when a runner
  exists).
- **Runner, signing, and publication availability are EXTERNAL
  conditions and do not block engineering completion.** They are
  recorded as exclusions exactly like the P4 live-capture diagnostics
  (PLAN §6 P4: "Live O1/O2/O3 captures, S0W menu calibration, cycles/
  audio checks, hardware checks, and perceptual review are excluded
  diagnostics, not P4 closure gates or queued work" — the same
  posture, applied to owner-gated/world-gated conditions): P7 gates
  grade only the engineering (§2). No P7 gate may depend on a store,
  a signing key, or a runner being present — that is what makes the
  plan's "do not block engineering completion" sentence mechanical.
- **CDDA is user-supplied and never redistributed.** The packaged game
  never bundles, commits, or distributes the original music tracks or
  any derivative; the optional lossy cache is generated locally on
  first run, in a user-owned location, never redistributed (§4).
- **"SteamDeck defaults stretch" is a PLATFORM DEFAULT, not a mode
  toggle.** A recorded platform-profile default over the landed D215
  scale surface (§5); the generic default stays the shipped posture.

## 2. The deliverable map: ENGINEERING vs EXTERNAL-CONDITIONAL

| Deliverable (`id`) | Kind | Plan anchor | What lands |
|---|---|---|---|
| `linux-native` | engineering | "Linux native + Flatpak" | the release-shaped Linux artifact produced by the per-push artifact job (Linux is the dev platform and an existing per-push CI leg; the P7 landing is the artifact, not the toolchain) |
| `flatpak-manifest` | engineering | "Linux native + Flatpak" | the committed Flatpak build manifest (+ its build definition in CI); Flathub submission is the `publication-stores` exclusion |
| `windows-installer` | engineering | "Windows installer" | the committed installer definition built by the artifact job; Authenticode is the `signing-keys` exclusion |
| `macos-universal2-ci` | engineering | "macOS universal2 through automated CI" | the committed universal2 (aarch64 + x86_64) CI job definition; a hosted macOS runner is the `macos-runner-availability` exclusion |
| `ci-artifacts-per-push` | engineering | "CI artifacts per push" | the per-push artifact-upload jobs extending the existing ci.yml matrix (Linux + Windows now; the macOS leg joins when a runner exists) |
| `cdda-user-supply` | engineering | "CDDA: user-supplied original tracks (WAV/CD), optional local lossy cache generated on first run — never redistributed" | the §4 contract: the user-supply lookup, the silent-miss posture, the user-owned local lossy cache |
| `steamdeck-default` | engineering | "SteamDeck defaults stretch" | the §5 platform-profile default recorded over the landed D215 scale surface |
| `macos-runner-availability` | external-conditional | "Runner, signing, and publication availability are external conditions and do not block engineering completion" | recorded exclusion (PLAN §3 records the same posture for macOS CI); never a gate |
| `signing-keys` | external-conditional | (same sentence) | recorded exclusion: code-signing identities are owner-held secrets; unsigned artifacts are the honest engineering output |
| `publication-stores` | external-conditional | (same sentence) | recorded exclusion: store publication (e.g. the Flathub review queue) is owner-gated distribution, not engineering |

The split's teeth: an EXTERNAL-CONDITIONAL row can never be "landed" by
engineering work (§3 R8 — it carries no status and no gate), and an
ENGINEERING row can never be closed by an external condition (§3 R2 —
it is landed exactly when its proving gate is named). The plan's
non-blocking sentence is therefore not prose but a join discipline.

## 3. The deliverable registry (`p7-ports-map-v1`)

The registry is the machine-checkable form of §2 — a fenced TOML block
inside this doc (the D216 hd-asset-pins-v1 precedent; stdlib `tomllib`,
no deps), schema string `p7-ports-map-v1` fail-closed. One
`[[deliverable]]` row per item:

| Field        | Type | Rule |
|--------------|------|------|
| `id`         | str  | unique, non-empty, whitespace-free |
| `kind`       | str  | one of `engineering`, `external-conditional` |
| `plan_anchor`| str  | non-empty — the plan phrase the row operationalizes |
| `status`     | str  | engineering rows only: `pending` or `landed` |
| `gate`       | str  | engineering rows only: non-empty iff `status = "landed"` — the proving P7 required gate |
| `note`       | str  | required on `external-conditional` rows (the recorded exclusion reason); optional elsewhere |

Mechanical rules (fail-closed, `tools/check-p7-ports-map.py`; the
numbering mirrors the P6 catalog checker so the family reads alike):

- **R1 registry discipline:** exactly one `p7-ports-map-v1` block;
  ids unique and whitespace-free; `kind` in the closed set; unknown
  keys fail; `status`/`gate` on an external-conditional row fail.
- **R2 evidence discipline:** an engineering deliverable is `pending`
  or `landed`; a landed row names its proving gate, a pending row
  carries none (one source of truth — landed exactly when the gate
  is named). A deliverable is landed exactly when its proving gate is
  named, never by an external condition.
- **R3 coverage:** the engineering set is exactly the seven ids of §2
  and the external set exactly the three. Additions or re-scope are a
  DECISIONS entry + a checker update, never silent.
- **R4 gate join:** every named gate resolves to a `[[gate]]` id in
  `docs/required-gates.toml` AND sits in the P7 phase
  `required_gates` list (a landed deliverable is proved by a gate the
  phase actually runs).
- **R5 scaffold-first manifest:** a non-empty P7 `required_gates` list
  starts with `p7-ports-scaffold`, that `[[gate]]` is defined, its
  commands run this checker, and its `tracked_paths` carry this doc,
  both tools, and the manifest (P7 gates can never be wired ahead of
  the contract that grades them — the D175 rule's P7 analogue).
- **R6 phase-close consistency:** manifest P7 `status = "green"`
  requires every engineering deliverable `landed` (P7 cannot close
  with unfinished engineering; necessary, not sufficient — the full
  P7 exit is PLAN §6). P7 status stays pending until every
  engineering deliverable is landed.
- **R7 boundary sentences:** the doc carries the §1 plan sentences
  verbatim (whitespace-normalized matching) plus the CDDA
  never-redistribute boundary and this unit's own bounds.
- **R8 exclusions stay exclusions:** an `external-conditional` row
  carries a note (the recorded reason) and NEVER `status` or `gate`.

Layering (one source of truth per fact): the manifest's own schema is
`tools/validate-required-gates.py`'s job (its suite,
`tools/test-validate-required-gates.py`, applies the strict manifest
key schema to every new gate — this gate included); the deliverable
split + registry + phase-close rule = this checker. It reads ONLY
committed docs — no network, no game-data read, no writes (hermetic,
PATH-free under the validator's bwrap; no `corpus` key, no `writable`).

```toml
schema = "p7-ports-map-v1"

# ---- ENGINEERING deliverables (P7 gates grade these) -------------------

[[deliverable]]
id = "linux-native"
kind = "engineering"
plan_anchor = "Linux native + Flatpak"
status = "landed"
gate = "p7-ci-artifacts"
note = "LANDED with p7-ci-artifacts: the native Linux artifact is the release binary (target/release/bedlam-shell) that the ci.yml build matrix's ubuntu-latest leg uploads on every push (artifact bedlam-shell-linux-x86_64, if-no-files-found: error, engine binary only, unsigned)."

[[deliverable]]
id = "flatpak-manifest"
kind = "engineering"
plan_anchor = "Linux native + Flatpak"
status = "pending"
gate = ""
note = "The committed Flatpak build manifest and its build definition; Flathub submission is the publication-stores exclusion."

[[deliverable]]
id = "windows-installer"
kind = "engineering"
plan_anchor = "Windows installer"
status = "pending"
gate = ""
note = "The committed installer definition built by the artifact job; Authenticode is the signing-keys exclusion."

[[deliverable]]
id = "macos-universal2-ci"
kind = "engineering"
plan_anchor = "macOS universal2 through automated CI"
status = "pending"
gate = ""
note = "The committed universal2 (aarch64 + x86_64) CI job definition that runs when a runner exists; the runner itself is the macos-runner-availability exclusion."

[[deliverable]]
id = "ci-artifacts-per-push"
kind = "engineering"
plan_anchor = "CI artifacts per push"
status = "landed"
gate = "p7-ci-artifacts"
note = "LANDED with p7-ci-artifacts: per-push upload steps inside the existing ci.yml build matrix -- every push uploads the release binary from each leg (bedlam-shell on ubuntu-latest -> artifact bedlam-shell-linux-x86_64, bedlam-shell.exe on windows-latest -> bedlam-shell-windows-x86_64) via actions/upload-artifact@v4 with if-no-files-found: error and 14-day retention; the artifact is the engine binary only (never game-data, never assets), unsigned, no credential; the macOS leg joins when a runner exists (macos-universal2-ci)."

[[deliverable]]
id = "cdda-user-supply"
kind = "engineering"
plan_anchor = "CDDA: user-supplied original tracks (WAV/CD), optional local lossy cache generated on first run"
status = "pending"
gate = ""
note = "The section 4 contract: user-supplied originals, the silent-miss lookup, the user-owned local lossy cache — never redistributed."

[[deliverable]]
id = "steamdeck-default"
kind = "engineering"
plan_anchor = "SteamDeck defaults stretch"
status = "pending"
gate = ""
note = "The section 5 platform-profile default: fill-the-panel on the 1280x800 16:10 panel, recorded over the landed D215 scale surface; the generic default stays Integer + Nearest."

# ---- EXTERNAL-CONDITIONAL items (recorded exclusions, never gates) -----

[[deliverable]]
id = "macos-runner-availability"
kind = "external-conditional"
plan_anchor = "Runner, signing, and publication availability are external conditions and do not block engineering completion"
note = "PLAN sec 3 records the same posture for macOS CI (runner availability is external and does not block other technical gates; goldens never run on macOS CI): an automated macOS runner is a world/owner condition. The engineering is the committed universal2 job (macos-universal2-ci); the runner itself is never a gate."

[[deliverable]]
id = "signing-keys"
kind = "external-conditional"
plan_anchor = "Runner, signing, and publication availability are external conditions and do not block engineering completion"
note = "Code-signing identities (Windows Authenticode, macOS notarization certificates, Flatpak GPG keys) are owner-held secrets; unsigned artifacts are the honest engineering output and no P7 gate ever requires a key."

[[deliverable]]
id = "publication-stores"
kind = "external-conditional"
plan_anchor = "Runner, signing, and publication availability are external conditions and do not block engineering completion"
note = "Store publication (the Flathub review queue, any store page) is owner-gated distribution, not engineering; the exclusion is recorded — exactly like the P4 live-capture diagnostics — so the phase grades only the engineering."
```

## 4. The CDDA user-supply + local-cache contract

Grounded facts (VERIFIED): the original CD is mixed-mode — track 1 is
the data track and tracks 2..8 are seven CDDA audio tracks (~206–225 s
each, 44.1 kHz 16-bit stereo; the corpus carries the WAV rips
`BEDLAM02..08.WAV` — GROUNDWORK.md, RESEARCH-8STREET.md); the EXW
drives them through the MCI CD-audio path (RE-EXW-MAINLOOP.md).

The contract (D221; the plan's "CDDA: user-supplied original tracks
(WAV/CD), optional local lossy cache generated on first run — never
redistributed" made operational):

- **USER-SUPPLIED ORIGINALS.** The packaged game never bundles, never
  commits, never distributes the music tracks or any derivative. The
  user supplies the original tracks — WAV rips of the original CD
  audio, or the audio CD itself. Git stays engine-only (PLAN §1/§5:
  no assets, no asset-derived dumps; the MANIFEST holds hashes only).
- **LOOKUP + SILENT MISS.** The engine resolves music through a
  documented lookup over user-supplied locations. A miss is MUSIC
  SILENT + a stderr note, never fatal, never a task (the §11
  "music fallback = CDDA" posture made honest: the fallback itself
  must never break the game; the 8street comparator's own
  CDDA-disabled build is standing evidence the game runs
  music-silent).
- **OPTIONAL LOCAL LOSSY CACHE, GENERATED ON FIRST RUN.** On first
  play the engine MAY transcode the user-supplied tracks to a lossy
  codec into a USER-OWNED cache directory (the XDG cache home or the
  platform equivalent — never `game-data/`, never the repo, never any
  artifact of the build). The cache is keyed by source identity and
  regenerated on mismatch; it exists to cut decode cost and disk; the
  user's WAV/CD sources remain the source of truth. The cache is
  never redistributed — it is a derived copy, the D21
  originals-or-derivatives rule applied to audio.
- **PARITY BOUNDS.** The music path stays OUT of the sim (the D17 b
  presentation bucket; audio never enters a hash — the D212 posture);
  with no user tracks the shipped behavior is music-silent,
  corpus-faithful, never an error.

## 5. The SteamDeck stretch default

The plan sentence "SteamDeck defaults stretch" is a PLATFORM DEFAULT,
not a mode toggle (D200 layering: a platform knob OUT of
`ModeConfig`; both pacing arms accept it identically; it selects
nothing in the sim).

DECIDED (D221): on the SteamDeck platform profile — the 1280x800
16:10 panel — the shipped default becomes FILL-THE-PANEL (stretch:
the panel is never left with pillarbox bars by default), recorded
over the landed D215 scale surface (`ScaleMode` Integer/Fit/Fill +
`FilterMode` Nearest/Linear, consumed by the parity pipeline's GPU
scale path):

- Generic platforms keep the shipped default bit-for-bit — Integer +
  Nearest exactly as the original DOS framebuffer posture (pinned by
  the D215 test `scaling_defaults_to_the_shipped_integer_nearest`;
  that pin must stay green when this deliverable lands).
- The SteamDeck profile overrides the default `ScaleMode` to the
  fill-the-panel arm — over the landed surface that arm is `Fill`
  (fill the whole target). If the delivering unit instead lands an
  explicit aspect-distorting `Stretch` arm (fill the panel WITHOUT
  the Fill arm's centered crop), that landing records the exact arm
  in this registry row's note. The USER-VISIBLE posture this
  contract pins is: the 16:10 panel is filled edge to edge by
  default.
- The default is a platform PROFILE default selected at startup,
  overridable by the same `--scale`/`--filter` CLI that already
  exists (D215). Platform identification (how a SteamDeck is
  recognized) is the delivering unit's scope; the identification must
  be recorded when it lands.

## 6. Gate wiring (the first P7 required gate) + the gate shape the phase closes on

Per the D175 pattern the contract lands before any packaging work:

- `docs/required-gates.toml` P7 `required_gates =
  ["p7-ports-scaffold"]` (the FIRST entry; R5 enforces it stays first
  once more P7 gates land).
- Gate commands = the fail-closed checker + its hermetic test suite
  (`tools/check-p7-ports-map.py`, `tools/test-p7-ports-map.py`);
  `tracked_paths` = this doc, both tools, and the manifest. No
  `corpus` key (the gate reads no corpus); no `writable`.
- The gate validates the CONTRACT (split + registry + joins +
  boundary sentences), not P7 completion: it is green from the moment
  it lands (all engineering rows `pending` is the honest scaffold
  state — the D200 "empty is the honest state" principle). P7 status
  stays `pending` until the phase's actual exit.

**The gate shape the phase closes on:** P7 closes when (a) every
engineering deliverable in the registry is `landed` with its proving
gate named (R2/R4) — each such gate hermetic and offline, never
requiring a store, a signing key, or a runner present (the exclusion
discipline is what makes "external conditions do not block
engineering completion" mechanical) — and (b) the bounded
`--phase P7` validator verdict is green over the full gate list. R6
makes the eventual flip surveyable: `green` requires all engineering
rows landed. Later P7 gates land per deliverable (for example: a
per-push-artifacts gate that checks the artifact jobs' definitions
committed + hermetically parseable; a Flatpak-manifest gate; an
installer-definition gate; a CDDA-contract gate over the landed
lookup + cache surface; the SteamDeck-default gate over the recorded
platform profile), each behind the scaffold.

**Landed since (unit p7-ci-artifacts, D222):** the SECOND P7 gate
`p7-ci-artifacts` is that named example made real — the per-push
artifact-upload steps landed inside the existing ci.yml build matrix
(ubuntu-latest + windows-latest legs; the macOS leg joins with
`macos-universal2-ci` when a runner exists), and the gate grades the
committed definition hermetically: `tools/check-p7-ci-artifacts.py`
parses `.github/workflows/ci.yml` offline (stdlib-only YAML-subset
reader) and proves the per-push trigger, the release matrix, the two
binary uploads (`if-no-files-found: error`), and the absence of any
signing material. The registry rows `ci-artifacts-per-push` +
`linux-native` flipped `landed` in the same commit (R2).

## 7. P7 acceptance surface (pointer, not re-statement)

The full P7 phase definition (Linux native + Flatpak; Windows
installer; macOS universal2 through automated CI; the
external-conditions posture; CI artifacts per push; the CDDA
contract; the SteamDeck default) is PLAN §6 (P7) — this doc does not
restate it beyond the §1 verbatim quote it operationalizes. Every P7
unit cites the plan sentence it implements; divergences from the plan
are DECISIONS.md entries, never silent.
