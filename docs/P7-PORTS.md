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
status = "landed"
gate = "p7-flatpak-manifest"
note = "LANDED with p7-flatpak-manifest: packaging/dev.roguefort.bedlam.yml — the committed flatpak-builder manifest (app-id dev.roguefort.bedlam, the repo remote's own reverse DNS; runtime org.freedesktop.Platform + sdk org.freedesktop.Sdk at the PINNED runtime-version 24.08; command bedlam-shell; the CLOSED five-token finish-args surface --socket=wayland/--socket=fallback-x11/--socket=pulseaudio/--device=dri/--share=ipc — no host filesystem grant, no network, no bus, no wider device) + its CI build definition (the ci.yml job flatpak on ubuntu-latest: installs flatpak-builder, installs org.freedesktop.Sdk//24.08 + the rust-stable Extension at the SAME pinned version the manifest carries (the version join), builds THIS manifest with flatpak-builder with build/repo dirs OUTSIDE the checkout, exports the UNSIGNED single-file bundle bedlam-shell.flatpak naming the same app-id, uploads it per-push as bedlam-shell-flatpak-x86_64 with if-no-files-found: error + 14-day retention). The bundle carries the ENGINE BINARY ONLY: one simple-buildsystem module (cargo build --release --locked -p bedlam-shell under the rust-stable extension, deliberately not --offline since no vendored set is committed) installing exactly one binary + one desktop entry into /app; the single dir source is the repo root with a skip list whose floor (.git, game-data, game-data-2, derived, derived-2, goldens, ghidra-project, target) is checker-pinned — nothing from the corpus or its derivatives ever enters the copy, and outside the skip list no manifest value references the corpus at all; the desktop entry ships no Icon (no asset ever, D21); the user supplies their OWN original install at run time (bedlam-shell's INSTALL_DIR positional argument, default game-data/BEDLAM; a host grant is the user's own flatpak override, never baked in) and CDDA resolves through the shell's documented lookup (inside the sandbox $XDG_DATA_HOME is the per-app data dir — exactly the second lookup root, no extra grant); no key ever signs anything (signing-keys) and Flathub submission stays the publication-stores exclusion. The gate grades the committed manifest hermetically (tools/check-p7-flatpak-manifest.py, offline)."

[[deliverable]]
id = "windows-installer"
kind = "engineering"
plan_anchor = "Windows installer"
status = "landed"
gate = "p7-windows-installer"
note = "LANDED with p7-windows-installer: packaging/bedlam-shell.nsi — the committed NSIS installer definition (D227) compiled by the ci.yml job windows-installer on windows-latest into the UNSIGNED bedlam-shell-setup.exe, uploaded per-push as bedlam-shell-windows-installer-x86_64. The definition is a CLOSED grammar (checker-enforced command set; unknown commands, plug-ins, compiler directives, labels, C-style comments, line continuations, wildcards, path separators in File sources and switches on Delete/RMDir are all parse errors): Name 'Bedlam engine'; OutFile bedlam-shell-setup.exe; Unicode true; InstallDir $PROGRAMFILES64\\Bedlam with RequestExecutionLevel admin + CRCCheck force; the minimal page flow directory+instfiles (uninstaller uninstConfirm+instfiles); exactly two sections. The install section is pinned instruction-for-instruction: SetOutPath $INSTDIR; exactly TWO staged bare File sources (bedlam-shell.exe staged by the CI job's Copy-Item + windows-installer-README.txt — the closed engine-only file set, nothing else can ride along); WriteUninstaller; the Add/Remove-Programs registration (HKLM Uninstall\\BedlamEngine DisplayName + UninstallString); CreateDirectory $SMPROGRAMS\\Bedlam; ONE CreateShortcut onto the installed engine whose working directory is $INSTDIR (NSIS stores $OUTDIR as the shortcut's working directory; SetOutPath runs first — the engine's documented default lookup root sits directly inside the install folder, and the README spells out the INSTALL_DIR positional too). The uninstall section is the exact inverse: every installed artifact deleted BY NAME (the checker refuses any Delete of a file the installer never wrote), the ARP key removed, RMDir on empty directories only (the recursive switch cannot even parse). The CI job joins: cargo build --release --locked -p bedlam-shell (deliberately not --offline), choco install nsis, the staging Copy-Item, makensis run with working-directory: packaging on THIS script (so every relative path resolves under either candidate rule), upload via actions/upload-artifact@v4 with if-no-files-found: error + 14-day retention. No key ever marks the installer (signing-keys: Authenticode is the owner-held exclusion, denylist enforced across script + README + job, comments included); the corpus token appears nowhere in script or job, and in the README only inside the documented default layout game-data\\BEDLAM."

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
status = "landed"
gate = "p7-cdda-user-supply"
note = "LANDED with p7-cdda-user-supply: engine/bedlam-shell/src/cdda.rs — the documented lookup (explicit --music-dir/BEDLAM_MUSIC_DIR, then $XDG_DATA_HOME/bedlam/music, then the install dir; candidate names BEDLAM0N.WAV then TRACK0N.WAV for CD tracks 02..08, case-insensitive) with the SILENT MISS posture (a miss = music silent + one stderr note, never fatal, never a task), plus the optional local lossy cache (IMA ADPCM 4:1, dependency-free integer math) generated on first run into $XDG_CACHE_HOME/bedlam (default ~/.cache/bedlam; the Windows platform equivalent %LOCALAPPDATA%/bedlam/cache), keyed by source identity (length + FNV-1a-64), regenerated on mismatch, guarded against game-data/ and any git work tree, never redistributed; music stays out of the sim (D17 b/D212) and the headless smoke stays at its recorded baseline."

