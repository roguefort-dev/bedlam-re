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
   intact, MANIFEST.sha256 clean before AND after. NOTE D83
   (2026-08-22): S0.json was REGENERATED (difficulty row now dumps;
   19 anchor + 10 per-frame) — re-stage before the session (the
   fa49e9cf byte-identical check predates this; regenerate +
   re-diff `diff stage`). NOTE D84 (2026-08-22): S0.json regenerated
   AGAIN (resolve_at=anchor — resolve now reads the loader statics at
   the ANCHOR stop, mission start, not the arm stop; the S0 checklist
   step-1 text is amended in RUNTIME.md) — re-stage again before the
   session (dbx-plan scenarios/S0.scen --out ...). The operator
   session is TURNKEY — start at checklist step 1, no prep left.
   D84 BONUS HOOK: the same session can calibrate the committed S0W
   scripted walk (`diff capture scenarios/S0W.scen` once — its
   per-stop `# walk stop N walk-mode/zone/mission ...` transcript
   comments map menu transitions to stop indices; then rewrite the
   DRAFT stop counts in S0W.scen — pure data, no code).
   OPERATOR STEPS: (b) FORCE_DIFF_RUN=1
   `diff capture` — walk the title menu to ZONEA/MISSION1; capgen does
   the boot trap → flat-CS guard (SELINFO base==0, loader-stub stops
   retry) → BP CS:0005A6EB arm (the ack echoes the selector = the
   per-run pin) → resolve w/h + TOT/DAT/claim pointers (AT THE ANCHOR
   STOP per D84) → 3-record capture (the INT3-at-entry proof step is
   SUPERSEDED — CS-register addressing needs no selector); (c) `diff
   stitch` × 2 runs; (d) DH-G1 verdict = identical chains MODULO the
   frame-counter/RNG blob bytes (no counter reset exists — 14 INC
   sites incl. menus; T2/T3 classes per DESIGN §6; any OTHER byte
   diff = a channel finding, record + stop); (e) cycles=fixed 60000
   listen calibration (audio-live plan env variant). Record
   fingerprints (chain/dump sha256, selector pin, w/h, pointers) in
   RUNTIME.md; DECISIONS if a pin changed. NOTE the 6 deferred TS rows
   (cgr/bin/min/lnk/order-table/yline — extent formulas unpinned) are
   consciously OUT of the first golden; adding them later is additive
   (re-baseline chains deliberately). Manifest checks bracket
   corpus-touching steps. NOTE (D85 completion): the E-side S0/S1
   counterparts now exist — `parity_harness --canonical --scenario
   tools/diffharness/scenarios/{S0,S1}.scen --out ...` — chains pinned
   in tests/canonical_dump_gate.rs (8901789a88cf61fe / 1c4e7b4c9d9b0947);
   the live session's O1 chains compare against THOSE with the
   LANDED W7 differ (D87, `dbx-diff` -- cross-channel mode handles
   the counter/RNG classes + the coverage findings automatically;
   RUNTIME.md 'W7 the differ'). NOTE D88 (2026-08-22): the differ's
   robot maps now carry the FULL 31-leaf pin (S1 coverage = the 2
   E-only rows + the target trio per robot) + drop_countdown reads
   raw +0x80 (the phase-gate word; the +0x2C pod timer is not
   canonical, E never emits it). Expect the alarm_ctr decay question
   (EXD decrements it per phase-0 pass, EXW 7g.1 documents no decay)
   to surface as the first candidate finding if any damage happens
   in-scenario. NOTE D90 (2026-08-22): the target trio is now
   SOURCED — the live S1 plan carries the move-target 0x60 span and
   the differ splices it into the robot-bank row (S1 coverage = the 2
   E-only rows ONLY, blink-cursor + move-target-words; zero robot
   field gaps). Re-stage the S1 plan for any S1 capture the same way
   as S0 (dbx-plan scenarios/S1.scen --out ...). NOTE D91
   (2026-08-22): S2 now exists for any order→walk live capture —
   re-stage its plan the same way (dbx-plan scenarios/S2.scen --out
   ...) and read the plan's `_e_staging` field first: the live O1
   banks the MRK squad ONLY, so the robot-count diff vs E is the
   recorded scenario seam, never a finding (the original's in-game
   arm needs the click path — the bare 0x10e0a4 triple write does
   not move robots; DESIGN §6a's seam-approximation note stands
   until a live session refines it).
 2. DONE 2026-08-22 (worker ce347a0e claim 2, commit 4210f55, D96,
    §7j.32): [P4/RE] THE .BDG TEMPLATE-BANK READER unit — CLOSED.
    Loader disk order pinned (+0x3E,+0x46,+0x42,+0x4A interleaved
    vs slots); +0x46/+0x4A = the UNDER pair, the ONLY banks any
    code reads (restore re-verified instruction-exact: linear
    (z'·H+i)·W+j, mirror word +2·z, seen +0x10+z, DAT volume low
    byte); +0x3E/+0x42 = the CURRENT pair ≡ shipped TOT/DAT at
    .POS footprints (434/435 ZONEA/M1; 1 miss = footprint overlap,
    last-slot-wins) — DEAD EDITOR PAYLOAD, zero readers, the
    runtime spawn-stamp hypothesis RETIRED. BONUS: the 0x1E-B
    mirror-record grammar unified (+0x1B/+0x1C = the OBJECT-HEIGHT
    pair — closes the MISSIONVIEW §8.1 producer hunt; +0x1D no
    traffic); FUN_0044889a/FUN_00448b80 = the objective-building
    family (zone-7 gate, counter [0x46cce0] over types 0x44..0x47,
    at zero SFX 0x28/0x29 + extraction-arm cells 0x46cd00/
    0x46ccfc/0x46ccc4); .POS word 2 = BASE Z LEVEL (FORMATS §12
    corrected); FUN_0041bc1c TRT death stamp (per-zone rubble word
    0x454a04). Docs-only; registry_anchors green; manifest clean.
3. [P4/FORMATS] THE .BLD RECORD WALK unit (unattended, bounded RE):
   the .BLD names/graphics side (FORMATS §17 — the 201-B/64-B-
   extension hypothesis still unanchored; the residual item of the
   D93 .MOFO retirement). Anchor the record grammar against the
   corpus .BLD files + the loader in ghidra-project/
   exw-text-objdump.txt; land FORMATS §17 + ledger rows.
    NOTE 7j.32 (2026-08-22, D96): the .BDG side is fully closed —
    BLD is the names/graphics sibling (r=0.985 size correlation);
    the loader call should sit near the .NME/.POS/.BDG loader
    family (mission-load chain 0x447b3a..0x447c00) — grep the
    ".BLD" DGROUP string for the anchor.

## Backlog (not yet started)
- [P4.2/W7-followups] after the differ core: the T2/T3 field maps on
  the E side (projectile/critter banks, effects/debris rings) as
  their producer families land in-engine (S3+ pairing per §10-W12);
  the O2/Wine tiebreak channel (W11); O3 8street comparator (W10).
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
- The 0x4787c4/0x47879c hot-rect record — CLOSED 2026-08-22
  (§7j.31/D95): ONE 0x20-stride array base 0x4787bc, grammar +
  7-writer census + octile picker + class dispatcher landed; SP
  click-orders never robot-targeted; new pins 0x46cc00/0x4ddb20
  (watch-set candidates for click parity, additive when needed).
- RETIRED 2026-08-22 (D93/§7j.29): the ".MOFO loader" — never
  existed (string-tail misparse). REMAINING from this bullet:
  the .BLD record walk (names/graphics
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
- Mission SFX tier (MENU1/MENU2-style mixer instruments; the
  bank→name DATA PREREQUISITE DELIVERED 2026-08-22 by §7j.30/D94 —
  202 durable assignments, zero unnamed cells) + the order SFX 0x2A armer click + the
  damage/alarm SFX families (7g.1) + the pickup SFX 0x43a48e
  entries (7h.2) + the select-ack SFX pair 0xC+k/0xF (7j.6) + the
  debris arrival-SFX pair FUN_00421e60/FUN_00421dec (7j.11 item 4).
  NOTE 7j.17 pinned new FUN_0043a48e banks: _DAT_004edf94/
  _DAT_004edfe4/_DAT_004edfac (robot fire) and
  _DAT_004edffc/_DAT_004edff0/_DAT_004edfa8 (critters/POI).
  NOTE 7j.20: the beacon armer's SFX is FUN_004239ef(0x2a,3).
  NOTE 7j.25/7j.30 CLOSED: the destroy-thud pair 0x4edfb8/
  0x4edfbc = DEADMAN1/DEADMAN2.RAW and the FULL bank-name walk
  landed as §7j.30 (commit a0f291c, D94).
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
- 2026-08-22: P4/RE THE .BDG TEMPLATE-BANK READER unit COMPLETE
  (worker ce347a0e claim 2, commit 4210f55, D96, docs-only;
  objdump-only from ghidra-project/exw-text-objdump.txt, no Ghidra
  run; read-only corpus probes in /tmp/opencode). CLOSED with a
  negative-result headline: +0x3E/+0x42 have ZERO readers — they
  are the editor's CURRENT-state pair (bank1 ≡ shipped TOT word,
  bank3 ≡ shipped DAT byte at every .POS footprint, 434/435
  ZONEA/M1 cells; the 1 miss = a genuine footprint overlap,
  last-.POS-slot-wins), already baked into the shipped mission
  files — the runtime spawn-stamp hypothesis is RETIRED. The
  loader DISK ORDER is interleaved vs the slot layout
  (+0x3E,+0x46,+0x42,+0x4A; 0x41a71d..0x41a782); the destroy
  restore consumes ONLY the UNDER pair (+0x46 → TOT-mirror plane
  words +2·z; +0x4A → seen=(word==0) @+0x10+z + DAT volume low
  byte; linear (z'·H+i)·W+j; z ∈ [z0, min(z0+D,8))) —
  re-verified instruction-exact. BONUS DECODES: (a) the 0x1E-B
  TOT-mirror tile-record grammar unified (plane words +0x00..0x0F,
  seen +0x10..0x17, +0x18 scorch, +0x19 variant<<4, +0x1A door
  bit7, +0x1B/+0x1C = the OBJECT-HEIGHT pair (z0, z0+D) — the
  MISSIONVIEW §8.1 unknown producer found = the objective family;
  +0x1D zero traffic); (b) FUN_0044889a/FUN_00448b80 = the
  OBJECTIVE-BUILDING family (zone-7 gate [0x4edd8c]==7, counter
  [0x46cce0] over instance types 0x44..0x47, heights stamped/
  cleared, at zero SFX FUN_004239ef(0x28,3)/(0x29,3) +
  extraction-arm cells 0x46cd00/0x46ccfc/0x46ccc4); (c) .POS word
  2 = the BASE Z LEVEL (FORMATS §12 kind-gloss corrected);
  (d) FUN_0041bc1c TRT death stamp (per-zone rubble word table
  0x454a04 + k15 debris + splash). Deliverables: RE-EXW-SIM §7j.32
  + 3 rewritten + 4 new ledger rows + FORMATS §2/§12/§16 +
  MISSIONVIEW §2 update + D96. registry_anchors green, manifest
  clean both sides, PUSHED. Queued: the .BLD record walk (item 3).
- 2026-08-22: P4/RE THE HOT-RECT RECORD unit COMPLETE (worker
  aa62f5ed claim 2, commit 5abeaad, D95, docs-only; objdump-only
  from ghidra-project/exw-text-objdump.txt, no Ghidra run).
  HYPOTHESIS CONFIRMED (one refinement): the 0x4787c4/0x47879c
  family is ONE 0x20-stride record array — base 0x4787bc (rec 0),
  count [0x46ccd8], cap 0x77, per-frame reset @0x403a9a; 0x47879c
  = base−0x20 = the dispatcher's 1-based view. Grammar: +0/+4
  world corner, +8/+0xC hit-box ORIGIN (NOT center — the picker
  adds w/2,h/2), +0x10 z, +0x14 w, +0x18 h, +0x1C type. Full
  traffic census: 7 writer sites ALL in renderer FUN_00403938 (w1
  0x403c87 robots gated [0x4edb88]==2 ∧ ≠local-player — MP-ONLY,
  type (idx+1)|0x1000, z+0x21, corner tile+0xB; w2-w7 critter
  .NME draw paths, type idx+1 plain, z ∈ {raw,+0x20,+0x10,>>8}, w
  ∈ {0x3C,0x40}) + picker FUN_00419943 (octile priority
  FUN_0041ebf8, early-out <4, returns i+1; ground fallback = iso
  (mx−0xF0)·k/0x1E0 + TRT active-scan → 0x2000|(idx+1)) +
  dispatcher FUN_00410644 (MissionShell @0x448021; type cell
  [0x46cc00] NEW pin; bit12 robot corner+z; bit13 TRT via the
  −0xC-bias base 0x4cccec ×0x20+0x10 — the 7j.28 "critter
  0x4cccec" gloss CORRECTED to TRT; else critter z =
  FUN_004128ec>>8+0x15; tail [0x4ddb20]|=2 order latch NEW pin +
  mouse consume). SEAM CONSEQUENCES: SP click-orders can NEVER be
  robot-targeted (E seam must not fabricate them — S2's ground
  seam validated); order-target units are per-class formulas vs
  the D82 cells 0x4dd484/88/8c. Deliverables: RE-EXW-SIM §7j.31 +
  3 ledger rows (supersedes the §7j.16 skeleton rows) + D95.
  registry_anchors green, manifest clean both sides, PUSHED.
  Queued: the .BDG template-bank reader unit (item 2) + the .BLD
  record walk (item 3).
- 2026-08-22: P4/RE THE SFX BANK-NAME WALK unit COMPLETE (worker
  7972b334 claim 2, commit a0f291c, D94, docs-only; objdump-only
  from exw-text-objdump.txt + DGROUP re-read from the binary, no
  Ghidra run; adopted + validated the interrupted 09:55 WIP dump
  exw-banknames.txt with an independent extractor — 17 widened-
  window rows verified, 1 artifact row rejected). DELIVERABLE: the
  COMPLETE bank→name map — 202 durable assignments, ZERO unnamed
  durable cells (RE-EXW-SIM §7j.30 + 2 ledger rows): mission set
  FUN_0043a1d3 = 27 registers incl. the MIDIGUN-duplicate quirk at
  0x4edf70; screen sets share cells (MENU1/2 + BEEP1/4/5/7 +
  TEXTBOX1 + DOOROPEN/CLSE); mission-extra BEAMIN/THROW/BIOFIRE/
  PEXPLODE/CACODETH/SQUAWK/GRUNT1..3; speech = 53 8-B {A,B}
  records at 0x4ee014 (95 files, pair slot-order FLIP at SPCH16,
  11 empty +4 slots, playback bypasses the steal path via
  0x44c8c4); GFX families + language G-variant gates (index
  0x4eba1c==1, edition [0x4edd8c]>4 → GRILLA) + per-ROLE palette
  cells (0x4edbf8 ×6 names). SFX CELLS HOLD VOICE-BASE HANDLES:
  FUN_0043a36e/39c = 1-/4-voice registers (staging cell 0x46af0c),
  FUN_0043a48e = the play/steal function (listener 0x4edde4/8,
  priority/age arrays 0x4ee1c2/0x4ee2e2, default vol 0x7f/pan
  0x8000 at −1,−1). All prior bank pins re-confirmed cell-exact.
  The unnamed 0x46afXX cells characterized (0x46af4c = the DAT
  volume pointer; 0x46af58 = 0x2710-B arena; 0x46af0c = the load
  staging cell; 0x46af5c = struct base). Manifest clean both
  sides. Queued: the hot-rect record unit (item 2).
- 2026-08-22: P4/FORMATS THE .MOFO LOADER unit COMPLETE as a
  NEGATIVE RESULT (worker 0a08a5e1 claim 2, commit 03e8c3b, D93,
  docs-only; objdump-only from the existing exw-text-objdump.txt, no
  Ghidra run). THE .MOFO LOADER DOES NOT EXIST: 0x457a4c "MOFO\0"
  is the dead tail of the fatal string "Buggered direction in
  MOFO" @0x457a3c (DGROUP bytes re-read from the binary; ZERO code
  refs — full .text immediate scan + the empty Ghidra XREF block);
  no ".MOFO" byte sequence in EXW or EXD; no *.MOFO file in the
  corpus (manifest verified both sides). The extension-tag family
  is CLOSED at .NME/.TRT/.POS/.BDG @0x457a57..0x457a6d (one ref
  each → the four CLOSED loaders 7j.15/7j.18/7j.25; the 7j.15
  ".MOFO" gloss corrected). BONUS PIN: the string's sole consumer
  FUN_00415490 = the mode-9 SEEK per-step target-acquisition
  dispatcher — dword@+0x10 dual-purpose (wander heading 0..255 /
  mode-9 direction 0..3 seeded RandA()&3 at the 0xB-dormant wake),
  `cmp 3; ja fatal` → the standard fatal idiom (fade-cancel
  0x420100 + print 0x44d2ac + FATAL EXIT 0x44d2da); 4-way tables
  0x415480 (acquisition: tight −4..+0xF ahead on the walk axis,
  |Δ|<0x18 cross + z; c3 robot-y-RAW quirk) + 0x412ef8 (steppers
  y−1/x+1/y+1/x−1); hit → target w@+0x7A + mode := 2 + anim := 0.
  Deliverables: RE-EXW-SIM §7j.29 + 2 ledger rows + FORMATS §0.1.
  Queued: the SFX bank-name walk (item 2).
- 2026-08-22: P4.2/W9 GATES/CI WIRING unit COMPLETE (worker cd3ebd73
  claim 2, commit 5026afc, D92). (a) CI LEG: the named
  `diffharness` job in .github/workflows/ci.yml (`cargo test -p
  diffharness` + `cargo test -p bedlam-game --test
  canonical_dump_gate --test differ_gate`) — DESIGN §9 DH-G3 section
  landed with the CI-proves-vs-live-session split (CI: compile +
  skip-cleanly + the corpus-free tests; the pinned chains
  8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 run on
  corpus-present machines; original-side NEVER in CI). (b) THE SWEEP
  was empirical, not grep-only: fresh corpus-free git clone (faithful
  CI-checkout sim; game-data never committed) + `cargo test
  --workspace --no-fail-fast` → exactly ONE non-skipping corpus
  dependency: menu_gate, 3 of 5 tests panicking via corpus_host()'s
  expect on the absent corpus. (c) FIX: menu_gate gains
  corpus_present() (LANGUAGE.ENG marker) + the three guards in the
  file's own pattern; the expects stay as corrupt-corpus tripwires.
  Post-fix: clone 52/52 targets green, workspace 565 tests green
  (all 5 menu_gate tests executing for real with the corpus), fmt/
  clippy clean, manifest clean both sides. Recipe in DESIGN §9:
  re-run the clone test whenever a new corpus gate lands (the named
  job enforces it). Queued: the .MOFO loader unit (item 2) + the SFX
  bank-name walk (item 3).
- 2026-08-22: P4.2/W8-s2 THE S2 ORDER SCENARIO unit COMPLETE (worker
  7faaeb53 claim 2, commits a9e6964 + 786c9fb, D91). (a) STAGING
  DESIGN (docs-first): grammar v1.2 `markers = x,y,z[; ...]` header
  key — the walk seam (the click-order moves only the OTHER robots
  in the order radius; the clicked robot snaps to spread slot 0, and
  D89 pins the SP squad at 1 robot on EXW/EXD/E alike). E stages via
  the EXISTING load_mission(staged_markers) seam (no staging-rule
  change, MRK+markers ≤ 12); O1 records the seam in the plan's
  `_e_staging` field and NEVER fabricates (a dbx-plan test pins that
  no inject row touches the robot bank/count). Also landed the
  predecessor 3595c744's uncommitted D90 journal. (b) S2.scen:
  markers 18,73,1 (the mission_corpus_gate walker) + order 21 73 1 +
  frames 16, tiers T0,T1,TS. (c) canonical_dump_gate
  corpus_s2_order_walk: 17 records, chain 809f4961b7757da4 pinned,
  double-run byte-identical; asserts the arm frame (window 0x197−1
  after the arming pump's decrement — the single-robot window-0
  clear does NOT fire at 2 alive; claims slots 0+1; clicked robot
  state-3 snapped + no target; walker state-4, present=1 target
  (22,73) Q5, stop_dist 1000000), the walk window (frames 1..6,
  monotone 18→21), the arrival clear at frame 7 (state 4→3, snapped
  ONE TILE SHORT at the (21,73) origin — the west-approach
  ARRIVE_RADIUS semantics; beacon 0 + claims all 0 on all-state-3;
  target RETAINED, present=1 persists), the move-target-words row
  form, and the steady tail. (d) differ_gate: S2 row in the loop —
  fabricated O1 carries the present=1 span both ways through the D90
  splice; cross PASS-WITH-NOTES (exactly the 2 E-only rows, ZERO
  robot field gaps), double-run PASS modulo counter/RNG, FAIL on
  money. (e) dbx-plan compiles S2: order-target 3-cell write at
  frame 1 + `_e_staging`; capture-plans/S2.json committed +
  byte-pinned. Workspace 52 suites green (565 tests), fmt+clippy
  clean, manifest clean. Queued: W9 gates/CI wiring (item 2).
- 2026-08-22: P4.2/W8-prep THE ROBOT-COUNT OVERRIDE PIN unit COMPLETE
  (worker b0656949 claim 2, commit f106cf1, D89, docs-only). ANSWERED:
  the original SP does NOT fill the 0x46cbe0 network-marker override —
  EXW FUN_0040cca0 @0x40cd8d gates it on [0x4edb88]!=0 (network
  sessions only; the EXD twin is the mode==0 branch of FUN_0001d9cd,
  instruction-for-instruction), and the title menu "New Single Player
  Game" @0x43aaa3 sets 0x4edb88=0 ∧ 0x46cbe0=1 for every local
  session. SP ZONEA banks ONE robot in EXW, EXD, and E alike —
  robot-count parity holds, robot-count diffs in SP scenarios are a
  genuine finding class, NO E-side staging seam changes. Corrections
  landed: EXW 0x46ccbc = TOTAL (EXD cap 0x11950c twin) vs 0x46cbd8 =
  PER-PLAYER (EXD 0x11958c twin) — RE-EXD-MAP §5 robot-bank row +
  RE-EXW-SIM §7c.7 fixed; future MP bank dumps must bound by the cap
  cell. Faithful quirk recorded: the SP marker write hits
  record[12]+0x2A (stale MRK-copy counter, both twins) — harmless.
  Evidence: local ghidra-project/exw-spawncount-asm.txt (verbatim
  extract from the 7j.27 objdump — no new Ghidra run; desync/
  realignment note in its header). DESIGN §10-W8 resolved; DECISIONS
  D89; RE-EXD-MAP §5d. Queued: the move-target plan-row fill (item 2).
- 2026-08-22: P4.2/W7-followup THE EXD ROBOT BACK-HALF PROBE unit
  COMPLETE (worker 03be9318 claim 2, commits 455ca41 + 206b776, D88).
  (a) RE NOTES FIRST (455ca41): two `-process BEDLAM.EXD -noanalysis`
  passes (EXDRobotBackhalf{,2}.java; dumps ghidra-project/
  exd-robot-backhalf{,2}.txt) — hop 1 program-wide census of the
  0xf6d34..0xf6ddc family (every hypothesis offset has traffic;
  `[i·0xA8+const]` + `[i·0x15·8+const]` idioms) + the FUN_0001c7dc
  per-phase-tick disasm/decompile (the phase-4/5 gate, decay family,
  stop 0xf4240 arm, arrive snap, beacon auto-order, pod-gate tail);
  hop 2 the writer family: FUN_0001ef61 = the DAMAGE APPLIER (EXW
  0040e230 twin — hit_flash +=1 first, alarm trip 100, alarm_ctr +3/
  >100/reset, shield absorb/decay, hp ceiling battery·100+5000,
  death paths := death_flag 1), FUN_0001d9cd = the SPAWN INITIALIZER
  (EXW 0040cca0 twin — kind := player type, variant := RandA&3,
  facing := 0xFFFF, probe_z 8-word seed, stat switch 0x2A/0x2B/
  0x2C×200 → charges/battery/pool, MRK pos formula, stagger, cap
  0x11950c := count 0x11958c), FUN_0001d274 = robot_move (dir_byte
  := angle, facing cardinals, anim ((a+4)&0xFF)>>3), FUN_0001e440
  probes, FUN_00020dea pad charge (armor +20 clamp 3000 bar 2500,
  pool gate), FUN_000180a1 portrait clamp-5, FUN_0005961c = the SP
  all-dead sweep (death_flag READER — closes 7g.6). §8 table
  rewritten: 23 fields + per-field provenance; coverage gaps 26 → 3
  (target trio, record-external); move-target EXTENT pinned (0x60-B
  span at 0xf75ec); §1b size fix (14,644 B = FUN_0001476d). (b) CODE
  (206b776): EXD_ROBOT_MAP/EXW_ROBOT_MAP widened to the 31-leaf pin
  (FieldKind {I32,U16,I16}, armor i16); drop_countdown rebound
  +0x2C → +0x80 (the ENGINE field's semantics — the W7 binding was
  the pod timer); differ.rs fixture = the full independent
  transcription (anti-fabrication now the target trio, 34−31=3);
  differ_gate.rs inverse fabricator places all mapped fields, S1
  coverage re-pinned 2+26 → 2+3 with the pinned chains
  8901789a88cf61fe / 1c4e7b4c9d9b0947 re-asserted GREEN. Workspace 52
  suites green, fmt+clippy clean, manifest clean before/after the
  Ghidra runs. NOTE for W8/live: EXD alarm_ctr has a phase-0 decay
  EXW 7g.1 does not document (evidence gap); EXD SP banks exactly 1
  robot on ZONEA like E. Queued: W8 the robot-count override pin
  (item 2, now half-answered) + the move-target plan-row fill (item
  3, coverage 3 → 0).
- 2026-08-22: P4.2/W7 THE DIFFER unit COMPLETE (worker
  c594df62 claim 2, commits a9d741f + 04d1d27 + 0dfdb0c, D87 + the
  RE-EXD-MAP sec 8 basis). (a) RE NOTES FIRST (a9d741f): the EXD
  robot-record normalizer field map provenance-tagged — x@+0/y@+4
  [seed#1+anchor writer], z@+0x08 PINNED NEW (the per-player writer's
  d@(0xf6d3c+i)+0x20), state@+0x0C, drop@+0x2C, stop@+0x74, hp@+0x78,
  alive@+0x7C; the 26 unmapped canonical fields = coverage findings
  (never zero-filled); the seed-#1 EXW-front conflict recorded OPEN
  (O2 uses the SIM sec 3 table; W11 arbitrates); the non-robot O1 row
  forms (beacon u16-span widen, static-map-wh 0x2c span, typedb
  len-0==all-zero-grid equivalence, rng u32->u64). (b) THE DIFFER
  (04d1d27): differ.rs — channel normalizers (E canonical parse / O1
  per sec 8 / O2 per EXW sec 3+7f/7g), MODES DoubleRun (the DH-G1
  verdict: identical modulo frame-counter T2 + rng T3, draw-COUNT
  checks still apply) + CrossChannel (per-field classes, O2
  ARBITRATION: O2 sides with O1 -> engine-bug, with E ->
  original-divergence, none -> provisional), the `coverage` bucket
  (metered, never silent, notes-not-fails), constant-shift alignment
  (<=8, T1-timing note), event-timing table, T3 draw-count gate,
  report_text + manifest_json; bin/dbx-diff CLI (mode auto from
  channels, --tiebreak/--t2-quantum/--report/--manifest, FAIL exit
  code). (c) VERIFIED: 15 differ tests (hand-built EXD fixtures =
  independent sec-8 transcription; the W6 canonical literal as the
  shared-field contract; arbitration both ways; 26-gap math; shift;
  determinism); corpus-gated differ_gate.rs — S0/S1 run_canonical
  (pinned chains 8901789a88cf61fe / 1c4e7b4c9d9b0947 re-asserted) x
  the INVERSE normalizer -> cross PASS-WITH-NOTES (S0: 0 coverage;
  S1: 2+26 + the one T2 counter note, zero engine-bug/structural),
  double-run PASS modulo counter/RNG, FAIL on money perturbation; CLI
  smoke-tested on the real S0 E dump (PASS, chain pin shown). 52
  diffharness tests total; workspace test/fmt/clippy green; manifest
  clean. PUSHED 0dfdb0c. Queued: the EXD robot back-half probe
  (item 2) + the W8 robot-count override pin (item 3).
- 2026-08-22: P4.2/W5-pad THE CAPGEN PAD OP unit COMPLETE (worker
  85dedea3 claim 2, commits fb92286 + b5d1920, D86). (a) capgen
  `{op:"pad"}` inject form: reads the 8-B .PAD slot record
  {u16 active@+0, x@+2, y@+4, z@+6} from the pad bank at the
  capture-frame stop (MEMDUMPBIN through the bank's own SEG form +
  slot*8), FAILS LOUD unless active==1 and x!=0xFFFF (a slot the
  staged mission never loaded is a capture error naming the slot),
  then writes {x,y,z} as i32-LE x3 to the order-target triple (EXD
  0x10e0a4/a8/ac; tile coords = the shared-grammar contract). The op
  writes only the ORDER — the robot's arrival arms extraction
  in-game. (b) dbx-plan un-gate: `pad <slot>` compiles to the op row
  (bank from static-pad-slots — a READ anchor with its OWN gap error;
  targets from order-target; slot 0..998 re-checked). (c) The §7j.20
  extraction-pad census committed as S6 authoring data in DESIGN §7
  (zone 1 {8,0x10,0x12,0x18}, zone 2 {4,5,7,0xE,0x11}, zone 3
  {0,1,6,0xF,0x15}, zone 4 {0,2,0x10,0x15,0x16}, zone 5 {8,9 ×2,
  0x3D} + the shared slot-6 tail). (d) VERIFIED: `dbgprobe pad`
  headless GREEN both legs (positive: seeded slot 2 = the real
  ZONEA/MISSION1.PAD record 0 (5,61,0) → triple 05000000 3d000000
  00000000 + injected flag; negative: inactive slot 3 aborts, no
  transcript — NB the negative run log lives OUTSIDE the capgen
  workdir, capgen purges stale *.log there); gate/inject/flow/walk
  regression-GREEN; new dbx-plan tests (op-row emission vs the REAL
  registry + the READ-anchor gap refusal); S0/S1/S0W byte-pinned
  plans unchanged; scratch CLI end-to-end compile (pad 8 → frame-6
  op row); workspace test/fmt/clippy green; manifest clean. The E
  side still rejects pad steps naming the S6 seam (W12). Queued: W7
  the differ (item 2).
- 2026-08-22: P4.2/W6 THE ENGINE DUMP EMITTER unit COMPLETE (design
  83f04b9 by worker 1f758667 claim 2 — interrupted mid-implementation;
  adopted + completed by worker 36f752cd claim 2, commits 54d781a +
  docs, D85 + completion addendum). (a) parity_harness --canonical:
  drives GameHost over the SHARED v1.1 scenario grammar (the D82
  seam) and stitches channel-E W3 dumps through the SAME
  runner::stitch + encode_dump path as O1 captures; T0/T1/TS field
  maps in examples/parity_harness/canonical.rs per DESIGN §6a (every
  unmapped row an explicit E-gap); 3 read-only accessors added, no
  engine behavior changed, diffharness a bedlam-game DEV-dep only.
  (b) WALK-PHASE FIX over the WIP: walk accepts ONLY boot steps (the
  blanket empty-walk rejection had made the difficulty seed
  unreachable); DESIGN §6a amended. (c) VERIFICATION
  (tests/canonical_dump_gate.rs): hand-encoded §6a grammar fixture
  (98-byte robot-bank record literal, frame digest pinned
  b359f7d282db7cb8); synthetic MissionSim run (chain
  ea0bc53dc95ff0b2, double-stitch byte-identical, surviving
  2-robot order window 0x196); corpus-gated S0/S1 (3/401 records,
  chains 8901789a88cf61fe / 1c4e7b4c9d9b0947, byte-identical double
  runs); seam gates (boot difficulty=2 → money 3000, command/pad
  rejections naming the seams, P 0x19 banned, order arm proven by
  state-3 + tile snap). FINDINGS: static-map-wh = TOT-header map
  size 25×75 (30004/15004 are FILE bytes); E stages no
  network-marker override → ZONEA single-robot squad, armer's
  window-0 case clears the order on the arming tick (W8 must pin
  0x46cbe0 override parity). Workspace green (49 suites), fmt+clippy
  clean, manifest clean. PUSHED 54d781a. Queued: W5-pad (item 2),
  W7 the differ (item 3).
- 2026-08-22: P4.2/W5-walk THE SCRIPTED-MENU-WALK DRIVER unit COMPLETE
  (worker 845abdc5 claim 2, commits 59ec9a5 + b67dcaa + 33b2c17, D84).
  (a) DESIGN FIRST (59ec9a5, RUNTIME.md "W5 walk driver"): the stop
  model — the BPLM boot trap on the frame-counter cell 0x1195f0
  doubles as the walk driver, one stop per counter-writing screen
  frame; SMV writes at stop i become screen frame i+1's input;
  keystore re-arm per input (AnyKeyWait twin consumes on read);
  anchor BP CS:0005A6EB arms only AT THE LAST WALK STOP (BPDEL * drops
  the trap — no stop-type ambiguity). (b) COMPILER (b67dcaa):
  walk-phase keystore steps -> stop-indexed plan rows ("walk" key;
  Advance consumes stops; runaway 1M guard; order/pad/command refused
  as "not menu-walk steps"); registry-derived walk_watches calibration
  trio (walk-mode 0x1075d8 / walk-zone 0x107500 / walk-mission
  0x119610); resolve_at=anchor emitted for ALL plans — FIXES the
  latent D81 gap (loader statics are mission-load values; the
  arm-stop read was pre-mission garbage feeding len exprs); S0/S1
  regenerated, S0W.scen + capture-plans/S0W.json committed (STRUCTURAL
  DRAFT schedule — stop indices calibrate live via the per-stop
  transcript comments; then pure data). (c) CAPGEN plan v3: the walk
  loop (write-then-dump per stop, arm-at-walk-end), walk_watches
  transcript comments, resolve_at=anchor, boot_writes at the accept
  stop (identical for walk-less plans). (d) VERIFIED headless:
  `dbgprobe walk` GREEN (stop indexing incl. a pure-skip stop,
  calibration notes, arm-at-walk-end, anchor resolve feeding expr
  lens); gate/flow/inject regression-GREEN; 52 diffharness tests (4
  new), fmt+clippy clean, manifest clean. PAD op deliberately its own
  unit (queued item 3). Queued: item 2 = W6 the engine dump emitter.
- 2026-08-22: P4.2/W5-followup THE EXD INPUT-TWIN CENSUS unit COMPLETE
  (worker ef11271c claim 2, commits 79362a9 + 110718d, D83). Four
  Ghidra probe passes (-process BEDLAM.EXD -noanalysis): (a) KEYSTORE
  0x894d4 — AnyKeyWait twin FUN_00030792 (scan 1..0xFE skip 0x2a/0x36,
  consume-on-read) + the DOS KeySink = INT-9 hook @0x303f5 (make/break
  keystore[AL&0x7f]:=1/0 + held-counter 0x107534 + the ARROW-REMAP
  OR-0x80 twin) + installer/memset FUN_0003064d; ScanToChar twin
  0x307c1 (shifts 0x894fe/0x8950a, tables 0x8077a/0x8097a). (b)
  COMMAND RING 0x9255c stride 0x80 + COUNT 0x119588 — builder
  FUN_0005b066 / consumer FUN_00019ee9 (EXW 00449c94/00409138 twins;
  record byte@+0 marker/id@+1/spot@+3/flags@+5/xyz@+7/+9/+0xB EXACT;
  auto-arm 1000000; weapon-slot dispatch w2/3/4→FUN_0001c3fb,
  w6/7/8→FUN_0001bd8f, artillery→projectile bank 0x980d4×0x36
  field-exact = T2-ready bonus). (c) ORDER TARGET 0x10e0a4/a8/ac +
  order-active 0x10e140 — consumer bit1 writes + click-order twin
  FUN_00021112 (pick twin FUN_0002a271, iso (p−0xF0)·k/0x1E0 EXACT);
  EXD MissionShell trio position EXACT. (d) DIFFICULTY 0x119558 — the
  172/236/300 formula + respawn table twin 0x81050 in FUN_00023967;
  44 refs, writers FUN_0002c6e3. Registry: 6 rows filled (TI aliased
  where pinned; emptiness rule → T2-T4 only); dbx-plan order-target
  form + REAL-registry step tests; S0/S1 regenerated (19+10 / 33+24).
  Scratch-verified: keystore/order/command steps compile end-to-end
  (incl. remapped arrow CS:0008959F). Divergence seeds 6-7 (attack-
  break frame-masks vs RandA; EXD staging cells). 49 diffharness
  tests; workspace test/fmt/clippy green; manifest clean. PUSHED
  110718d. Queued: item 2 = the scripted-menu-walk driver (unblocked).
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
