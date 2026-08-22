# NEXT - task queue (top first; rewrite this file at end of every run)

## Now
1. [BLOCKED] [P4.2/DH-G0-live] S0 LIVE SESSION —
   INTERACTIVE-ONLY REMAINDER (operator/desktop; ALL machinery landed +
   headless-verified by the fa49e9cf unattended prep, commits f659db5 +
   d5550a3 + ee2f0d4, D81 — follow docs/RUNTIME.md "S0 LIVE SESSION
   CHECKLIST" step by step).
   BLOCK REASON (recorded once 2026-08-22 by unattended worker
   e63e5ff4, claim 1): steps (b)-(e) are operator-desktop-gated —
   FORCE_DIFF_RUN=1 `diff capture` opens the live game window where a
   human must walk the title menu to ZONEA/MISSION1 (twice, fresh
   boots) and the cycles calibration needs ears on live audio
   dropouts; docs/RUNTIME.md "Explicitly NOT done here" item 2 bars
   unattended runs from launching desktop game sessions (the
   refused-unattended gate exists for exactly this). Unattended-safe
   step (a) was RE-VERIFIED GREEN by that worker this run: dbgprobe
   gate + dbgprobe flow both pass, staged
   runtime/harness-out/diff/S0/capture-plan.json is byte-identical to
   the committed capture-plans/S0.json, staged corpus + run.conf
   intact, MANIFEST.sha256 clean before AND after. The operator
   session is TURNKEY — start at checklist step 1, no prep left.
   OPERATOR STEPS: (b) FORCE_DIFF_RUN=1
   `diff capture` — walk the title menu to ZONEA/MISSION1; capgen does
   the boot trap → flat-CS guard (SELINFO base==0, loader-stub stops
   retry) → BP CS:0005A6EB arm (the ack echoes the selector = the
   per-run pin) → resolve w/h + TOT/DAT/claim pointers → 3-record
   capture (the INT3-at-entry proof step is SUPERSEDED — CS-register
   addressing needs no selector); (c) `diff stitch` × 2 runs;
   (d) DH-G1 verdict = identical chains MODULO the frame-counter/RNG
   blob bytes (no counter reset exists — 14 INC sites incl. menus;
   T2/T3 classes per DESIGN §6; any OTHER byte diff = a channel
   finding, record + stop); (e) cycles=fixed 60000 listen calibration
   (audio-live plan env variant). Record fingerprints (chain/dump
   sha256, selector pin, w/h, pointers) in RUNTIME.md; DECISIONS if a
   pin changed. NOTE the 6 deferred TS rows (cgr/bin/min/lnk/
   order-table/yline — extent formulas unpinned) are consciously OUT
   of the first golden; adding them later is additive (re-baseline
   chains deliberately). Manifest checks bracket corpus-touching steps.
2. [P4.2/W5-followup] THE EXD INPUT-TWIN CENSUS (unattended, RE
   unit): pin the EXD keystore alias (EXW g_keystore 0x4edc44, 256 B
   scan-indexed) + the order-target triple + the command-ring/count
   twins + difficulty, and fill the registry exd_addr gaps so the W5
   injection steps compile for O1 (D82 machinery is landed + proven;
   ONLY the aliases are missing). ENTRY POINTS: the any-key scan
   family twin (EXW FUN_0041f9d1 scans 1..0xFE), the InputReset
   memset-256 twin (EXW 0x4207b5), the MissionShell consumer-call
   twin (@EXW 0x448030 → the EXD command ring), the (d+1)%3
   difficulty site. KNOWN DEAD END: FUN_0002ec12 = only the P-latch
   spin (ghidra-project/exd-input-probe.txt, EXDInputProbe.java).
   Fill watches.toml rows (exd_status verified) + RE-EXD-MAP sec 4/5
   — dbx-plan compiles them automatically. AFTER the keystore alias:
   the scripted-menu-walk driver (BPLM-on-frame-counter walk stops +
   mission-start detect) becomes its own unit.

## Backlog (not yet started)
- [P4.2/W6] ENGINE DUMP EMITTER: parity_harness --canonical (per-tick
  canonical records in the W3 schema, T0/T1 field maps first) — the E
  side of the differ; consumes the v1.1 grammar steps directly
  (shared seam, D82).
- [P4.2/W7] THE DIFFER: normalizer + DESIGN §6 comparison modes +
  report writer + fingerprint manifest.
- CLOSED by 7j.27: the DROPSHIP ring producers (writer census,
   animator map, 7×5 grid correction, latch census, the 0x4c71f4
   pass head). CLOSED by 7j.26: the [0x4ede24]/[0x4ede28] "7×7 screen-address
  table" question — it is the terrain RESTAMP list (count + 3-dword
  {dest row, tile-x, tile-y} records, blitted via FUN_00401471;
  writer FUN_00440a2d = the scroll/camera restamp stager, confirming
  the hypothesis). REMAINS open slim: FUN_00440dc2's own identity
  (reads the backbuffer [0x4ede18] @0x440e02; the 7j.16
  TOT-materializer caller). CLOSED by 7j.17: the [0x4edd60] height-bank family and the
  projectile z-encoding census. CLOSED by 7j.24: the critter
  death-handler family. CLOSED by 7j.25: the destroy-tail
  effect-entry map + the 160-vs-0xA8 stride anomaly + the
  .POS/.BDG loaders + the .BDG grammar (FORMATS §12/§16).
  OPEN small: projectile type 0x69 vs the FUN_00419aff
  damage table (7j.17/7j.18 — low priority).