[[deliverable]]
id = "steamdeck-default"
kind = "engineering"
plan_anchor = "SteamDeck defaults stretch"
status = "landed"
gate = "p7-steamdeck-default"
note = "LANDED with p7-steamdeck-default: the RECORDED platform profile over the landed D215 scale surface — identification = the DMI sysfs identity read once at window startup (/sys/devices/virtual/dmi/id: board_vendor 'Valve' AND product_name 'Jupiter' (the 1280x800 LCD deck) or 'Galileo' (the 1280x800 OLED deck), trimmed + case-insensitive, both fields required, FAIL-CLOSED to Generic on any other identity, missing files, or a non-sysfs platform; the env is deliberately not consulted); on the SteamDeck class the default PresentConfig scale becomes the EXPLICIT ASPECT-DISTORTING Stretch arm this unit lands (the whole 640x480 frame onto the whole panel edge to edge — no bars, no crop; Fill was NOT chosen because its centered crop hides the top and bottom of the game's own 480 rows), the filter default stays Nearest on every platform, and the explicit --scale/--filter words always win; every other machine keeps Integer + Nearest bit-for-bit (the D215 default is untouched, PresentConfig::default() is unchanged); engine/bedlam-shell/src/platform.rs + the Stretch arm in bedlam-platform scale.rs; the profile selects nothing in the sim (D200 layering, pinned by the trajectory/hash invariance test)."

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