- The per-zone FUN_00433980 case table (≈28 pad ids × 7 zones,
  beyond the §7j.19 head decode; §7j.20 item 2 gives the ~25
  extraction-pad (zone,slot) pairs and §7j.21 the record
  high-water marks + the record↔pad arm mapping task) + the
  FUN_00424a6f message string table — mechanical, decode per
  zone only when P4.2 needs it.
- The 0x4787c4/0x47879c hot-rect record (renderer FUN_00403938
  writes it @0x403c93, count [0x46ccd8]; picker reads
  center@+8/+0xC + w@+0x14, order dispatcher reads corner@+0/+4 +
  z@+0x10 + type@+0x1C — [hypothesis] one 0x20-stride record with
  both views). Anchors the click-target rect semantics.
- The .MOFO loader (the last of the FUN_00416458 sibling
  loaders @0x457a4c; .NME/.TRT/.POS/.BDG all CLOSED —
  7j.15/7j.18/7j.25) + the .BLD record walk (names/graphics
  side; FORMATS §17 — the 201-B/64-B-extension hypothesis
  still unanchored) + the .BDG template-bank plane↔mirror-word
  mapping (which bank feeds which restore word — 7j.25 pinned
  banks @+0x46/+0x4A = TOT-mirror/seen+DAT; @+0x3E/+0x42
  readers still open).
- The debris-stager ENGINE widening beyond kind 5 (fed by the
  7j.11 20-kind table + the 11 seq tables): model the k2/k8
  single-center scorch (values 3/4), the k1/k20 shared-tail
  ring, and the +0x20 physics classes (0/1/2/3/6 ->
  FUN_0040de9c) — all producers now DECODED (7j.22 weapon
  family, 7j.24 critter deaths, 7j.25 destroy tail) but all
  sit OFF the corpus path (nothing fires/dies/gets destroyed
  in the gates); lands with the P4.2 harness.
- Keyboard latch wiring for the sidebar (F1/F2/F3, keys 1..7,
  MSpace; RE-EXW-INPUT line 95) - blocked on the P2e InputFrame
  button bit-map assignment.
- Title-menu polish backlog (all optional, none block P4): pin the
  menu BACKDROP content (RE-EXW-TITLEMENU sec 8 - the 0x64000
  PresentCopy buffer), HOF + CREDIT_1..13 page flows (RE sec 6),
  the save-load restore path (FUN_0044745e + completion bits),
  CONFIG.BDL writer family (FUN_0042540c) for name persistence,
  OPTIONS.MRS staging on Title (music track_name wiring), and the
  FUN_00448ef1 multiplayer lobby if ever needed.
- Mission SFX tier (RE-EXW-SIM sec 9 open item 5; MENU1/MENU2-style
  mixer instruments exist) + the order SFX 0x2A armer click + the
  damage/alarm SFX families (7g.1) + the pickup SFX 0x43a48e
  entries (7h.2) + the select-ack SFX pair 0xC+k/0xF (7j.6) + the
  debris arrival-SFX pair FUN_00421e60/FUN_00421dec (7j.11 item 4).
  NOTE 7j.17 pinned new FUN_0043a48e banks: _DAT_004edf94/
  _DAT_004edfe4/_DAT_004edfac (robot fire) and
  _DAT_004edffc/_DAT_004edff0/_DAT_004edfa8 (critters/POI).
  NOTE 7j.20: the beacon armer's SFX is FUN_004239ef(0x2a,3).
  NOTE 7j.25: the destroy-thud pair 0x4edfb8/0x4edfbc =
  DEADMAN1/DEADMAN2.RAW (loader 0x43a29b..0x43a368 strings —
  a full bank-name walk is a bounded SFX-unit add-on).
- The pickup tile-word PRODUCER (7h.3: the 0x4796bc type-DB
  mirror rows + the probe-latch walk + the DAT z-plane consume +
  the 0x454a90 floor-word swap) — unblocks the apply_pickup
  dispatch from host-seamed to corpus-real; needs the
  MISSIONVIEW sec 8 mirror producers first.
- Camera scroll input for the mission (cursor+drag, RE-EXW-INPUT).
- RE-EXW-MISSIONVIEW sec 8 open items: type-DB tail producers
  (+0x1a/+0x1b/+0x1c — NOTE +0x18 is CLOSED as the runtime scorch
  writer FUN_00422287 per 7j.8/7j.9, reader verified raw), the
  u32[0x456ca8] anim sequence + the water flag producer (needed
  before the 0x12d/0x12e/0x12f flush remaps can leave water-off
  semantics), BIN u32[bank+0] header word (NOTE 7j.16: the ".BIN"
  load is pinned — header word -> 0x46cdb8; the [0x4ede1c] bank's
  CONTENT consumers still open). CLOSED: u32[0x4dd444]
  (7e.4 - the PALTRAN ramps); +0x18 producer (7j.8/7j.9 -
  FUN_00422287, reader raw, ring landed D57).
- MISSIONVIEW sec 5d tail notes: ROBNUMS name plates,
  Shield/Variant bank staging (nodes enqueue, flush skips while
  unstaged). The debris physics/collision FUN_0040de9c (7j.7
  head decode) lives here too (+ the 0x454510+ physics-param
  dword table census-noted in 7j.11 item 5; 3 octile reads per
  7j.16; reads BOTH the critter and POI counts — collision
  family).
- RE-EXW-SIM sec 9 open items 2-3: FUN_00440e45 identity (THE SHOP
  per 7d: WEAPICON/CONLITE/SHOPFONT/SHOPLITE + SHOP.SMK + the
  weapon-table writer family - see 7d.2; 1 octile read per 7j.16;
  NOTE 7j.17: it also reads the command count 0x46cbe0 — MP shop
  sync), robots() extra-phase semantics + state-1 producers.
- P4.2 differential harness (budgeted ~2 weeks, PLAN sec 6 P4.2):
  DESIGN DOC LANDED 2026-08-22 (docs/DESIGN-DIFFHARNESS.md, D77, commit
  7bc2c9d) — oracle topology (O1 EXD/DOSBox-X primary instrument, O2
  EXW/Wine canon tiebreak, O3 8street second comparator), tiered watch
  set (every address ledger-anchored), seam injection (COMMAND records/
  orders/.PAD step-on — never raw input), canonical-record differ, gates
  DH-G0..G3, build order W1..W12 (W1 = EXD import + address map, now the
  head item). The doc also arbitrates the two 7j hypotheses (the debris 2k
  start delay and the blink-cursor-from-spawn question) + the 7j.9 overlap
  last-write-wins read of the five rings. NOTE 7j.20: the harness
  must model the mission-start pod-descent stagger (w@+0x2C =
  1+k·(2000−m·1000/27)) — the first seconds of any mission have
  the robots frozen in pods (7j.27: descent ≈41 frames, pod phase
  2 = one tick, release = state 6) — and arm extraction via a scripted
  .PAD step-on, not a click. NOTE 7j.22: weapon fire needs
  injected COMMAND records (FUN_00449c94/0x4dd4a0) or order
  dispatch, not raw input — the fire family is fully anchored
  for it (per-type cadences + damage tables). NOTE 7j.25: the
  destroy family is now fully decoded end-to-end (resolver →
  restore → 5-effect loop → chain walks), ready for the harness.
- TOT semantics follow-up: FORMATS sec 2 plane 6/7 (the ~2000-slot
  POS linkage) — KNOWN-staged (word mirror at record words 6/7)
  but the drawer treats them as ordinary stack levels - check
  whether plane 6/7 words ever draw on shipped maps (ZONEA tile
  642 is the only cell) before touching FORMATS. NOTE 7j.16: the
  .TOT volume->mirror materializer FUN_00440a2d copies ALL 8
  planes' nonzero words — the plane semantics now have their
  runtime reader; re-check the mirror-word consumers (0x4796bc)
  for plane-specific behavior.
- OPERATOR NOTE (carried): MANIFEST-2.sha256 at the repo root mismatches
  470 files - it documents a different tree snapshot (its BEDLAM.LOG
  entry is the sha256 of an EMPTY file). Re-anchor or delete it. It was
  never used as the integrity gate: MANIFEST.sha256 is the canonical
  AGENTS-named manifest and verifies clean.

## Done (append concise entries only)
- 2026-08-22: P4.2/W5 THE INJECTOR unit COMPLETE (worker 683a65d6
  claim 2, commits c443207 + fa31828 + 5e882cd + 28ef5e7, D82).
  (a) grammar v1.1: keystore/order/pad/command/boot steps, one frame
  boundary per line; until-anchor splits walk/mission phases; boot is
  walk-phase only; command payloads raw hex <=0x80. (b) capgen SMV
  emitter: boot_writes at the arm stop + frame-keyed inject rows
  applied BEFORE the watch dumps (`frame N 1` injected flags in
  DBXCAP = injection_applied in the W3 dump); the command-ring append
  OP (count u32 read via the plan's own SEG:OFF form, payload
  zero-extended to the stride at base+count*stride, count bump);
  addr_to_linear: CS: flat identity (bounded to image top), numeric
  segs seg<<4; byte-only tokens. `dbgprobe inject` GREEN headless
  (boot beefcafe at 0000:0500 in the anchor frame, marker re-writes
  read back same-frame, command append count 0->1 + zero-extended
  0x10-stride record, all frames flagged; gate + flow
  regression-green). (c) dbx-plan T1: Form::CountExpr + count-cell
  resolve rows (robot 0x11958c / TRT 0x11949c / object *(0x119584)
  count 0x119554) + map-w/h grid exprs; S1.scen compiles ->
  capture-plans/S1.json committed + byte-pinned; S0.json verified
  byte-identical; anti-fabrication: selection-triple dumps only the 4
  verified alias bytes, beacon-family its 5 u16 cells (10 B).
  (d) step compiler: boot_writes/inject plan rows with HARD alias
  gates (keystore/order-target/command-ring/difficulty are registry
  gaps -> scenarios carrying them fail naming the seam; emission paths
  proven against a fabricated-address registry in tests only); two TI
  registry rows formalize the command seam (0x4dd4a0/0x46cbe0). RE
  FACT: FUN_0002ec12 = the P-latch spin only, NOT the EXD keystore
  (exd-input-probe.txt; RE-EXD-MAP W5 note) — the input-twin census is
  the queued follow-up. 49 diffharness tests; workspace test/fmt/
  clippy green; manifest clean. PUSHED 28ef5e7. Queued: item 2 =
  the EXD input-twin census.