**LANDED (unit `p7-cdda-user-supply`, D223):** the contract above
is the shipped surface in `engine/bedlam-shell/src/cdda.rs`
(bedlam-shell/platform only — no engine change; the gate is
`p7-cdda-user-supply`, hermetic `bedlam-shell --lib` + the registry
checker; no corpus read by the gate). The LOOKUP resolves each of
the seven tracks over the ordered roots (1. the explicit
`--music-dir DIR` flag / `BEDLAM_MUSIC_DIR` env, 2. the user's
`$XDG_DATA_HOME/bedlam/music` — default
`$HOME/.local/share/bedlam/music`, 3. the game's own install
directory — the packaged game's user-owned tree; in the repo layout
the operator's read-only corpus copy), matching candidate names
`BEDLAM0N.WAV` then `TRACK0N.WAV` (CD track N = 2..8, the
mixed-mode numbering) case-insensitively, first match in root
order. The SILENT MISS posture is one stderr note (a full or
partial miss names the silent posture + where to put the rips);
nothing is ever fatal. The CACHE transcodes each resolved track
ONCE — first run — into `<cache>/music/trackNN.bcda`: the whole
16-bit PCM track IMA-ADPCM-encoded (a real lossy codec at 4:1, the
repo's dependency-free integer-math posture) behind a small header
carrying the SOURCE IDENTITY (file length + FNV-1a-64 of the
bytes); later runs recompute the identity and REGENERATE exactly
the mismatched entries (write-then-rename, so a torn entry is
impossible; a corrupt or unparseable entry regenerates too). The
cache home is `$XDG_CACHE_HOME/bedlam`, defaulting to
`$HOME/.cache/bedlam` (`%LOCALAPPDATA%/bedlam/cache` on Windows);
`--no-music-cache` opts out (the plan's "optional"). The startup
guard REFUSES a cache home inside the game install tree
(game-data) or inside any git work tree (a `.git` at the root or
an ancestor), and the default construction can never land in
either — the cache is user-owned and NEVER redistributed. Verified
first-hand on the window host: 7/7 resolved with a generated cache
(43-byte header + exactly 1/4 of the PCM bytes), a second run all
FRESH, a modified source regenerating EXACTLY its own entry, both
refusal guards firing with their notes, `--no-music-cache`
disabling, and the headless smoke EXACTLY at its recorded baseline
(scene `696adb1cd110e062` / parity `cce30c983b97b16d` / audio
`110400/158092`) with the flags noted + ignored.

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

**LANDED (unit `p7-steamdeck-default`, D224):** the contract above
is the shipped surface. The IDENTIFICATION is the hardware's own
DMI identity, read ONCE at window startup from the standard sysfs
tree (`engine/bedlam-shell/src/platform.rs`): `board_vendor` =
"Valve" AND `product_name` = "Jupiter" (the 1280x800 LCD deck) or
"Galileo" (the 1280x800 OLED deck), matched trimmed and
case-insensitively; BOTH fields are required and everything else —
any other vendor or product, missing files, a platform with no
sysfs DMI tree — classifies FAIL-CLOSED as Generic (the env is
deliberately not consulted: `STEAMDECK=1` is a Steam-session fact,
not a hardware fact; a desktop exporting it is not a SteamDeck).
The probe is read-only and never fatal. The ARM this unit lands and
records is the explicit aspect-distorting `Stretch` (the second
branch §5 allows): the WHOLE 640x480 frame maps onto the WHOLE
panel edge to edge — no bars, no crop (the `Fill` arm was not
chosen: it fills the panel but center-crops, hiding the top and
bottom of the game's own 480 rows). The new arm rides the landed
D215 surface as a fourth `ScaleMode` (whole frame / whole target
geometry; the full-frame uv of Integer/Fit; the absolute cursor
inverse of Integer/Fit), selectable by the same `--scale stretch`
word (fail-closed domain `integer|fit|fill|stretch`), so a user on
ANY machine can ask for it and a SteamDeck user can still ask for
Integer bars — the CLI word always wins. On a SteamDeck the
startup notes the profile default on stderr
(`--scale` override hint included); the filter default stays
Nearest on every platform (the contract overrides the scale arm
only). PARITY BOUNDS pinned by test: the profile is a platform
knob OUT of `ModeConfig` (D200) that both pacing arms accept
identically; the sim config, executed ticks, tick count, state
hash, scene hash AND frame parity hash are identical under every
class/CLI combination; the headless path never probes DMI (it owns
no surface; the flags note + ignore exactly as before); and the
generic default is `PresentConfig::default()` bit-for-bit — the
D215 pin `scaling_defaults_to_the_shipped_integer_nearest` stays
green (generic platforms keep the shipped Integer + Nearest).

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

**Landed since (unit p7-cdda-user-supply, D223):** the THIRD P7 gate
`p7-cdda-user-supply` proves the §4 contract over the landed shell
surface (`engine/bedlam-shell/src/cdda.rs`) — command 1 is the
hermetic `bedlam-shell --lib` battery (the lookup + silent miss, the
WAV shape, the ADPCM codec pins, the identity-keyed cache verdicts
and regeneration, the containment guards, and the sim-config
invariance), command 2 re-runs `tools/check-p7-ports-map.py` (the
registry flip + gate join). The registry row `cdda-user-supply`
flipped `landed` in the same commit (R2).

**Landed since (unit p7-steamdeck-default, D224):** the FOURTH P7
gate `p7-steamdeck-default` proves the §5 contract over the landed
platform-profile surface (`engine/bedlam-shell/src/platform.rs` +
the Stretch arm in the bedlam-platform scale surface) — command 1
is the hermetic `bedlam-shell --lib` battery (the DMI
identification incl. every fail-closed shape, the profile default
per class, the CLI-wins rule, the fill-the-panel geometry on the
1280x800 panel, the trajectory/hash + both-arms gate-answer
invariance over the profile selection), command 2 re-runs
`tools/check-p7-ports-map.py` (the registry flip + gate join). The
registry row `steamdeck-default` flipped `landed` in the same
commit (R2).

**Landed since (unit p7-windows-installer, D227):** the SIXTH P7
gate `p7-windows-installer` proves the Windows deliverable over
the committed definition — `tools/check-p7-windows-installer.py`
parses `packaging/bedlam-shell.nsi` offline under a CLOSED NSIS
COMMAND GRAMMAR (stdlib only; every rule fail-closed) and pins the
installer attributes (Name, OutFile `bedlam-shell-setup.exe`,
Unicode, `$PROGRAMFILES64\Bedlam` + admin + `CRCCheck force`, the
minimal page flow, exactly the install + `un.` sections), the
closed engine-only File set (two staged bare names — the binary +
its README), the instruction-for-instruction install body (the
uninstaller, the Add/Remove-Programs registration, one Start-Menu
shortcut whose working directory is `$INSTDIR` — NSIS stores
`$OUTDIR` as the shortcut's working directory, and `SetOutPath`
runs first, so the engine's documented default lookup root sits
directly inside the install folder), the exact-inverse uninstall
(every Delete names an installed artifact; `RMDir` never
recurses — the switch cannot even parse), the README contract
(engine-only boundary, supply-your-own, the documented default
layout `game-data\BEDLAM` as the only corpus token it may carry),
and the CI build join (the `windows-installer` job on
windows-latest: `cargo build --release --locked -p bedlam-shell`,
`choco install nsis`, the staging `Copy-Item`, makensis run with
`working-directory: packaging` on THIS script, the strict
`if-no-files-found: error` upload with bounded retention). The
registry row `windows-installer` flipped `landed` in the same
commit (R2). Authenticode stays the `signing-keys` exclusion: the
installer is UNSIGNED by design and the denylist is enforced
across script + README + job, comments included.

## 7. P7 acceptance surface (pointer, not re-statement)

The full P7 phase definition (Linux native + Flatpak; Windows
installer; macOS universal2 through automated CI; the
external-conditions posture; CI artifacts per push; the CDDA
contract; the SteamDeck default) is PLAN §6 (P7) — this doc does not
restate it beyond the §1 verbatim quote it operationalizes. Every P7
unit cites the plan sentence it implements; divergences from the plan
are DECISIONS.md entries, never silent.