- 2026-08-22: P4.2/DH-G0-live the UNATTENDED PREP unit COMPLETE (worker
  fa49e9cf claim 1, commits f659db5 + d5550a3 + ee2f0d4, D81). The S0
  live-capture machinery landed + headless-verified; the queue's
  interactive item is now a turnkey checklist (RUNTIME.md "S0 LIVE
  SESSION CHECKLIST"). RE FACTS (source-pinned at e522642, RUNTIME.md
  "S0 live channel mechanics"): GetHexValue resolves REGISTER NAMES in
  the default MEMDUMPBIN/BP parse path → `CS:001195F0` addressing with
  NO numeric selector (the BP ack echoes it — the per-run pin; the
  INT3-at-entry proof step is SUPERSEDED); BP locations resolve
  EAGERLY at arm time (pre-boot arming mis-resolves) vs BPLM LAZY
  per-instruction (the boot trap on the frame-counter cell 0x1195f0);
  SELINFO rides the logfile (the flat-CS guard, loader-stub stops
  retry); debuggerrun=watch would free-run (staged conf flips to
  debugger — channel-mode only, sim pins untouched); NO counter reset
  exists (14 INC sites incl. menu screens) → the live DH-G1 verdict is
  identical-chains-MODULO frame-counter/RNG cells (T2/T3, DESIGN §6);
  SDL dummy video has no keyboard (live plans unset SDL_VIDEODRIVER).
  CODE: capgen plan v2 (boot_commands/boot-trap retry loop/arm_commands
  + selector-pin capture/resolve cells + ast-whitelisted arithmetic
  addr-len exprs/anchor-vs-per-frame watch split/plan env overrides/
  plan-level frames+time_limit; fixed the plan-named logfile being
  resolved against the wrong CWD) — `dbgprobe flow` gate GREEN headless
  (BPLM 46C trap → arm → resolve com1=0x3f8 → expr rows carrying REAL
  IVT f000:ca60 + BDA COM1/COM2/LPT1 bytes; legacy `dbgprobe gate`
  regression-green; both unattended-safe, no game). dbx-plan
  (tools/diffharness Rust bin): compiles scenario tiers + watches.toml
  into the plan with EVERY address derived from registry rows
  (anti-ghost asserts on extents/cell layouts; committed artifact
  capture-plans/S0.json pinned by a byte-equality test); 9 T0 + 9 TS
  rows resolved (TOT/DAT extents = 4+16wh / 4+8wh, cross-check: ZONEA/
  M1 evaluates to 30004/15004 = the FORMATS-pinned file sizes; map-wh
  two-cell span; claim-bank pointer read), 6 TS rows DEFERRED with
  explicit unpinned-extent reasons (cgr/bin/min bank-sized, lnk
  map-sized, order row count, yline tables) + the 2 T0 gaps; S0 staged
  + plan deployed. 42 diffharness tests, fmt+clippy clean, workspace
  build green; manifest verified around the corpus rsync. PUSHED
  ee2f0d4. Queued: the interactive-only live session (item 1) + W5.
- 2026-08-22: P4.2/DH-G0 the O1 CAPTURE-CHANNEL RE-PIN unit COMPLETE
  (worker 4deb0081 claim 1, commits 395180b + d858728 + 1e7392f, D80).
  DECISION (a): self-built DOSBox-X at e522642 with --enable-debug=heavy
  (+ --disable-sdlnet --disable-avcodec: host gaps, harness-irrelevant),
  repo-local under runtime/, C_DEBUG+C_HEAVY_DEBUG verified in config.h,
  binary sha256 24f71092… recorded; flathub pin stays as the D29 sandbox
  baseline; GameLink + O2-ptrace rejected as primary (D80 rationale).
  CHANNEL PROVEN HEADLESS (no game): tools/runtime/dbx-capgen.py — the
  PTY driver with count-based [log]-logfile acks, a mandatory drain
  thread, and a 1.0s post-ack settle (the three empirically-bisected
  gotchas = RUNTIME.md "D80 CHANNEL GOTCHAS") — wired as
  dosbox-harness.sh `dbgprobe` (unattended-safe; GREEN: 3 frames × 3
  watches; frame1 pre-boot zeros → frame2 POST IVT f000:ca60 + BDA
  COM1/COM2/LPT1 → frame3 DOS-kernel vectors 0070:000e) and `diff
  capture` (FORCE_DIFF_RUN=1 interactive gate; consumes the staged
  capture-plan.json = the live unit's deliverable). BPINT/BPLM/
  MEMDUMPBIN/RUNWATCH/SMV all behaviorally verified (SMV linear
  write+readback pins real-mode linear==seg<<4; BPLM arm+fire on the
  next write). RUNTIME.md D80 sections + DECISIONS D80 + watch skeleton
  updated; live-unit checklist = pmode flat-selector proof (INT3@entry
  0x5fbb0 → SELINFO/LDT → present-tail BP@0x5a6eb) + DH-G1 double-run
  + cycles calibration. Manifest verified. PUSHED 1e7392f. Queued:
  DH-G0-live (interactive-gated) + W5 injector (unattended).
- 2026-08-22: P4.2/W4 the DOSBOX-X RUNNER unit COMPLETE via the ticket's
  split clause (worker d35c7066 claim 1, commits d9a3f77 + 19c3bdf, D79).
  (a) unattended-safe slice LANDED: dosbox-harness.sh `diff
  stage|run|stitch` (EXD corpus scratch runtime/harness-corpus-exd from
  game-data/BEDLAM — launch line pinned DOS4GW.EXE BEDLAM.EXE[launcher]/
  BEDLAM.EXD[LE image] via header+launcher-string evidence; per-scenario
  conf deploy; run=refused-unattended gate); scenario grammar v1 + S0/S1
  scenario files; DBXCAP v1 channel-agnostic capture transcript + the
  zero-dep dbx-stitch bin (registry/tier/O1-exd_addr anti-ghost guards,
  frame-count contract, W3 encode + JSON digest manifest with
  self-contained SHA-256, FIPS-vector-pinned); synthetic replay fixture
  test decodes the dump + pins chain vector 1685e11311ae5b21; fmt+clippy
  green workspace-wide; MANIFEST verified both sides of the corpus read;
  staged fingerprints cross-checked vs sha256sum. (b) live piece
  [BLOCKED]-on-DH-G0-channel-repin: the D79 AUDIT found the pinned
  flathub DOSBox-X has NO integrated debugger (configure.ac --enable-
  debug off; flathub builds --enable-sdl2 only; debuggerrun/-break-start
  inert across piped/PTY probes) and its Duktape startup.js is LOG-ONLY
  (enumerated: _emu.emulator/version/log + console.log -> [log] misc
  channel, needs misc=true; Buffer/CBOR with no I/O; no memory access,
  no hooks) — D29's debugger-presence claim corrected in RUNTIME.md;
  DESIGN §3/§9/§10-W4/§11 + the watch skeleton amended. Next head: the
  channel re-pin unit (queue item 1).
- 2026-08-22: P4.2/W3 the DUMP SCHEMA unit COMPLETE (worker 6f14cea1
  claim 1, commit fca6657). tools/diffharness/src/dump.rs = the DESIGN
  §3 format as code, schema_ver 1, all-LE: "BDLD" header {channel 1..4
  (O1/O2/O3/E), build_sha256[32], scenario, pins} → frames {frame_no,
  injection_applied, per-watch {id, len u32, raw}, frame_digest} →
  "BDLT" trailer {frame_count, chain}. frame_digest = FNV-1a-64 over
  the BDLD-tag-prefixed canonical frame bytes (domain separation vs
  StateHash); chain = the D28 parity_harness construction verbatim
  (incremental Fnv1a64 write_u64 per frame digest) so dump chains are
  directly comparable fingerprints. Encoders registry-driven: canonical
  watch order = watches.toml file order, unknown/duplicate ids
  rejected, frame_no strictly increasing (encode+decode), empty blobs
  legal (count-0 banks); identical state ⇒ identical digests on every
  channel (tested). decode_dump verifies every digest + count + chain +
  truncation/trailing/magic/bool/utf8 (tamper tests cover payload,
  chain, count, truncation, trailing, magic, schema, channel).
  hash.rs = zero-dep MIRROR of bedlam-core's FNV-1a-64 (dependency
  would pull thiserror), pinned to the engine's public vectors by
  tests/dump_schema.rs::engine_hash_vectors. 15 integration + 3
  in-module tests; workspace build/test/fmt/clippy green (release
  build clean). Docs: DESIGN §3 + §10-W3 LANDED, DECISIONS D78.
  PUSHED fca6657. Queued: W4 (DOSBox-X runner diff mode + DH-G0
  debugger-surface pin).
- 2026-08-22: P4.2/W2 the WATCH REGISTRY unit COMPLETE (worker 873ebd5e
  claim 1, commit 01a6847). tools/diffharness/watches.toml = the DESIGN
  §4 watch set as data, 73 rows: S0 trigger (EXW PresentEnd 0x425a03 /
  EXD instruction 0x5a6eb), T0 (11 rows, EXD aliases filled: frame
  counter 0x1195f0, RNG A/B 0x107470/74, score 0x10da28, money 0x119600,
  zone 0x107500, mission 0x119610, mode 0x1075d8, linear-m 0x119610),
  T1 (17 rows: robot bank 0xf6d34/count 0x11958c, selection triple
  0x11954c selected-idx only, per-player anchor 0x971a4, move-target
  0xf75ec/0xf761c, beacon family 0x119628-30, claims 0x119632, tile grid
  0xfe37c, platform bank 0xf93cc, type-DB mirror 0xac1e4 + derived
  +0x18/+0x19/+0x1A rows, object bank *(0x119584) indirect + count
  0x119554, TRT 0x95264 + count 0x11949c, armor-pad alias), TS (all 15
  static-after-load §5b rows incl. the volume pointer cells
  0x107454/0x107518/0x107540/0x107434/0x107538 indirect, PAD 0xf63c,
  order table 0x91ee4, player type 0x1075c0, dither 0x8ded4, EXD-only
  cursor clamp), T2/T3/T4 (exd-empty per the W1 ticket) + TI (the six
  injection-surface rows anchored on RE-EXW-INPUT). The 6 tagged gaps
  (difficulty, SFX gate, blink-cursor, order target, no-extract latch,
  selection cursor/squad) stay explicitly exd-empty. NEW zero-dep
  workspace member tools/diffharness: minimal TOML-subset registry
  parser + tests/registry_anchors.rs = the mechanical anti-ghost guard
  (every anchor string must resolve EXACTLY to a ledger row heading /
  markdown heading in its named doc; guard verified to bite on a
  fabricated anchor) + schema invariants (tier set, exd_status vs
  exd_addr consistency, T2-T4/TI exd-emptiness, indirect-pointer rules,
  gap discipline). cargo test/fmt/clippy green; manifest verified.
  PUSHED 01a6847. Queued: W3 (dump schema).
- 2026-08-22: P4.2/W1 the EXD IMPORT + EXW->EXD ADDRESS MAP unit COMPLETE
  (worker d06341cf claim 1, commits 350b53a + 10aea57 + f6e067a + 8447ba7,
  docs + 8 Ghidra probe scripts). BEDLAM.EXD imported ONCE into
  BedlamWatcom (LeLoader, x86:LE:32:default + openwatcomcpp; object1
  0x10000-0x72800 23225 fixups, object2 0x80000-0x12583e, entry 0x5fbb0;
  analysis green; manifest verified before AND after). 8 probe passes
  (-process BEDLAM.EXD -noanalysis, never re-imported; dumps
  ghidra-project/exd-probe*.txt). HEADLINES: MissionShell = FUN_000596ed
  (mission load chain in EXW order; robots x6 FUN_0001c7dc(i,i+1); enemy
  x4; P-pause spin key 0x19); S0 DUMP POINT = instruction 0x5a6eb (CALL
  PresentFlip FUN_00010670, counter [0x1195f0]++ @0x5a6f0-fd after the
  flip — EXW tail order exact); PresentFlip = FUN_00010670 (339 B = B2
  twin exact). T0/T1 + static-after-load aliases ALL mapped in
  docs/RE-EXD-MAP.md with dual anchors: RNG A/B 0x107470/0x107474, score
  0x10da28, money 0x119600, zone 0x107500, mission+linear-m 0x119610
  (TRT-hp + pod-stagger formulas byte-exact), mode 0x1075d8, frame
  counter 0x1195f0; robot bank 0xf6d34/count 0x11958c (stagger w@+0x2C
  formula exact), selected idx 0x11954c, per-player anchor 0x971a4,
  move-target arrays 0xf75ec/0xf761c, beacon family 0x119628-0x119630,
  claims 0x119632, tile grid 0xfe37c, platform bank 0xf93cc, type-DB
  mirror 0xac1e4, object bank *(0x119584) + count 0x119554, TRT array
  0x95264 + count 0x11949c, type table 0x108428 (stride 0x4E/282
  recs/banks +0x3E-4A — EXW layout byte-exact), all volume pointer
  cells + PAD 0xf63c + LNK 0x10336c + map w/h 0x1074b8/0x10748c, order
  table 0x91ee4, player type 0x1075c0, dither 0x8ded4. 5 divergence
  seeds logged (robot-front x/y shifted -4; EXD's 3 merged monoliths;
  single mission scalar for EXW's two; indirect pointer-cell banks;
  /KARMA switch). 6 explicit gaps tagged with anchor methods
  (difficulty, SFX gate, blink-cursor, order target, no-extract latch,
  selection cursor/squad). PUSHED 8447ba7. Queued: W2 (watch registry).
- 2026-08-22: P4.2 the DIFFERENTIAL-HARNESS DESIGN DOC unit COMPLETE
  (worker 4d7b9a5b claim 1, commit 7bc2c9d, D77, docs-only; no engine
  change, no Ghidra run needed). docs/DESIGN-DIFFHARNESS.md written:
  oracle topology decided (O1 = BEDLAM.EXD under pinned DOSBox-X as the
  PRIMARY scripted-differential instrument — observation never patches
  the binaries; O2 = EXW under pinned Wine as canon tiebreak with every
  RE'd address verbatim via a ptrace watcher; O3 = instrumented 8street
  as late second comparator; E = the engine). Frame model: one dump per
  MissionShell loop pass at the epilogue/present tail, aligned by
  g_frame_count@0x46ae68 <-> engine tick. The tiered watch set T0-T4
  with EVERY address anchored to its RE-EXW-SIM §8 / §7j.x ledger row
  (robots/orders/terrain T1; projectiles/critters T2; effects/debris/
  rings/objectives T3; SFX/order/debris/destroy event capture T4).
  Injection = seam writes only (g_keystore 0x4edc44/cursor/mouse,
  ORDER target 0x4dd484/88/8c + 0x46cc30/60 words, COMMAND records
  0x4dd4a0 for fire — the 7j.22 route, .PAD step-on for extraction).
  Differ compares canonical records in 5 modes per the 0b budget;
  divergence classes engine-bug/original-divergence(O2-arbitrated)/
  watch-artifact/accepted-T3. Scenario corpus S0-S8; hypothesis
  dispositions tabulated (pod stagger S1, debris 2k start-delay S1/S4,
  blink-cursor-from-spawn S1 via 0x4dc5d0, ring overlap statically
  MOOT per 7j.10 + confirming read S4, mid-flight blits = T2
  render-side OUT of state-diff scope). Dumps = asset-derived: live
  only under runtime/harness-out; git carries fingerprints only.
  Gates DH-G0..G3; build order W1-W12. DECISIONS.md D77 added.
  Manifest verified. PUSHED 7bc2c9d. Queued: W1 (EXD import +
  EXW->EXD address map, T0/T1 rows bounded).
- 2026-08-22: P4 7j.28 the PROJECTILE MID-FLIGHT DRAW family unit
  COMPLETE (worker ffec42cf claim 1, commits 9a1d205 + 27481c2,
  D76, docs-only; objdump-only from ghidra-project/
  exw-text-objdump.txt — an analyzeHeadless was running). The
  400×0x36 dispatch fully mapped (primary 0x404141 + secondaries
  0x404d27/0x404d08): shell 5 (WEAPONS 3..7, counter d@+0xE wraps
  7→3), artillery 9..0xB (8..15), mortar 0xE (frame 1 + the
  8-puff trail), damped {0xF/0x13 base 0x20, 0x17 base 0x28,
  0x1A/0x1F base 0x18} + wobble gate |vx|∨|vy|>0x40, rocket 0x24
  (SHRIKE 64-dir + ≤8 SMOKE puffs dist 0x20+0x10·i, count TTL/4),
  homing 0x29 (REAPER 64-dir + GENERAL reticle on target d@+6
  {0x1000 robot/0x2000 critter/else FUN_004128ec} + 4 puffs).
  BANKS NAMED + corpus-verified: WEAPONS/SHRIKE 64/REAPER 64/
  SMOKE 4/GENERAL 153 imgs (= [0x4eddbc]/[0x46af30]/[0x46af2c]/
  [0x46af34]/[0x4edd7c], boot string block 0x45884e..). The
  trail-ring draw consumer @0x404464 CLOSED (puffs @ 0x4e66b8+
  link·0x68+8+i·0xC, WEAPONS 0x10+(tick+i)&7, mode 0x12E, ring
  words unread). The 50×0x22 walk CLOSED (jump table 0x403908
  read from file: 0x65/0x67/0x68 single strip sprites 0x3C/0x3C/
  0x38, 0x66 NOT drawn, 0x69 the per-level BEAM column 0x34-strip
  with +0xA = top z level, +0x1A = bottom). CORRECTIONS: 0x40427a
  = loop-next (unlisted types NOT drawn mid-flight — no "generic
  draw body"); 0x17 draws damped (the 3-clone split is tick-side).
  FUN_0040798e call shape pinned (mode 0x12C/0x12D/0x12E = the
  4th stack arg; the 7j.21 "sprite 0x12E" gloss corrected).
  Render tail now FULLY decoded. Manifest verified. PUSHED
  27481c2. Queued: the P4.2 differential-harness design doc.
- 2026-08-22: P4 7j.27 the DROPSHIP RING PRODUCERS unit COMPLETE
  (worker e635cb76 claim 1, commit 2aa7cb7, D75, docs-only; dump
  ghidra-project/exw-text-objdump.txt = full .text objdump
  0x401000..0x460000, no Ghidra run — one was already running).
  The pod-descent family writer census COMPLETE: resets
  FUN_0040cca0 0x40cd3d (pods memset 0x150 every spawn) +
  MissionShell 0x447a7e/0x447a8d (dropship/exits); spawners
  FUN_0041faf0 (dropship {1,1,group 0,alt 0x200,beacon<<5}),
  FUN_0041fb4b(idx) (pods {1,1,group 0,alt 0x400,robot>>8}, from
  the w@+0x2C 0-hit in FUN_0040b9f6 + msgs 9/10/0xB), 7j.18's
  FUN_0041fa51 (exits); animator FUN_0041fbb1 3-machine per-tick
  write map decoded — +0x14 = the DROPSHIP.BIN IMG-GROUP selector
  (7j.19 "toggle" superseded): 0↔1 flicker phases 1-2, ramps
  2..5 oscillating 4↔5 in departure with x −= group·4, alt +=
  (alt>>2)+1; pod phase 2 = ONE tick = robot RELEASE (state 6,
  alive 1, payout 100·w@+0x94+5000, SFX 0x4edfe0). NEW third
  writer FUN_00412a98 0x412b60 = per-rescue exit-dwell reset
  (multi-POI elevators). Latch 0x46aed4: boot-clear GameMain
  0x41c408 (NOT per-mission) + gates the MP respawn 0x40e7a1.
  CORRECTION 7j.26: ring grid = 7 cols × 5 rows (0x23 = 35 = one
  group), not 7×7; dropship sy −= beacon z word 0x4eabb8 (always
  0, one no-op reader 0x4070c0). The 0x4c71f4 pass head-decoded =
  projectile mid-flight draw dispatch + the 0x4cc654 50×0x22
  sibling (states 0x65..0x69 → table 0x403908). 4 ledger rows
  updated + MISSIONVIEW §5e corrected. Manifest verified. PUSHED
  2aa7cb7. Queued: the projectile mid-flight draw family (7j.28).
- 2026-08-22: P4 7j.26 the MISSIONVIEW §5d DRAW TAILS unit
  COMPLETE (worker 7658328a claim 1, commits 753f0a2 + 2d124e6
  + d9bb40f, D74, docs-only; dump ghidra-project/
  exw-effectstager-asm.txt (objdump 0x41a220..0x41a4f8)). Both
  consumer passes decoded: (a) the EFFECTS LOOP (0x4cf638,
  80×0x1E) draws DEBRIS.BIN imgs 0..23 (u16@+0x16 group ×8 +
  frame&7, counter u16@+0x1C++ in the draw) via the DIRECT blit
  FUN_00401e39, sy base 0x100 (−0xC vs robots) + the SECOND
  shake table 0x454518, z Q13; 7j.25 field map CORRECTED:
  d@+0x14 = RISING vz 6000..12069 (high word = the sprite
  group), u16@+0x1A = SPAWN DELAY (the producer ECX arg),
  FUN_0041ec59(n) = bounded-uniform RandB()/(0x8000/n−1)
  helper (identity pinned); mover FUN_00419f62 kills at the
  z=12 ceiling/off-map. (b) the PLATFORM LOOP (0x4eb638,
  32×0x14) uses the ENQUEUE path: DAT_0046af54 = SMOKER.BIN
  (pinned) frame 0 mode 300 + smoke column frame d@+0x10+1
  mode 0x12d (DARKPAL) at sy−0x20; tick FUN_004238af cycles
  2..16 intro/5..16 loop. FUN_00401e39 CODEC DECODED + the
  .BIN container CORPUS-VERIFIED (u16 count word0, u32 dir at
  bank+2+4·img, offsets rel. own slot; 24/24 DEBRIS + 160/160
  DANTE exact-consumption; DEBRIS 24/SMOKER 17/DROPSHIP 210
  imgs — MISSIONVIEW open item 4 RESOLVED, FORMATS §18
  cross-ref). BONUS: the three DROPSHIP ring passes recorded
  (producers → 7j.27) + the [0x4ede24/28] backlog re-pinned
  as the terrain RESTAMP list. 7 new + 2 rewritten ledger
  rows. Manifest verified. PUSHED d9bb40f.
  Queued: the DROPSHIP ring producers (7j.27).
- 2026-08-21: P4 7j.25 the WEAPON-FIRE FAMILY TAIL unit COMPLETE
  (worker 399aeff4 claim 1, commits 3bfd400 + 1016123 + b4950a8
  + 6183be5, D73, docs-only; dump ghidra-project/
  exw-destroytail-asm.txt + full-objdump census). The
  FUN_0041a894 destroy tail decoded WHOLE: TERRAIN RESTORE
  first (footprint W×H×D loop: TOT-mirror z-words ← template
  bank@type+0x46, seen + DAT volume ← bank@type+0x4A, linear
  (z·H+i)·W+j), then the FIVE-EFFECT loop over the type-table
  entries @+0x16+8m — selector word 1..9 → jump table
  0x41a870 (idx sel−1); payload words = tile offsets off the
  0x46cbf4 record; stager stack = (delay, param=score|−1),
  callee ret 8. GER gate REFINED (skips the whole tail for
  type 0xb, record still dies). FUN_0041a225 = FIRST producer
  of the MISSIONVIEW §5d effects bank 0x4cf638. The
  160-vs-0xA8 stride anomaly CLOSED (21·idx·8 = 0xA8 canonical
  — 7j.13 census slip). BONUS: FUN_0041a4f8 = the .POS loader
  (2000×0x10 → the 0x46cbf4 object array) + the .BDG loader
  (the 0x4dedf2 type table) — .BDG grammar CLOSED; FORMATS
  §12/§16/§19 rewritten. 4 new + 2 rewritten ledger rows.
  Manifest verified. PUSHED 6183be5. Queued: 7j.26.
- 2026-08-21: P4 7j.24 the CRITTER DEATH-HANDLER family unit
  COMPLETE (worker 0f986419 claim 1, commit 3819586, D72,
  docs-only; dumps ghidra-project/exw-dead1..5*.txt). The six
  per-kind handlers decoded (k1 FUN_00418835 .. k7
  FUN_0041896c); BOUNTY GATE (killer robot type == [0x4edb90]
  → score += 30/50/500/75/150/1000); SECOND DISPATCHER
  FUN_0040dce0 = debris crush (via physics tick FUN_0040de9c);
  FUN_0041a14f/FUN_0041a494 = the 0x4cec38 effect-row spawner
  + age-LRU allocator; 7j.17 CORRECTED (death handlers never
  call FUN_00424355); FUN_0040e230 SP tail CONFIRMED + MP
  respawn completed; FUN_0042382c = FIRST producer of the
  0x4eb638 platform bank. 8 new + 2 rewritten ledger rows.
  Manifest verified. PUSHED 3819586. Queued: 7j.25.
- 2026-08-21: P4 7j.23 the ACTOR HIT APPLIERS unit COMPLETE
  (worker ad591680 claim 1, commit 45329e9, D71, docs-only;
  dumps ghidra-project/exw-hitters{,2,3,4}*.txt + the NEW
  StoreScan.java operand scanner). FUN_004190bc = the CRITTER
  hit applier (kind switch w@+0x00, damage =
  FUN_00419aff(weapon) per-WEAPON, 6 per-kind death handlers,
  25% knockback FUN_0041a028 + impact SFX FUN_00421fc2);
  FUN_00418fca = robot box-test applier → FUN_0040e230;
  TRAIL ALLOCATOR CLOSED (FUN_00412a4a 20 slots, writer
  FUN_0040a9ff mortar spawner, link/active/ring-zero
  protocol); third critter-applier caller found
  (FUN_00403938 weapon 0xC=5000 blast, owner −1). 7 new + 2
  rewritten ledger rows. Manifest verified. PUSHED 45329e9.
  Queued: 7j.24.
