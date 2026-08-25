# DESIGN-DIFFHARNESS — the P4.2 differential harness architecture

Status: DESIGN DOC (P4.2, 2026-08-22, worker 4d7b9a5b, claim 1). Provenance:
every watched address is anchored to its ledger row (RE-EXW-SIM §8 row name +
amendment section, or the named companion doc). Tag convention per PLAN §9:
[verified] = disasm-anchored (the ledger row already carries the evidence);
[derived] = consequence of verified rows; [hypothesis] = to be settled by this
harness; [pin-unverified] = runtime fact to confirm at the first interactive
session (the RUNTIME.md skeleton's UNCERTAIN discipline).

Companions: PLAN §0/§0b/§6-P4.2 (role + budget), RUNTIME.md (pinned DOSBox-X +
Wine + the B2 sandbox), DECISIONS D17/D26/D28/D29 (engine determinism, CPU
baseline, sandbox), DESIGN-GAME §7 (hash boundary), DESIGN-AUDIO (audio side,
out of scope here), DESIGN-RENDER (pixel side, out of scope here).

## 0. What this harness IS and IS NOT

Per the tiered parity budget (PLAN §0b) the harness is NOT an all-zone
tick-parity gate. It is three things:

1. **Divergence meter** — quantified per-scenario, per-field-class divergence
   between the original and our engine, so spend is visible, not vibes.
2. **Structural-error catcher** — layout/count/ordering mistakes (wrong record
   stride, wrong producer order, missed reset) that endpoint goldens cannot
   see. This is the primary P4/P5 use.
3. **Regression tripwire** — same scenarios re-run per change; first-divergence
   frame + field must move only when the change intended it.

It additionally serves as the **hypothesis arbitrator** for the accumulated
7j open questions (§8) and as the **trigger instrument** for the corpus-off
producers (nothing fires/dies/gets destroyed on the corpus path today; §7 S3+).

## 1. Oracle topology — three original-side channels + the engine

| id | channel | role | status |
|---|---|---|---|
| O1 | BEDLAM.EXD (DOS) under pinned DOSBox-X | PRIMARY scripted differential oracle: per-frame state dumps + input/command injection | pinned runtime exists (RUNTIME.md D29, B2-proven sandbox model); needs W1 (EXD import + EXW→EXD address map) |
| O2 | BEDLAM.EXW (Win95) under pinned Wine | CANON TIEBREAK + spot-check channel: every RE'd address applies verbatim; arbiter whenever O1 diverges from EXW semantics | prefix pinned (RUNTIME.md); watcher = host-side ptrace driver (W11) |
| O3 | instrumented 8street build | SECOND comparator (test-only per PLAN §0/§1): emits tick-boundary state dumps; three-way disagreement localizes inherited-8street error | late (W10); never a porting source |
| E  | our engine (bedlam-core MissionSim + bedlam-game scenes) | the compare target; per-tick canonical dumps + state hash chain (D17/D26, parity_harness D28) | exists; needs the canonical dump emitter (W6) |

**Decision (D77):** O1 is the scripted-differential primary because PLAN §6
P4.2 names DOSBox-X debugger memory-watches, the D29 sandbox model (flatpak
isolation, corpus-by-rsync-scratch, interpreter core, watch-mode debugger) is
already proven against a sibling DOS4GW LE binary, and the debugger is the
only channel with first-class guest-memory visibility without touching the
original binary. **EXD is the instrument of observation; EXW stays the canon
of record** (PLAN §0 tiebreak order). Every EXW↔EXD behavioral divergence the
harness surfaces lands in docs/DIVERGENCES.md and is classified
`original-divergence` by the differ — never silently treated as an engine bug.
O2 exists precisely to adjudicate those, with zero address translation because
the whole watch set below is already EXW-anchored.

What this rules out (and why):
- **No patching/instrumenting the original EXW/EXD binaries.** The oracle must
  remain byte-identical to the shipped game (hash-pinned at run start) or it
  stops being an oracle. All observation is external: debugger reads, ptrace
  reads. This is also why O3 (8street, which we DO rebuild) is a separate,
  clearly-labeled comparator rather than the primary.
- **No host-level synthetic input.** Keystroke/mouse event injection at the
  OS/emulator level is frame-indeterminate. All injection writes the game's
  own input/command state at the frame boundary (§5).

## 2. Frame model and alignment

[verified — RE-EXW-SIM §1, RE-EXW-PACER] The original mission frame is one
MissionShell@0044771c loop pass: input handling → FUN_0040b835 (mouse click)
→ FUN_00410644 + FUN_00449c94(1) + FUN_00409138 (orders/commands) →
robots() FUN_0040b9f6 × **6 phases** → enemy pass ×4 (FUN_00410823(i) +
FUN_00412010, + FUN_004197d4 on odd i) → epilogue chain (debris tick
FUN_00420549@0x447feb, epilogue tick FUN_00424051@0x447ff0, splash/arrival/
platform ticks, effects mover FUN_00419f62@0x44813d) → draw chain →
PresentEnd@00425a03 (DDRAW flip) → `g_frame_count++` (0x46ae68).

- **One harness frame = one engine tick** (D17 fixed timestep = 1 original
  frame; the 6-phase + 4-pass sub-tick structure is inside both).
- **Dump point = the epilogue/present tail, after the last state writer and
  before the flip.** EXW anchor: PresentEnd call site in the MissionShell
  tail (0x447ff0-adjacent epilogue; the dump reads are side-effect-free so
  the exact instruction within the tail is a W1/W11 detail recorded in the
  run manifest). EXD anchor: the EXD present/flip site, pinned by W1 (the
  analog of the B2 PresentFlip@0x1066b skeleton trigger).
- **Frame index** = g_frame_count@0x46ae68 [verified] — every dump record
  carries it; the differ aligns by it, never by wall time.
- **Pause path**: the P-latch pause still calls PresentEnd in its spin
  [verified SIM §1] — the runner must not inject P (scan 0x19) mid-scenario,
  and the differ treats a frozen frame as a halt, not divergence.
- **Mission-start offset**: the first seconds of any mission have robots
  frozen in pods (§8 S1). Scenarios diff steady-state from an explicit anchor
  event (e.g. "first frame with all pods released"), defined per script —
  never from mission-frame 0.

## 3. Dump pipeline (original side)

```
scenario script ──> runner (W4) ──> DOSBox-X (pinned conf, debugger watch mode)
      │                                   │  frame-trigger hit:
      │                                   │    1. apply this frame's injections (§5)
      │                                   │    2. bulk-read the scenario's watch tiers (§4)
      │                                   │    3. emit frame record to D: (harness-out)
      ▼                                   ▼
engine dump emitter (W6, parity_harness -canonical) ──> same frame-record schema
      │                                                   │
      └────────────────> differ (W7) <────────────────────┘
                             │
                    divergence report (classes, meter, first-divergence)
```

- **Trigger**: ~~[pin-unverified] the exact DOSBox-X debugger command surface
  (BPINT/BPLM/D forms, watch-mode logging, startup.js automation route) is
  verified at the first interactive session per the RUNTIME.md skeleton
  checklist — that session converts this section's UNCERTAINs into committed
  runbook facts. Fallback trigger class if watch mode can't bulk-read on
  demand: a single linear breakpoint at the frame-tail site whose handler
  reads the whole tier list (the B2 skeleton already assumes this shape).~~
  **RESOLVED NEGATIVE 2026-08-22 (W4 DH-G0 channel audit, RUNTIME.md "DH-G0
  channel audit"): the pinned flathub DOSBox-X 2026.08.02 has NO integrated
  debugger** (build gates it off; `debuggerrun`/`-break-start` inert) **and
  its Duktape startup.js is log-only** (no memory access, no hooks).
  O1 therefore needs a **channel re-pin** before any live trigger: (a)
  self-build DOSBox-X at a pinned commit with `--enable-debug=heavy`
  (conf pins carry over), (b) GameLink linear-read feasibility, or (c) the
  O2 ptrace route as primary. W4 ships the channel-AGNOSTIC plumbing: the
  runner stages the scenario + corpus + conf and consumes a **capture
  transcript** (`DBXCAP` line format, tools/diffharness/src/bin/dbx-stitch)
  that whatever channel lands at DH-G0 emits; the stitcher converts it to
  the W3 dump + digest manifest. The live-run piece is
  [BLOCKED]-on-DH-G0-channel-repin.
- **Dump format** (W3, one schema for O1/O2/O3/E): a versioned record stream —
  header {schema_ver, channel, build_sha256, pin versions, scenario id},
  then per frame: {frame_no, injection_applied, per-watch {id, raw bytes,
  len}}, then a trailer digest. Per-frame digest = FNV-1a-64 over the
  canonicalized records (same hash util as the engine's StateHash, tag
  `BDLD`), giving a per-scenario **dump chain** directly comparable to the
  parity_harness scene-hash chain (D28). **W3 LANDED** — the byte grammar
  is pinned in `tools/diffharness/src/dump.rs` (module docs) and D78:
  `BDLD`-tagged LE records, canonical watch order = registry file order,
  digest input = tag + canonical frame bytes, chain = the D28
  construction (incremental FNV, `write_u64` per frame digest), frame_no
  strictly increasing, `BDLT` trailer {frame_count, chain_digest};
  encoders validate ids against the committed registry, decode verifies
  every digest + the chain. The FNV-1a-64 util is mirrored (not
  depended-on) to keep the crate zero-dep, cross-checked against the
  engine's public vectors in `tests/dump_schema.rs`.
- **Dump hygiene (hard rule)**: dumps derive from original game memory, so
  they are asset-derived data — they live under runtime/harness-out
  (git-ignored), NEVER in git. What git carries: the watch registry (W2),
  scenario scripts, the differ, and **content fingerprints only** (per-scene
  dump-chain digests), same policy as PLAN §6-P4.3 goldens.
- **Sandbox isolation** inherits D29 unchanged: corpus via rsync scratch
  (game-data read-only and invisible to the emulator), outputs to D:.
  MANIFEST.sha256 verified before and after every corpus-touching run.

## 4. The watch set (tiered; every row ledger-anchored)

Tiers order the build (W2 commits this table as data):
- **T0** frame/session scalars (always on, every scenario)
- **T1** the P4 slice (robots/orders/terrain — engine-modeled today)
- **T2** projectiles/weapons/critters (engine producers land with this
  harness's scenarios)
- **T3** effects/debris/rings/objectives (same)
- **T4** event-capture (breakpoint-logged calls, not bulk reads)

Static-after-load tables (type table 0x4dedf2, order table 0x4de664, TOT/DAT/
CGR/BIN/MIN bank pointers 0x4ede20/0x4edd58/0x4edd60/0x4ede1c/0x4edd9c, map
w/h 0x4eddec/0x4eddf0) are dumped ONCE at mission start and hash-compared —
they verify the loader, not the tick.

### T0 — frame & session

| watch | address (EXW) | layout | ledger anchor |
|---|---|---|---|
| frame counter | 0x46ae68 | u32 | RE-EXW-PACER names; SIM §1 loop tail [verified] |
| RNG states A/B | 0x4ede48 / 0x4ede4c | u32 ×2 (seeds 123456/234567, A reseeded per mission; LCG add-tail 0x62E9/0x3619) | RE-EXW-GAMETHREAD globals table; SIM §1; §7j.11 item 4 [verified] |
| score / money | 0x4dd40c / 0x46ae70 | u32 ×2 | SIM §7f sidebar (score strip) [verified] |
| difficulty | 0x46cbf8 | u32 (0..2) | §8 row "difficulty scalar" [verified] |
| zone / mission / mode | 0x4edd8c / 0x4edd88 / 0x4edb88 | u32 ×3 | §7j.21 elevator stager row; §8 rows [verified] |
| linear mission (m) | 0x46ae8c | u32 | §7j.20 pod stagger row [verified] |
| SFX master gate | 0x4ede58 | u32 (≠0 = SFX enabled) | §8 row "debris arrival SFX" [verified] |

### T1 — the P4 slice (robots, orders, terrain)

| watch | address (EXW) | layout | ledger anchor |
|---|---|---|---|
| robot bank | 0x4c69e4, count cell 0x46ccbc | count × 0xA8 raw (state+0x0C, hp+0x78, pod timer+0x2C = word @0x4c6a10+idx·0xA8, pos/facing/anim, order words +0x36..) | §8 rows "robot record base/stride/count", "pod-deploy countdown writers", "robot damage applier" [verified] |
| selection triple | 0x46cbd4 / 0x46cbdc / 0x46cbd8 | u32 ×3 (selected idx / cursor / squad size) | §7 "sim hash must cover" [verified] |
| blink-cursor selector | 0x4dc5d0 | u32 (0 or endangered-slot+1) | §7j.59 census [verified] (was §7j.7 item 6); the S1 hypothesis watch — statically constant 0 on corpus paths |
| per-player selected anchor | 0x4c71c4 | 4 × 0xC | §8 row "per-player selected anchor" [verified] |
| order target | 0x4dd484 / 88 / 8c | i32 ×3 (x/y/z) | §8 row "click order target" [verified] |
| per-robot move-target words | 0x46cc30 / 0x46cc60 | u16 arrays | §7, §7j.17 command-record row [verified] |
| extraction beacon family | 0x4eabb0 / b2 / b4 / b6 / b8 | u32 flag, u32 timer, 3 × tile | §8 row "extraction-beacon armer" [verified] |
| spread claims | 0x4eabba | 12 × u16 | §8 row "spread-claim picker" [verified] |
| no-extract latch array | 0x46aed4 | per-robot u32 | §7j.27 latch census [verified] |
| tile word grid | 0x460dfa + 2·tile | u16 × w·h·? (bounded by map w/h) | §8 row "tile word grid" [verified] |
| platform strength bank | 0x465daa + 2·tile | u16 × map | §8 row "platform strength bank" [verified] |
| type-DB mirror rows | 0x4796bc + 30·tile | 0x1E-stride rows (z-words @row+2z; seen byte @row+0x10+z) | §8 row "fast z-writer"; §7j.10, §7j.12 [verified] |
| type-DB +0x18 fade byte | 0x4796d4 + tile·0x1E | byte × map | §7j.10 item 1 (global per-frame fade) [verified] — the ring-overlap confirm watch |
| variant/flag bytes | 0x4796d5 / 0x4796d6 | byte × map | §8 row "type-DB tail stamper" [verified] |
| object instances | 0x46cbf4, count cell 0x46cbe8 | 2000 × 0x14 {x,y,z,id,flags,hp} | §8 rows "tile word grid", ".POS + .BDG loader" [verified] |
| TRT array | 0x4cccf8, count cell 0x46ccd4 | 250 × 0x20 | §8 row "terrain-structure array" [verified] |
| armor-pad byte reads | (covered by 0x4796d4) | — | §7j.8 item 8 (raw reader) [verified] |

### T2 — weapons & actors

| watch | address (EXW) | layout | ledger anchor |
|---|---|---|---|
| weapon-anim bank | 0x4c71f4 | 400 × 0x36 {type w@+0, owner, target, tick, xyz Q13, v, class, arc, trail link} | §8 row "weapon-anim tick" [verified] |
| projectile bank | 0x4cc654 | 50 × 0x22 {type w@+0, xyz Q13, v} | §8 row "projectile tick" [verified] |
| mortar trail bank | 0x4e66b8 | 20 × 0x68 {active, ring&7, 8 × xyz} | §8 row "mortar smoke-trail bank" [verified] |
| critter bank | 0x4cff98, count cell 0x46cc2c | count × 0x7E (state+0x0C, hp+0x06, kind+0x00, presence+0x24) | §8 row "critter-actor controller" [verified] |
| POI/personnel bank | 0x4dabdc, count cell 0x46cbf0 | count × 0x1E {active, state, heading, timer, xyz} | §8 row "POI/personnel controller" [verified] |

### T3 — effects, debris, rings, objectives

| watch | address (EXW) | layout | ledger anchor |
|---|---|---|---|
| debris stager | 0x476fbc | 128 × 0x30 {active, xyz, seq, kind, phys, delay+0x24, param, seq-table} | §7j.7 item 5; §7j.11 kind table [verified] — the S1 "2k start-delay" watch |
| effect rows | 0x4cec38 | 80 × 0x20 (LRU-aged) | §8 row "effect-row spawner" [verified] |
| rising-debris bank | 0x4cf638 | 80 × 0x1E {xyz, v, group, active, delay+0x1A, frame} | §8 row "effects-bank stager" [verified] |
| platform/blast bank | 0x4eb638 | 32 × 0x14 {x, y, z, age, frame} | §8 row "robot-death blast bank" [verified] |
| splash records | 0x4e9778 | 250 × 0xA {x, y, z, delay, age} | §8 row "splash records" [verified] |
| arrival/ride records | 0x4dcdb8 | 45 × 0x24 {active, marker, dest, countdown, robot} | §8 row "arrival ride tick" [verified] |
| door rects | 0x4dcae8 | 45 × 0x10 | §8 rows "door-rect list boundary", "door open/close" [verified] |
| delayed trigger timers | 0x4ea828 | 32 × 0x18 | §8 row "delayed trigger timers" [verified] |
| pod ring | 0x4e64c0 | 12 × 0x1C {active, phase, x, y, alt, group, dwell} | §8 rows "dropship ring banks", "pod spawner" [verified] |
| exit ring | 0x4e662c | 5 × 0x1C (same layout) | §8 row "exit/threat slots" [verified] |
| dropship frame | 0x4e6610 | 1 × 0x1C | §8 row "dropship deployer" [verified] |
| objective slots | 0x4eaaee | 6 × 0x20 {remaining, type, status, quota} + resolver phase 0x46cd00 | §8 row "mission-objective resolver" [verified] |
| escape counters | 0x4eba0c / 0x4eba10 | u32 ×2 | §7j.19 (CLOSED census) [verified] |
| tile-claim bank | 0x46af58 | 10000 B | §8 row "tile-claim bank" [verified] |

### T4 — event capture (breakpoint-logged calls)

| event | hook (EXW) | logged args | ledger anchor |
|---|---|---|---|
| SFX dispatch | FUN_0043a48e entry | (bank u32, ?, x, y, push) | §8 rows "destruction-thud SFX pair", "impact SFX trio" [verified] — feeds the SFX-family bank-name walk |
| order dispatchers | FUN_0040b615/0xaf98/0xa56f/0xace8/0xa7a1/0xa9ff entries | robot idx, target | §7j.17 command-record row [verified] |
| debris stage | FUN_00420608 entry | (x, y, z, kind, delay, param) | §7j.7 item 5; §7j.11 [verified] — direct producer-event stream |
| destroy tail | FUN_0041a894 entry | (x Q13, y Q13, ctr, damage, score flag) | §8 row "weapon impact resolver" [verified] |

Event capture is per-frame bounded (counts per frame are tens at most) and is
the cheapest route to the corpus-off producer question ("which bank/kind fired
and when") without diffing whole banks. [pin-unverified] feasibility of
entry-breakpoint logging at scale in watch mode — measured at W4 bring-up; if
per-call breakpoints perturb timing intolerably, T4 falls back to whole-bank
T2/T3 diffs (producer identity is recoverable from record deltas).

### Injection surface (watched AND written — §5)

| seam | address (EXW) | layout | anchor |
|---|---|---|---|
| key state | 0x4edc44 | 256 B scan-indexed, 1 = held | RE-EXW-INPUT §keystore [verified] |
| remapped key bytes | 0x4edd0c/0f/11/14 | bytes | RE-EXW-INPUT [verified] |
| mouse buttons | 0x4dc6e4 | bit0 L held, bit1 R held | RE-EXW-INPUT [verified] |
| scroll snapshot | 0x4eddcc | = mouse flags snapshot | RE-EXW-INPUT [verified] |
| game cursor | 0x4eddc4 / 0x4eddc8 | clamped x/y | RE-EXW-INPUT (CursorToGame@0044b428) [verified] |
| ESC/any-key latch | 0x4edb50 (g_input_seen) | u32 | RE-EXW-INPUT latch table [verified] |

**EXD aliasing**: every row above carries an `exd_addr` field in the registry,
filled by W1 (EXW→EXD address map). Until W1 lands a row's alias, that row is
O2-only (EXW verbatim) — the schema makes the gap explicit instead of
guessing. The B2 skeleton's ghost-fabrication lesson applies: no unanchored
address ever enters the registry.

## 5. Scripted input & command injection

[derived from verified seams] All injection happens at the frame trigger,
between the previous frame's present and this frame's input read — one script
step per frame, applied as raw guest-memory writes:

1. **KEYSTATE** — write g_keystore bytes (+ cursor/mouse flags for pointer
   paths). Covers camera/scroll (cursor position + drag), hotkeys, volume,
   any-key continues. The engine-side equivalent is the same script step
   mapped to an InputFrame — one script drives both sides (W5/W6 share the
   grammar; parity_harness's `step <frames> [buttons] [mouse] [dx] [dy]`
   extends with `keystore <scan>=<0|1>` lines).
2. **ORDER** — write order target 0x4dd484/88/8c + the per-robot move-target
   words (the exact FUN_00410644 outputs, §8 "click order target" row),
   skipping the click/pick UI. This is how S2 moves robots deterministically.
3. **COMMAND record** — write a record at 0x4dd4a0 (stride 0x80, count cell
   0x46cbe0++): flags byte@+5, weapon id — the FUN_00449c94/0x4dd4a0 route
   the queue item pins for weapon fire (7j.22): **never raw input**.
4. **PAD step-on** — an ORDER whose target is a .PAD slot tile (extraction
   arming per §7j.20: FUN_00433980 reads the pad, arms the beacon) — the
   sanctioned extraction trigger, not a click.
   **OP FORM (W5-pad, D86):** the target tile is READ FROM THE PAD BANK AT
   CAPTURE TIME, never baked from the .PAD file at compile time — the
   staged mission decides which slots exist. Runtime record layout
   (FORMATS §10 / RE-EXW-SIM §7j.16, both [verified]): 999 slots × 8 B
   at EXW 0x4e44f8 / EXD 0xf63c, `{u16 active@+0, u16 x@+2, u16 y@+4,
   u16 z@+6}`; the loader marks every parsed slot active=1 and stops at
   x==0xFFFF. The op: read slot `bank+slot·8`, VALIDATE (active==1 AND
   x!=0xFFFF — fail loud naming the slot, so a scenario targeting a
   slot the staged mission never loaded is a capture error, never a
   silent garbage order), then write {x,y,z} as three i32-LE words to
   the order-target triple (tile coords — the shared-grammar contract;
   the E side's order seam compares robot TILE positions, and the
   beacon armer itself takes `pos>>13` tiles). The robot's arrival on
   the tile is what arms extraction (FUN_00433980 → FUN_004247b5) —
   the op writes only the order, the game does the rest.
5. **BOOT setup** — difficulty 0x46cbf8 / mission-selection state written at
   scenario start (pre-mission), where a scenario needs non-default settings.

Determinism notes: with no injection the original polls zeros (keystore
memset at screen entries) — matching a null InputFrame on the engine side;
the only poll-time divergence class left is host-volume-key handling, which
scripts simply never touch. Mouse deltas are per-frame host business in the
original (D26 already derives engine actions per tick); the runner writes
absolute cursor positions, so both sides see identical absolute state.

## 6. The differ (W7)

**Normalization.** A field map per watch row converts raw guest bytes to
canonical records (`robot[i].pos_x_q13`, `projectile[j].type`, …). The engine
emitter produces the same canonical records from MissionSim/MissionScene per
tick. The differ NEVER compares raw bytes across implementations — only
canonical records — so layout differences are diff *findings*, not false
negatives.

**Comparison modes by parity class (PLAN §0b):**

| class | fields | rule |
|---|---|---|
| STRUCTURAL | record counts, bank occupancy runs, statics hashes, loader outputs | exact — the structural-error catcher |
| T1-exact | game-rule state: hp/score/money/state enums, damage results, pickup cases, objective statuses, platform hp | exact after alignment |
| T1-timing | frame indices of discrete events (release, destroy, arming, arrivals) | exact frame or ±1 (interleave class), reported |
| T2-tolerant | positions in-tick, draw-fed counters (frame counters, blink phase) | quantized tolerance; report-only |
| T3-statistical | RNG streams, debris jitter, spawn ordering | compare draw COUNTS and outcome distributions over the scenario, never bit-streams |

**Alignment.** Frame-indexed via g_frame_count ↔ engine tick counter. Where a
scenario's anchor event shifts frames between builds (e.g., pod release), the
differ aligns on the anchor event and reports the offset as a finding.

**Divergence classes & triage:**
- `engine-bug` — canonical semantics differ from EXW canon (O2 arbitrates).
- `original-divergence` — EXD≠EXW (log to docs/DIVERGENCES.md; engine keeps
  EXW; expected in hardware-coupled categories only — if it appears in pure
  game rules, that is itself a finding to investigate).
- `watch-artifact` — dump/injection artifact (re-run to confirm; the runner's
  double-run digest check catches nondeterministic captures).
- `accepted-T3` — budgeted divergence (statistical fields); feeds the meter.

**Report** (committed fingerprints only): per scenario — divergence meter
(counts by class), first-divergence {frame, watch, field, both values},
event-timing table, and the dump-chain digests for both sides. A regression
is any class-count change other than `accepted-T3` noise (statistical bands
pinned per scenario at first green).

**LANDED 2026-08-22 (W7, D87)** — `tools/diffharness/src/differ.rs` +
the `dbx-diff` CLI implement §6 as refined below; RE-EXD-MAP §8 is the
O1 field-map contract.

- **Normalizer**: E parses the §6a canonical grammar; O1 converts raw
  guest bytes per the RE-EXD-MAP §8 map (only individually pinned EXD
  robot offsets; scalars/beacon-widen/span/map-wh identity forms; the
  typedb len-0 ≡ all-zero-grid equivalence); O2 uses the RE-EXW-SIM
  §3/§7f/§7g EXW table (the seed-#1 EXW-front conflict is OPEN — W11
  arbitrates); rows/fields a channel cannot source are simply ABSENT
  from its normalized output (coverage, never fabrication).
- **Modes**: `double-run` (O1 vs O1 — the DH-G1 verdict instrument:
  identical modulo the frame-counter T2 + rng T3 classes) and
  `cross-channel` (per-field classes + O2 arbitration: O2 sides with
  O1 → `engine-bug`; O2 sides with E → `original-divergence`; no O2 →
  provisional `engine-bug`). T3 rows never bit-compare — the DRAW-COUNT
  (state-change census) is the statistical gate.
  **Arbitration lanes GATED (2026-08-23, W11-prep):** all four
  `compare_field` T1-exact tiebreak lanes are driven headless by
  `differ_gate` (`s1_o2_tiebreak_arbitration`, commit 4591f52): the
  O2 side is
  FABRICATED from the same E frames through the existing `inv_frame`
  inverse (valid because `normalize_o2_row`'s alias list takes EXW
  guest forms identical to EXD for every aliased row, and
  `EXW_ROBOT_MAP == EXD_ROBOT_MAP` — the §8 back-half probe; the O2
  `static-map-wh` row is W11-PINNED (D137, 2026-08-23; arithmetic
  CORRECTED by D138, 2026-08-24): the EXW cells are ADJACENT u32s
  with w LOW (0x4eddec/0x4eddf0, 4 apart — D137's "0x24 apart" was
  an arithmetic impossibility for these cells), so the O2 capture
  form is the 8-byte span @0x4eddec (w@+0x00, h@+0x04) — NOT the
  EXD 0x30 span (h LOW, 0x2c apart), so the fabrication is
  channel-aware since), the E side is
  re-stitched as a Channel::Engine dump when perturbed. The four
  lanes assert class+detail verbatim: (a) O2 agrees with O1 on a
  perturbed-E `money` → `engine-bug` "the engine (E) is the outlier"
  (verdict FAIL); (b) O2 agrees with E on a perturbed-O1 `money` →
  `original-divergence` "EXD diverges from EXW" (verdict back to
  PASS-WITH-NOTES — the re-class is budgeted); (c) all three differ →
  `engine-bug` "E wrong against both oracles"; (d) no tiebreak dump →
  `engine-bug` "provisional". Verified with NO production change —
  the lanes held as-written (W11's live channel inherits a proven
  arbiter).
- **Class policy refinement**: a `coverage` bucket beside `structural`
  — row/field coverage asymmetry (the E-gap list, the §8 normalizer
  gaps) is metered + reported, never silent, but NOTES the verdict
  (it moves only when coverage deliberately moves); structural VALUE
  mismatches (record counts, statics bytes, injection schedule,
  draw counts) fail it. A constant anchor shift ≤8 is applied and
  reported as a T1-timing note. Verdicts PASS / PASS-WITH-NOTES /
  FAIL; exit code non-zero only on FAIL.

### 6a. Canonical record grammar (W6 — the E-side field map, D85)

The canonical record is the CONTRACT both sides serialize into the W3
watch blobs: the E emitter writes it directly from engine state; the W7
normalizer converts O1/O2 raw guest bytes into the same grammar per the
registry row layouts. Little-endian, no padding, fixed field order per
row. Emitted rows only where the engine has a defined field map — every
other registry row is an explicit **E-gap** (the differ reports it as
STRUCTURAL missing-on-E, never silently skipped).

| watch id | canonical bytes (E) | source engine state |
|---|---|---|
| frame-counter | u32 | mission frame PRE-increment at the tail (`sim.frame()−1`; O1 reads the same pre-increment word at its tail — the registry row's dump-point ordering anchor) |
| rng-state-a | u64 | MissionSim PCG32 raw state (channel-native state word; T3-statistical class — never bit-compared) |
| rng-state-b | u64 | MissionScene RandB-stand-in PCG32 raw state (same class) |
| score / money | u32 / u32 | `MissionScene::campaign()` (0 / 3500 fresh — the §7j.64/C name-entry seed 4000−500·d at the boot-default difficulty 1, **S0-12b/D154**; `boot difficulty=d` overrides d and re-seeds through the engine's own `menu::start_score`; the pre-seam fresh value 4000 assumed the mis-modeled d=0 default) |
| difficulty | u32 | the session BOOT value (engine difficulty producers unmodeled — the record carries the scalar). **Default 1 since S0-12b/D154** (§7j.64/A: the GameMain boot head writes DIFFICULTY := 1 at 0x41c14a — the fresh-session default; `boot difficulty=d` overrides, and the scalar seeds the sim's difficulty-scaled damage rows §7j.15/1, so the S5C/S8 lanes ride it) |
| zone / mission / mode / linear-mission-m | u32 ×4 | host episode slot (`mission_slot()` / the §7j.64/D DERIVED cell); mode = 0 (SP, engine-modeled constant). **ZONE CONVENTION (D108):** E's zone is the 0-based mission-slot INDEX (0..6); the guest cell (EXW 0x4edd8c / EXD 0x107500) is 1-based set (zone_index+1, D99) — the O1 normalizer maps `cell−1` so both channels canonicalize to the index. First exercised by S5/S5B (zone 1 = ZONEB). **LINEAR AMENDMENT (S0-12b/D154, superseding D108's "linear stays the fresh-slot 0" note):** `linear-mission-m` is the guest's DERIVED cell [0x46ae8c] — `clamp(5·(zone−2)+mission−1, 1, 26)` recomputed by GameMain from the CURRENT slot every episode (0x41c520..0x41c556, §7j.64/D), NEVER the E episode progress counter `episode().linear()` the pre-seam code emitted; fresh/staged slots carry m from the derivation (S0 fresh (1,1) → 1; ZONEB/M1 → 1). The TRT hp tier selector (destroy staging) reads the same derived cell. A played campaign's own counter stays a live-capture seam, never an E fabrication |
| sfx-master-gate | u32 | **constant 1 (D136)** — the E engine's sound-on construction assumption (no audio config model; every dispatch the gate guards is presentation-tier). A capture machine with sound DISABLED dumps 0 → the intended loud finding; the D134 fingerprint companion (one dbgprobe read of [0x10743c] at the anchor stop) is the remedy, the D128 ACTIONPAN pattern |
| robot-bank | u32 count + count records; record = the modeled Robot field list in the `state_hash` order: alive u8, pos_x i32, pos_y i32, z i32, state u16, dir_byte u16, facing u16, anim u16, variant u16, probe_z u16×8, stop_dist i32, target_present u8, target_x i32, target_y i32, drop_countdown i32, hp i32, armor i16, hit_flash u16, alarm u16, kind u16, shield i32, shield_charges i32, shield_boost i32, battery i32, armor_pool i32, alarm_ctr i32, death_flag u16 | `MissionSim::robots()` |
| selection-triple | u32 selected idx only (the D83 anti-fabrication precedent: the alias-covered cell; cursor/squad join when their engine models + EXD aliases land) | `sidebar_selected()` |
| blink-cursor | u32 (0 or slot+1) | `sidebar_cursor()` (the 7j.6 select-ack selector). **Cross-channel note (D132, aligned 2026-08-23):** the O1 capture plans dump the EXD twin cell 0x10e108 (plain 4-B u32, `Form::Fixed`) — the O1 normalizer carries the named u32 arm so the row compares CLEAN cross-channel (no E-only finding; the differ_gate fabricated O1 side fabricates it identity, the D136 sfx precedent) |
| per-player-selected | 4 × {x i32, y i32, z i32} (player 0 = selected robot pos>>8 Q5 + z; 1..3 zero) | sim + sidebar |
| order-target | i32 ×3 | the ORDER-seam write (last injected target; the 0x4dd484 cells persist, so does the E session value) |
| move-target-words | u32 count + per-robot {present u8, tx i32, ty i32} (Q5, same units both sides — EXD writers are `tile<<5`, D90; the O1/O2 side dumps the 0x60-B EXD span and the differ SPLICES the trio into the robot-bank row, so this row stays E-only in cross-channel reports) | `Robot::target` |
| beacon-family | flag u32, timer u32, tile i32×3 | `MissionSim::order()` (window = timer 0x197) |
| spread-claims | u16 ×12 | `Order::claims` |
| no-extract-latch | u32 count + count u32 (all zero) | **(D136)** count = the robot-bank count (`robots().len()`); the latch is MP-lobby-claimed ONLY (D133) — never set on any SP path, so E's SP corpus construction is the all-zero bank (the guest boot-memset twin). The O1/O2 raw side is the bare `$robot_count*4` span — the normalizers prepend `len/4`; the count field is STRUCTURAL like every count word, so the robot-count scenario seams (D91/D103/D108 `_e_staging`) surface here exactly as on robot-bank.count |
| typedb-fade-byte, armor-pad-reads | u32 len + len bytes (the engine bank is lazily materialized; len 0 ≡ all-zero w·h — the ZONEA corpus until a death) | `armor_pads()` (the +0x18 byte family, 7g.3/7j.9) |
| static-map-wh (TS, anchor frame only) | u32 w, u32 h | terrain/view size |
| static-claim-bank (TS, anchor frame only) | the RAW 10000-B arena image — no count prefix, no field map (**S0-11b/D151**) | `MissionSim::claim_bank()` — staged at EVERY `load_mission` by `stage_claim_bank` (the §7j.63 door-rect stamp; the original's unconditional 0x447b85 initializer). The O1 plan dumps the same fixed span through the 0x119564 pointer cell, O2 through 0x46af58 → byte passthrough on all three channels (the static-map-wh fixed-extent precedent) |

**E-gaps (rows the E side does not emit in W6):** variant-flag-bytes
(T1); mortar-trail-bank + poi-bank (T2); the unmodeled T3 banks
(rising-debris, blast-bank, arrival-rides, door-rects,
trigger-timers, pod-ring, exit-ring, objective-slots,
escape-counters, tile-claims); s0-trigger (S0); every TS row except
static-map-wh (the engine parses the volumes into internal forms and
does not retain raw bytes); all T4 (event capture); all TI (the E
injection surface is the scenario step list, not watched keystore
bytes). **AMENDED 2026-08-23 (D136):** sfx-master-gate and
no-extract-latch LEAVE the list — E emits both now (the two table
rows above; the W6-followup to D133/D134). The same amendment
corrects the list for staleness, history preserved: the D85-era list
still named the destroy-family five (tile-word-grid,
platform-strength, typedb-mirror-rows, object-instances, trt-array)
and "all T2/T3" as gaps although W12-S3/S4/S6/S8 landed their
emitters (gated on the scenario staging keys — `destroy = 1` / T2
tiers / `pad` steps / `critters = 1`). **AMENDED 2026-08-25
(S0-11b/D151):** `static-claim-bank` LEAVES the "every TS row except
static-map-wh" clause — E emits it now (the table row above; staged
at every `load_mission`, the §7j.63 door-rect stamp). The per-frame
T3 `tile-claims` row REMAINS a gap (the TS row carries the load image;
the original's bank is mission-static — no writer after the
initializer — so the two rows are content-identical by construction
and the T3 row stays deferred with the other T3 banks).

**Frame model (E):** one MissionShell-equivalent frame = one
`pump_frame(dt=4)` = `MissionScene::tick` (six phases + epilogue) +
`present` (the render epilogue runs `rand_b` churn — the real engine frame
rhythm, so the dumps represent genuine engine frames, not stripped-down
ticks). The ANCHOR record = the tail of the FIRST mission tick
(`frame_no` 0); the dump point is after `present`, before nothing — the
engine has no separate flip. Records then run `frame_no` 1.. strictly
increasing; total = anchor + `frames` (the stitcher contract). Audio is
deliberately not pulled (state-only harness, §0).

**Scenario step consumption (E, the D82 shared seam — the same
`diffharness::runner` parser):** walk phase may carry ONLY `boot`
steps (any other walk step — the S0W menu-walk shape — is rejected
naming the P2e InputFrame button bit-map seam; the grammar pins boot
to the walk phase, so the difficulty seed stays reachable).
Mission-phase steps: `step N` = N null-input frames;
`keystore` = the seam maps scans → `InputFrame` — the pinned map is EMPTY
in W6 (no engine keyboard consumer exists; P2e), so steps mark the frame
injected and deliver the null frame; scan 0x19 (P-pause) is rejected
outright (§2 pause rule). `order x y z` = the click-order seam: record the
target triple (order-target row) and arm via `arm_order_at_robot` at the
alive robot whose tile == (x,y) (the E pick form — tile-exact; the EXW pick
is the 0x20-px screen-distance twin, a documented seam approximation W8 can
refine); no robot at the tile = the pick fails, target recorded. `command`
and `pad` are REJECTED naming the missing engine seams (the fire family and
extraction arming are not modeled — S3/S6 pair with their engine producers
per §10-W12). `boot difficulty=d` OVERRIDES the fresh-session
difficulty default 1 (§7j.64/A, the GameMain boot write 0x41c14a —
S0-12b/D154) = the campaign money seed (`set_campaign(0, 4000−500·d)`,
applied on EVERY run; the default d=1 seeds 3500) + the canonical
difficulty value. The dump
is produced by the same `runner::stitch` validation + `encode_dump` path as
O1 captures (channel E, build_sha256 = sha256 of the engine identity
string, pins = seed/dt/difficulty/zone/mission) — identical bytes given
identical state, by construction.

## 7. Scenario corpus (build order; each names its hypotheses)

| id | scenario | script sketch | tiers | arbitrates / proves |
|---|---|---|---|---|
| S0 | boot→mission | no injection; run to ZONEA/MISSION1 first frame | T0 + statics | loader parity (statics hashes); frame-trigger stability; the W1 EXD map on real memory |
| S1 | mission-start passive | no injection; record ~400 frames | T0/T1/T3 | **pod-descent stagger** (watch pod timer w@+0x2C ≡ 0x4c6a10 vs the `1+k·(2000−m·1000/27)` formula, pod ring phase→release, descent ≈41 frames, pod phase 2 = one tick, release = state 6 — §7j.20/§7j.27); **blink-cursor-from-spawn** (does 0x4dc5d0 go nonzero with no click, from which frame); **debris 2k start-delay** (which 0x476fbc +0x24 values actually get staged on the corpus path — is any ≈0x7D0?) |
| S2 | order→walk (the P4 slice) — **LANDED 2026-08-22 (D91)** | ORDER steps moving one squad member across ZONEA/MISSION1 (mirrors engine/bedlam-core/tests/mission_corpus_gate.rs; the walk needs a second robot — the `markers` staging key below, D91) | T0/T1 | slice field parity (positions, arrival snap, spread claims, move-target words); the engine's biggest existing seam |
| S3 | weapon fire family | COMMAND records: one per weapon class (bullet 2..4, shell 5, artillery 9..0xB, ballistic {0xE,0xF,0x13,0x17,0x1A,0x1F}, rocket 0x24, homing 0x29) at fixed targets | T0/T1/T2 + T4 | fire cadences, damage application (FUN_00419aff table), bank record lifecycle; the corpus-off weapon producers; T4 SFX events seed the SFX-family walk |
| S4 | destroy family — **LANDED 2026-08-22 (D105)** | S3 fire onto destructibles (tile 0x62 traps, platform 0x7d4, chainable objects) | T0/T1/T3 + T4 | destroy resolver → terrain restore → 5-effect loop → chain walks end-to-end (§7j.25); **five-ring overlap read** (0x4796d4 bytes around overlapping corpse rings — statically mooted by §7j.10's ≤7-frame fade; the harness read is the confirming observation); debris producer kinds/delays (stager widening input) |
| S5 | pickups & pads — **LANDED 2026-08-22 (W12-S5, D108; producer W12-S5-prep §7h.5)**: grammar v1.5 keys `zone = "B"` (the episode-slot host seam — the campaign-advance/save-load shells the host stands in for, D51 pattern; mission implicitly 1 via mask 0, ~~linear 0~~ [the emitted row is the D154 derived cell — 1 on ZONEB/M1]) + `pickup = 1` (stage the mission's OWN .TOT through `stage_pickup_surface` AFTER any destroy staging + the §7j.12/6 hazard stamper, the original's load order) — S5/S5B run ZONEB/MISSION1 (set 2) with destroy staged too, so the typedb-mirror-rows go REAL (S4's recorded empty-mirror divergence closes for S5-class scenarios; the S4 chain itself is untouched — S4 sets no pickup key) | TWO short walks (the order-window constraint forces the split: a second `order` needs the first cleared — all-alive-state-3 or the 0x197-frame window expiry, and 407 idle frames × ~340 KB/record of REAL mirror rows is not a shippable dump, D108). **S5 = the row-21 z3 corridor** (the only spot in the corpus where cases 1 and 2 co-occur walkably: cells (26,21) c1 w0x76, (27,21) c2 w0x7e, (28,21) c4 w0x82 — clicker marker (28,21,3), walker marker (25,21,3), `order 28 21 3` → slot-1 target (29,21); the walker collects c1/c2/c4 at frames 1/2/4 and arrives frame 5). **S5B = the row-10 z3 corridor** (case 3 + 4× c4: cells (74..78,10) w0x83/0x83/0x7b(c3)/0x83/0x83 — clicker (78,10,3), walker (73,10,3), `order 78 10 3` → slot-1 (79,10); consumes all five incl. the diagonal probe reach at (78,10) from (77,9), arrives frame 12). ORDER_RADIUS staging note: the claim needs the walker within 0xC0 Q5 of the ORDER TILE CENTER — the marker's +0xF00 spawn offset makes a 6-tile gap read 0xC1 (rejected); keep walkers ≤5 tiles out. Case-3 observability note: the walker spawns hp 5000 (the clamp ceiling), so the c3 body's +2500 is value-invisible in S5B — the consume + dispatch still ride the mirror/T0 rows; **the pre-damaged-walker variant LANDED as S5C (W12-S5C, D110 — §10-W12): the S4 artillery pattern spends the walker to 1256 pre-order, case 3 heals the exact +2500 → 3756 unclamped** | T0/T1 | pickup_case dispatch vs the type-DB mirror rows (7h.3/§7h.4): the consumed cell's mirror word (:= table-C floor 0x48F) + seen (:= 1) + the case-4 score/money folds + the case-1/2 robot fields (drop_countdown 1000 / shield 1000). DAT-byte visibility ANSWERED (D108): the consume's DAT := 0 (collision-plane empty) needs NO dedicated row — the mirror word/seen carry the pickup observation and the walkability change rides the robot-bank rows (the walker crosses the cells it consumes mid-walk); no watch set carries the raw DAT volume (it is not a guest span) |
| S6 | extraction — **LANDED 2026-08-23 (W12-S6, D112; the §7j.40 decode + the engine extraction family by 631bd28/edafd02)**: the walk is COMMAND-driven, not order-driven — a click NEVER arms the beacon (EXW's sole producer is the pad-script armer, §7j.40/5) and E's `order` step arms the beacon directly, so the trigger can never fire under a pending order; a COMMAND bit0 SELECT record gives the ORIGINAL's own walking robot (state 1 + move-target, no order) that the dispatcher dual-gate accepts | **S6 = the zone-1 census GROUND pad**: `pad 18` = slot 0x12 = (19,70,0) — the queue's `pad 8` gloss predates the verified census (slot 8 = (2,14,1) is a LEVEL-1 pad; a ground robot can never match z>>5 = 1, and (5,61,0) is slot 0's record; D112 records the deviation). Two COMMAND legs stage the crossing: west into tile (19,73) (terrain-probed: (20,70) east of the pad is blocked, column 19 open y67..y73), then due north THROUGH the pad mid-walk — the sub-tick probe fires with state 1 + target present, the armer halts the walker state 3 snapped at the beacon tile, and the same frame's MissionShell beacon block DEPLOYS (the single-MRK-robot window-0 expiry; with 2+ alive only the all-state-3/dead early expiry could). 75 records, chain c96f0735df1059ea, the full timeline pinned (descent → landing sweep state 3→5 → the RandA-jittered dwell → the group-scaled departure drift → the f69 completion freeze) | T0/T1/T3 | **arm-extraction via scripted .PAD step-on** (beacon family, exit ring phases, dropship deploy — §7j.19/§7j.20/§7j.27 LANDED as observations; the objective counters stay the §7j.40/7 script-objective E-gap); the beacon-family row's post-deploy form = the SURVIVING tile/claims latch (FUN_0041faf0 clears only the flag/window pair); the dropship-frame T3 row is E-only (no EXD alias — a coverage finding, never fabricated) |
| S7 | platform dynamics — **LANDED 2026-08-23 (W12-S7, D113; the §7j.41 decode + the engine platform-dynamics family by 984a078/ea2f259 + the scenario leg b9cbcf3)**: the whole lifecycle in ONE ZONEA/MISSION1 run — the grammar v1.6 `platforms = 1` arm key runs the epilogue creep tick from frame 0 (the ORIGINAL calls it unconditionally, one gate RandA per frame even unarmed — the E-side stream gap on S0..S6 stays until a deliberate re-baseline; O1 needs no staging, the rng-state rows are the budgeted channel-finding class) | **the trigger site**: ZONEA/M1's zone-1 code is 5 and the mission banks exactly ONE type-5 instance — .POS slot 74 @ (3,57,2), hp 75 — the marker stages the GUNNER on that tile; the frame-1 artillery 9 destroys it at f32 and FUN_00422600 builds the strength-300 ring at the dying instance's OWN tile (five build; the gunner's quadrant blocks three; the same burst's pair-7 destroys the fresh (4,56) — the first k7 debris). **the weaken/spread instrument**: four pre-build grenade volleys (f18–f30, aimed (2,54)/(3,54) so the overshoot rests on the field's north edge) detonate f32–35 ON the fresh platforms — two 75-hits take the 300s to 150 (the corrected §7j.41/3 ring gate old ≥ 200 ∧ new < 200 fires → the 150-rings build the north row + rebuild (4,56)), two more reach 75 (the second gate old ≥ 100 ∧ new < 100; the site latches), the next hit destroys the spent tiles (5× k7 each; 20 k7 total by f35). **the creep**: the armed 1/32 tick extends the bridge from the f34 site latch — the first 199 tile at f449, 22 creep tiles by f1240, the tail static (27 field tiles = 22 creep + 5 survivors). 1361 records, chain b41db389f3ad8947, corpus_s7 gates the full timeline (the water z-word 0x25D + seen-0 volume-2 semantics pinned) | T0/T1/T3 | platform family field parity (§7j.12 + the §7j.41 corrections): the trigger dispatcher (id == the zone code, build at the DYING instance's record, strength 300; zone 3 keyed by the within-zone mission number), the weaken ring gate (old ≥ 200∧new < 200) ∨ (old ≥ 100∧new < 100), the site latch on the weaken→ring path ONLY, the ring gates (both banks + claim + live-robot-quadrant + z ≥ 1 + empty word + plane-A 0 + plane-B(z−1) volume 1), the creep protocol (the per-frame gate draw + 2 jitters + the water ray walk + the step-back tip ring at 199), the volume-2 stage_platform write (seen 0) |
| S8 | critter engagement — **LANDED 2026-08-23 (W12-S8, D114; the §7j.42 decode by f9af5743's b3e78cb/05f0d95 + the engine leg 8786c9e + the differ/plan/docs legs this run)**: the grammar v1.7 `critters = 1` staging+arm key (the .NME through the FUN_00416458 §7j.18 spawn schedule + the controller FUN_00412f34 ARMED — the ORIGINAL loads .NME natively at every mission load and runs the controller UNGATED, so the loader's kind-4 heading draws + the per-frame controller draws are the budgeted E-side stream gap on unarmed scenarios, the D113 pattern) | **the engage instrument**: ZONEA/MISSION1 is the corpus zone that hosts critters (16: the 6 kind-5 at (18,9)/(18,8)/(18,7)/(7,8)/(7,7)/(7,6) z1 + 10 kind-4 seek-steppers); the marker stages the GUNNER at (18,13) — the FLAT floor-31 row south of the 95-plateau at row 11/12 (a plateau marker puts the artillery burst one z-level high and the §7j.23 z-box misses every critter) — 4 tiles from the (18,9) pack: the pack approaches under the juice draw, crosses the [0x60..0x80] transition band within frames, and 2-3 critters run the mode-2 fire cycle (the 3-D-aimed 0x68 projectiles into the ALIASED projectile bank; the odd-pass FUN_004197d4 walker applies 75/hit to the gunner). **the death instrument**: the gunner's slot 0 = artillery 0xB (the 7-frame ring-coverage burst); the frame-1 COMMAND bursts f32-38 and the script-blast 0xC lane kills the approached pack + the walked-in kind-4s (9 dead by f39) — the §7j.24 handlers run the weapon-gated debris + the 8/12-row effect-row bursts (the 0x4cec38 LRU bank turns over all 80 rows), the mode-6 dives, the 0x28 dying counters, the 1500-frame dormancy. The bounty gate stays DARK on the corpus path (script kills carry attacker −1; robot-owned kills need the bullet family — the S3-documented AI-order E-gap). 121 records, chain b5ae3f8be91c7449, corpus_s8 gates the lifecycle | T0/T1/T2/T3 | the §7j.43 corrections ride: the d=2 never-rolls break, the impact-aimed reversed-step dives, the wake/rise same-substep re-dispatch, the point-blank RETREAT band (<0x60, not a freeze), the approach band's aim+facing (no +0x80); the critter bank + effect rows are E-ONLY coverage rows (no EXD alias); the 0x68 fire + the robot damage/knock lanes + the RNG draws are the ALIASED observables |

The **mid-flight draw blit sequences** (7j.28) are render-side and stay OUT of
state diffs (T2 perceptual per §0b — they belong to frame goldens,
DESIGN-RENDER). What S3 DOES watch is their input data: the two projectile
banks + trail bank + draw counters (d@+0xE wrap, tick fields), which pins the
draw-relevant record semantics without pixel diffing.

**Per-zone case tables** (FUN_00433980, the ~25 extraction pads, the
0x424a6f message table) are decoded on demand as scenarios need them (S6
first) — the harness is what makes that decode mechanical (watch the armed
records instead of guessing the case).

**Extraction-pad census (S6 authoring data; §7j.20 item 2
[verified-mechanical-parse], the exact per-zone table stays the deferred
decode):** the .PAD slots whose pad scripts call the beacon armer, per
zone value — zone 1 {8, 0x10, 0x12, 0x18}, zone 2 {4, 5, 7, 0xE, 0x11},
zone 3 {0, 1, 6, 0xF, 0x15}, zone 4 {0, 2, 0x10, 0x15, 0x16}, zone 5
{8, 9 ×2, 0x3D}, plus the shared pad-switch tail (slot 6) for the zones
that reach it. S6's default ZONEA target = zone 1's set. The `pad <slot>`
step addresses slots by bank INDEX (slot i = runtime record i at
0x4e44f8/0xf63c; file record order is preserved by the loader), so this
table is the scenario author's slot picker — the op's runtime validation
(active==1, x!=0xFFFF) still guards against a wrong zone/slot pairing.

**Scenario staging key `markers` (D91, grammar v1.2):** the click-order
walk moves only the OTHER robots inside the order radius — the clicked
robot snaps to spread slot 0 (its own tile) and nothing else walks — so
any order→walk scenario needs a second robot. The SP squad rule stays
pinned at the zone table (D89: no override, ZONEA banks 1 robot on
EXW/EXD/E alike), so walk scenarios carry `markers = x,y,z` (a
`;`-separated list of extra squad markers, staged AFTER the MRK robots,
bounded so MRK+markers ≤ 12 = the bank cap). On E these stage through
the EXISTING `load_mission(staged_markers)` host seam (the same seam
mission_corpus_gate exercises via `sim.spawn_robot` — no staging-rule
change). On O1 the key is an E-side staging seam with no O1 write:
dbx-plan records it in the plan's `_e_staging` field (the registry gap
discipline applied to staging — named, never fabricated), and the live
O1 capture of a markers scenario banks the MRK squad only, so its
robot-count diff vs E is the recorded scenario seam, never a finding.
S2's marker = `18,73,1`, the mission_corpus_gate walker.

**Scenario staging key `loadout` (D103, grammar v1.3):** weapon-fire
scenarios need armed robots, and NO engine path reaches the weapon slots
(the original fills them at spawn from the session stat table — the D51
host-seam pattern). `loadout = idx,mask,id:ammo[,...][; ...]` stages
per-robot slot ids + the enable mask through the EXISTING
`stage_robot_weapons` host seam — the same discipline as `markers`:
an E-side staging seam, recorded never fabricated (O1 arms the same
slots by playing the real game; the differ never writes a bank).
Bounds mirror the original structures: robot idx 0..11 (the 12-robot
cap), mask 0..0x7F (7 slots), ids in the 2..0x28 dispatch domain,
positive i16 ammo, no mask bit beyond the staged list (auto-rearm never
arms an empty slot). S3's loadout: robot 0 (the MRK robot) with one
slot per INLINE-spawn class — artillery 9/0xA/0xB, prox mines 0x10
(→2× type 0xF), pressure mines 0x14 (→2× 0x13), bouncy grenades 0x1B
(→4× 0x1A), sticky 0x1D (→4× 0x1F) — and robot 1 (the `markers`
walker) with the rocket 0x20 (→1× 0x24).

**Scenario staging key `destroy` (D105, grammar v1.4):** the
destroy-family scenarios need the mission's destructibles staged, and
E's `load_mission` does not fetch them (the original loads the
mission's own .BDG type table + .POS instance list + .TRT structures
natively at mission load — FUN_0041a4f8 + FUN_004170a6, §7j.25/4).
`destroy = 1` (strictly `1`, once per scenario — a typo'd value fails
loud at the grammar, never silently skipping the staging AND its dump
rows) stages all three through the EXISTING `stage_destroy_family`
host seam and gates the destroy-family dump rows (they ride only
destroy scenarios — S0..S3 pinned bytes untouched). Unlike
`markers`/`loadout` this is an **EQUIVALENCE seam**: the staged
CONTENT is byte-identical to what O1 loads (no O1 write exists to
fabricate — dbx-plan records the key in `_e_staging` with the
equivalence note), so the destroy rows compare directly on a live
capture. The one recorded divergence: E's TOT-mirror/seen banks stage
EMPTY (the `init_tiles` TOT fill is the S5 pairing — §7h.4/D99), so a
live O1 mirror-rows diff before S5 is the staging seam, never a
finding. S4's staging: ZONEA/MISSION1's own files (211 live
instances, 3 turrets).

## 8. Open hypotheses ledger (what this doc does with each)

| hypothesis | source | disposition |
|---|---|---|
| pod-descent stagger formula + release semantics | §7j.20, §7j.27 (static decode) | S1 watch confirms/refutes numerically; engine modeling follows the observation |
| weapon fire needs COMMAND records, not raw input | 7j.22 (queue note) | adopted as the injection design (§5.3) — S3 validates |
| destroy family end-to-end | §7j.25 (decoded, corpus-off) | S4 is its first live observation |
| mid-flight draw blits | §7j.28 (decoded) | out of state-diff scope (T2); S3 watches the record data (§7 note) |
| debris 2k start-delay | queue phrasing at da4bf20 (after §7j.7/7j.8, the +0x24 start-delay field discovery) | S1/S4 record every staged +0x24 value on real runs |
| blink-cursor-from-spawn | §7j.7 item 6 → census-closed §7j.59/D131: producer = the idle-bombardment arm only; from-frame-0 behavior now statically decidable (constant 0 — no corpus scenario reaches the idle threshold) | S1 watches 0x4dc5d0 from the first mission frame; expect 0 throughout |
| five-ring overlap last-write-wins | §7j.9, then §7j.10 declared it moot (fade ≤7 frames) | recorded as statically CLOSED; S4 takes the confirming byte-level read anyway (cost: one watch row) |
| arm extraction via .PAD step-on, not a click | 7j.20 (queue note) | adopted as injection design (§5.4); S6 validates |
| +0x18 armor-pad ring read semantics (raw byte ≠ 0 arms pads) | §7j.8 item 8 | already wired in-engine; S1/S4 regression-watch 0x4796d4 |

## 9. Gates

- **DH-G0 watch-proof** (interactive, one session): first interactive
  DOSBox-X run per the RUNTIME skeleton checklist verifies debugger command
  names, the linear-address conversion, and produces S0 dumps whose digests
  reproduce across two runs. Converts [pin-unverified] items to runbook facts.
  **UPDATE 2026-08-22 (W4 audit):** step zero of DH-G0 is now a CHANNEL
  RE-PIN — the pinned flathub binary has no debugger and log-only JS
  (RUNTIME.md "DH-G0 channel audit"); options (a) self-build with
  --enable-debug=heavy, (b) GameLink feasibility, (c) O2-ptrace-as-primary.
  The audit itself converted the §3 trigger UNCERTAINs to NEGATIVE facts
  (committed); the breakpoint shape + bulk-read forms get pinned on the
  re-pinned channel at the first interactive session.
- **DH-G1 runner-determinism**: headless S1 run twice → identical dump
  chains (same pin, same scratch corpus). No CI; desktop/local only, results
  committed as fingerprints.
- **DH-G2 structural parity (the P4 acceptance slice)**: S0–S2 STRUCTURAL
  mode green (loader statics + bank layouts/counts + occupancy shapes).
- **DH-G3 field parity budget**: S2 T1-exact green; S3/S4 T1-exact on the
  producer paths as they land in-engine; meter thresholds pinned per
  scenario at first green. **W9 CI leg (LANDED 2026-08-22, D92):** the
  corpus-gated test set runs in CI as a NAMED workflow job
  (`.github/workflows/ci.yml` `diffharness`: `cargo test -p
  diffharness` + `cargo test -p bedlam-game --test canonical_dump_gate
  --test differ_gate`). WHAT CI PROVES: (a) the harness set compiles
  on every push, (b) the SKIP-CLEANLY property — every game-data
  reader guards on `corpus_present()` and a test that forgets (the W9
  sweep found exactly one: `menu_gate`, 3 of 5 tests panicking on the
  absent corpus) fails the job, (c) the corpus-FREE tests inside the
  gated files (the synthetic §6a grammar fixture, the dump schema,
  the registry anchors, the stitch replay, the differ unit tests)
  run for real. WHAT CI DOES NOT PROVE: the pinned-chain corpus
  assertions (`dac1cfd17bc7ede3` / `a18cb11ac8e4314e` /
  `d6649ce272ad6d96` + the differ cross-channel verdicts — the
  D136 re-baseline: E now emits sfx-master-gate + no-extract-latch,
  every chain moved deliberately) — CI
  checkouts never carry game-data (it is never committed), so those
  run wherever a corpus is present (dev/operator machines run the
  same commands pre-push; the identical test names make the leg
  auditable). The LIVE session (DH-G0) is the separate,
  desktop-gated proof of the O1 capture channel; original-side runs
  (O1/O2/O3) never run in CI by design (pinned emulator).
- **CI wiring**: original-side runs never run in CI (pinned emulator,
  desktop-gated). CI runs the ENGINE dump emitter + differ against committed
  reference fingerprints (corpus-gated, skip when game-data absent — the
  mission_corpus_gate pattern). **LANDED 2026-08-22 (W9, D92)** — the
  named `diffharness` job above + the workspace sweep: every
  corpus-dependent suite (assets corpus/fonts/loading/smk×2, core
  mission_corpus_gate, render mission_view_gate, game
  boot_attract/brief/menu/music/title_playback/mission_scene/
  canonical_dump/differ) verified by a fresh corpus-free clone test
  run to skip cleanly; `menu_gate` (the one exception) fixed with the
  same guard. Re-verify by re-running the clone test when adding a
  new corpus gate.

## 10. Build-order tickets

Ordered; each is one bounded unit. W1–W2 unlock everything; the runner and
differ come before any new scenario depth.

1. **W1 — EXD import + EXW→EXD address map.** Import BEDLAM.EXD to the
   Ghidra project (LE/DOS4GW class like B2; check no import running first),
   pin the EXD present/frame-tail site (the S0 trigger), and build the
   address map for T0/T1 rows (anchor by string refs + call-shape + the
   pinned constants; every mapped row gets dual anchors; mismatches →
   docs/DIVERGENCES.md seeds). Deliverable: docs/RE-EXD-MAP.md + registry
   `exd_addr` fills.
2. **W2 — watch registry.** Commit §4 as a data file (tools/diffharness/
   watches.toml: id, exw_addr, exd_addr, extent, layout ref, tier, anchor
   ref) + a validation test asserting every anchor string resolves to a
   ledger row heading (mechanical anti-ghost guard).
3. **W3 — dump schema.** The versioned frame-record format + FNV-1a-64 chain
   (pure Rust, tools-side); encoders for raw watch blobs. **LANDED** —
   `tools/diffharness/src/{dump,hash}.rs` + `tests/dump_schema.rs`
   (grammar pinned in §3 + D78).
4. **W4 — DOSBox-X runner.** Extend tools/runtime/dosbox-harness.sh with a
   `diff` mode: scenario script → conf copy → debugger automation → D: dumps
   → digest manifest. First target: S0 headless; then S1.
   **STATUS 2026-08-22:** unattended-safe staging LANDED (diff mode with
   `stage`/`run`/`stitch` sub-operations; EXD corpus scratch at
   runtime/harness-corpus-exd; scenario grammar v1 in
   tools/diffharness/scenarios/; the `dbx-stitch` bin converting a channel
   capture transcript `DBXCAP` to the W3 dump + digest manifest; replay
   fixture tests pin the pipeline determinism). The live automation piece is
   [BLOCKED]-on-DH-G0-channel-repin (RUNTIME.md audit: no debugger in this
   pin; startup.js log-only).
5. **W5 — injector.** The §5 vocabulary as runner-side writes (script
   grammar extension shared with the engine emitter).
   **STATUS 2026-08-22 (D82):** LANDED + headless-verified. Grammar
   v1.1 steps (keystore/order/pad/command/boot; walk/mission phases
   split at until-anchor), the capgen SMV emitter (boot_writes at the
   arm stop + per-frame inject rows applied BEFORE the watch dumps,
   `frame N 1` injected flags in DBXCAP, the command-ring append op:
   count read → zero-extended record → count bump), and dbx-plan's
   count-cell compiler for T1 (robot/object/TRT count resolve rows +
   count·stride extents; S1.json committed + byte-pinned). The O1
   alias gates are HARD: every §5 seam row (keystore, order target,
   command ring, difficulty) is a registry gap — scenarios carrying
   those steps fail compilation naming the seam until the EXD input/
   command twins are pinned (the walk driver unit follows the keystore
   alias). `dbgprobe inject` proves the machinery headless (no game).
   **W5-walk ADDENDUM 2026-08-22 (D84):** the scripted MENU WALK
   driver LANDED + headless-verified (`dbgprobe walk`, no game). The
   four §5 seam aliases were closed by the input-twin census (D83), so
   walk-phase KEYSTORE steps now compile to stop-indexed plan rows —
   one stop per counter-writing screen frame (the BPLM-on-frame-counter
   boot trap doubles as the walk driver), SMV writes re-armed per input
   (AnyKeyWait consumes on read), mission-start detection = the anchor
   BP armed at the LAST walk stop. `resolve_at=anchor` moves the
   loader-static reads to the mission-start stop (they are mission-
   load values — the D81 arm-stop read was a latent gap, fixed for
   S0/S1 too). The S0W scenario + draft schedule are committed; stop
   indices calibrate at the first live session via `walk_watches`
   transcript comments. The PAD step keeps its own unit (the capgen
   runtime pad-slot read op still pending, deliberately out of scope).
   **W5-pad ADDENDUM 2026-08-22 (D86):** the PAD step LANDED +
   headless-verified (`dbgprobe pad`, no game). The capgen
   `{op:"pad"}` inject form reads the 8-B slot record from the pad
   bank at the capture-frame stop (MEMDUMPBIN through the bank's own
   SEG form), validates the loader marks (active==1, x!=0xFFFF —
   fail loud), then writes {x,y,z} as i32-LE to the order-target
   triple (§5.4 OP FORM). dbx-plan un-gates `Step::Pad`: the op
   row's bank comes from the `static-pad-slots` registry row (a READ
   anchor — its own explicit gap error, distinct from the write-seam
   rule) and the target cells from `order-target`; slot bound 0..998
   re-checked. The extraction-pad census (§7) is the S6 slot picker.
   The E side still rejects pad steps naming the S6 engine seam
   (extraction arming) — W12 pairs it.
6. **W6 — engine dump emitter.** parity_harness gains `--canonical`:
   per-tick canonical records in the W3 schema (MissionSim/MissionScene
   field maps for T0/T1 first).
   **STATUS 2026-08-22 (D85):** LANDED. `parity_harness --canonical
   --scenario <S.scen>` drives GameHost over the SHARED v1.1 scenario
   grammar (the D82 seam: same `runner::Scenario` parser as the O1 side)
   and emits the channel-E W3 dump via the same `runner::stitch` +
   `encode_dump` path as O1 captures. The canonical record grammar (the
   field-map contract W7's normalizer must match) is §6a above; E-gaps
   and the scenario step semantics (keystore/order/boot consumed;
   **`command` CONSUMED since W12-S3-prep (2026-08-22, §7j.37): the
   payload stages as a COMMAND record into the sim ring and the
   pumped frame's consumer fires the weapon dispatch — the fire
   family lives in `bedlam-core::weapon`; the S3.scen unit pairs the
   canonical rows**; pad + walk-phase still rejected naming the
   missing engine seams)
   are recorded there. Verified by a synthetic comparison fixture (the
   byte grammar + digests pinned) + corpus-gated S0/S1 runs (the
   mission_corpus_gate pattern; dumps stay runtime-only, chain digests
   pinned in the test).
7. **W7 — differ.** Normalizer + the §6 comparison modes + report writer +
   fingerprint manifest output. **DONE 2026-08-22 (D87)** — see the
   LANDED note in §6; the normalizer contract is RE-EXD-MAP §8, the
   O1/O2/E field maps live in `tools/diffharness/src/differ.rs`, the
   CLI is `dbx-diff` (RUNTIME.md "W7 the differ"). The move-target
   u16-word deferral is CLOSED by W7-followup2 (D90): the arrays are
   u32 ×2 (per-robot x/y by absolute id, −1 = none, Q5 `tile<<5`),
   the O1 plan row emits the fixed 0x60-B EXD span, and the differ
   splices the target trio into the robot-bank row; the pad op is
   modeled through the order-target rows it writes (D86).
8. **W8 — scenarios S1/S2 wired end-to-end** (first full O1↔E diff; DH-G2).
   S2 LANDED 2026-08-22 (D91, commit 786c9fb): scenarios/S2.scen (the
   `markers` staging key + `order 21 73 1`), the canonical gate with
   the pinned chain 809f4961b7757da4 (the full walk timeline — arm,
   present=1 window, arrival snap, beacon/claims clear), the
   differ-gate S2 row (present=1 spans both directions through the
   D90 splice), and the byte-pinned capture-plans/S2.json (the
   `_e_staging` seam field). The LIVE O1 leg of S2 stays with the
   operator session (the S0 checklist; re-stage the S2 plan the same
   way) — and note the live robot-count diff is the `_e_staging`
   seam, never a finding.
   NOTE (W6 addendum, D85 completion): the E runner stages the
   host-default marker set — no network-marker override — so ZONEA is
   a single-robot squad on E. ~~W8 must pin whether the original SP
   fills the 0x46cbe0 override (robot-count parity) before reading
   robot-count diffs as findings.~~ PINNED 2026-08-22 (W8-prep unit,
   RE-EXD-MAP §5d, D89): the original SP does NOT fill the override —
   the spawn branch (EXW FUN_0040cca0 @0x40cd8d / EXD FUN_0001d9cd
   mode==0) applies the zone rule unless `[0x4edb88] != 0`
   (= EXD mode 0x1075d8), and the title menu sets `0x4edb88 := 0 ∧
   0x46cbe0 := 1` for "New Single Player Game". SP ZONEA banks ONE
   robot in EXW, EXD, and E alike: robot-count parity holds, and
   robot-count diffs in SP scenarios are a genuine finding class.
   (Count-cell note for future MP scenarios: the bank dump must bound
   by the CAP cell — EXW 0x46ccbc total / 0x46cbd8 per-player, EXD
   0x11950c cap / 0x11958c per-player; in SP all equal the zone rule.)
9. **W9 — gates/CI wiring** (DH-G3 + corpus-gated CI job).
   **LANDED 2026-08-22 (D92):** the named `diffharness` CI job (§9
   DH-G3 bullet — what CI proves vs the live session), the workspace
   corpus-skip sweep via a fresh corpus-free clone test run (all 52
   test targets green after it; the one non-skipping suite
   `menu_gate` — 3 tests panicking on the absent corpus — fixed with
   the `corpus_present()` guard), and the §9 CI-wiring landing note
   with the re-verification recipe (re-run the clone test whenever a
   new corpus gate lands).
10. **W10 — 8street instrumented comparator (O3).** Rebuild 8street at the
    pinned commit with state-dump hooks emitting the W3 schema (test-only
    comparator; no code enters this repo).
    **FEASIBILITY NOTE 2026-08-24 (D142, W10-prep):** the landing study is
    docs/O3-8STREET-COMPARATOR.md — the pinned clone + digests
    (8street/Bedlam @ a8622e6, tree f9df7045, bedlam.asm/bedlam_data.inc
    sha256s recorded), the build toolchain + operator gates (the FIRST
    build needs sudo + network: JWasm + i686 SDL2/SDL_mixer; compile.sh
    alone is unattended-safe after), the memory-layout reality (all cells
    resolve by SYMBOL NAME; the bedlam_data.inc DRIFT LEDGER — 8 filler
    defects, exact only up to 0x4DC6D0 — with the cross-validation table
    re-anchoring 8street semantic symbols to registry EXW cells:
    current_money≡money, difficulty≡difficulty, robots_available≡the D89
    per-player cell, game_mode≡mode, zone/zone_level≡zone/mission,
    rnd_seed1/2≡rng-state-a/b, sound_enable≡sfx-master-gate,
    mission_square≡static-tot-volume), the hook family (H1 frame tail =
    the game_level wait site [ASM] loc_448730:99697 ≡ EXW 0x425a03; H2
    anchor = loop-head first entry loc_447E6A:98943; H3 the D77 §3 inject
    seams through the three resolution cases; H4 the hook emits DBXCAP v1
    directly, reusing stitch→encode→chain→differ unchanged), the differ
    intake gaps (dbx-stitch --channel o3 + the O3 anti-ghost rule LANDED
    2026-08-24 as W10-impl-a, commit f584eab/D143 — the O3 rule is the
    O2 mirror: ids validate against `exw_addr`, the EXD-only row rejects
    loud; the O3 field map + o3-seam classification LANDED 2026-08-24
    as W10-impl-b/D144), the never-comparable
    o3-seam classes (config/registry-writer rows, volume-key scancode
    swap, OPTIONS.BDL-backed cells), and the artifact/hygiene split
    (fork outside the repo; outputs runtime-only; O3 runs against a
    STAGED corpus copy — the reconstruction writes SAVES/+BEDLAM.LOG).
    VERDICT: feasible; the rebuild is operator-gated and parked until a
    three-way tiebreak is actually wanted.
    **THE DIFFER O3 FIELD MAP LANDED 2026-08-24 (W10-impl-b, D144; spec
    commit 7d28bc2):** `normalize_o3_row` = the O2 map verbatim + the
    O3-8STREET §5a seam ledger (row-id + exw_addr-cell matchers over
    the D128 registry-config family cells) classifying `Class::O3Seam`
    — report-only, excluded from tiebreak arbitration, binding the O3
    channel only. Gates: the real-S0 fabricated O3 self-cross PASS 0
    findings; a seeded seam row → exactly one o3-seam finding; the
    same seed on `money` still FAILs EngineBug (selectivity, never a
    blanket suppressor); a synthetic ACTIONPAN row proves the cell
    matcher end-to-end; the O2-headers control proves the class never
    fires without an O3 side. W10 IN-REPO WORK IS COMPLETE — only the
    operator-gated rebuild + live captures remain.
11. **W11 — Wine/EXW spot-check channel (O2).** Host ptrace driver:
    frame-tail breakpoint at the EXW site + process_vm_readv bulk reads of
    the same registry rows; used to arbitrate every `original-divergence`
    finding and for canon-only EXW behaviors.
    **O2 PLAN FORM LANDED 2026-08-24 (D138, W11-prep):** `dbx-plan
    --channel o2` compiles the O2-side plan (every address = the
    registry `exw_addr` canon cell in flat linear form — zero
    translation; the DOSBox boot/arm machinery replaced by the
    `trigger` object; walk scenarios refused) and byte-pins
    `capture-plans/S1-o2.json`. The D137 static-map-wh pin's span
    arithmetic was CORRECTED by D138: the EXW w/h cells are ADJACENT
    (4 apart), so the O2 capture form is the 8-byte span @0x4eddec
    (w@+0x00/h@+0x04) — see D137-CORRECTION. The driver itself stays
    operator-gated W11 work.
    **O2 STITCH SUPPORT LANDED 2026-08-24 (D139, W11-prep):** the
    stitch side of the D138 plan form — `runner::stitch` threads the
    dump channel through the anti-ghost validation (O2 transcripts
    validate every id against the registry's `exw_addr`, the mirror
    of the O1 `exd_addr` rule; the one EXD-only row
    `static-cursor-clamp` rejects LOUD, never silently — and the rules
    are per-channel mirrors, never global: an EXD-gap T3 row with a
    live EXW cell dumps fine on O2). `dbx-stitch --channel o2`
    produces the O2 dump + manifest through the same
    channel-agnostic stitch/encode/chain machinery (§3).
    `differ_gate` (`s0_o2_transcript_stitch_channel_rule`) drives the
    lane headless: the fabricated O2 transcript of the real S0 run
    (D138 row forms) stitches through the enforced rule + decodes
    channel-marked, the EXD-only row refuses, and the same row stays
    legal on O1. With this, the O2 plan (dbx-plan), the O2 dump
    normalizer + tiebreak (differ), and the O2 stitch are all
    channel-complete — the ptrace driver itself is the only remaining
    W11 piece (operator-gated).
    **O2 TRANSCRIPT EMITTER SKELETON LANDED 2026-08-24 (D140,
    W11-prep):** `tools/runtime/capgen-o2.py` — the headless producer
    of the O2 DBXCAP. Contract split (the skeleton's reason to exist):
    the W11 ptrace DRIVER (operator-gated) services the plan — trigger
    hits at `trigger.site`, `process_vm_readv` per watch row — and
    logs a **DBXFEED v1** read/write log; `capgen-o2` is the pure
    plan interpreter + transcript emitter that VALIDATES the feed
    against the plan walk 1:1 (addr+len per read, hit numbering,
    inject arithmetic re-derived) and writes the DBXCAP v1 the D139
    stitcher consumes. Grammar: `DBXFEED v1` / `kind synthetic|driver`
    / `hit <n>` (hit 0 = the optional boot position; hit 1 = the
    ANCHOR — the driver's mission-load policy decides where the feed
    starts, the plan never guesses) / `read <addr> <len> <hex>` /
    `write <addr> <len> <hex>` (inject entries precede the frame's
    watch reads in each block, mirroring the O1 write-then-dump
    ordering). Inject coverage is the FULL plan grammar: plain
    {frame,addr,bytes} writes, `op:command` ring appends (the emitter
    re-derives base+count*stride from the logged count-cell read and
    validates the zero-extension + count bump), and `op:pad` step-ons
    (the 8-B slot record read is mark-checked, the xyz triple writes
    validated) — frame numbering = capture frames, anchor = 1, on
    BOTH channels (`compile_steps` pins "anchor-relative boundary
    numbering = capture frame numbers"; the D138 comment's "Nth
    trigger hit after the anchor" reads as: frame numbers count the
    post-mission-load hit sequence, of which the anchor is #1).
    A `--synthesize-feed` mode builds a deterministic SYNTHETIC feed
    for any o2 plan — a reference mini-driver whose arithmetic
    exercises every feed form, including inject ops (the S3-o2
    compile path drives `op:command` end-to-end headless). The
    frame-counter alignment check (counter value must advance +1 per
    hit from the anchor) warns + records a transcript comment on
    drift (a missed trigger hit would otherwise silently misalign
    frames). The smoke (`tools/runtime/capgen-o2-smoke.sh`,
    unattended-safe, no Wine, no corpus read) proves the chain:
    dbx-plan --channel o2 byte-pins against the committed S1-o2.json →
    synthesize → emit → `dbx-stitch --channel o2` (manifest O2:EXW/Wine,
    frame contract) → `dbx-diff` decode + normalize_o2_row intake →
    the loud rejections (a `static-cursor-clamp` row spliced into the
    transcript refuses; a truncated feed refuses). Synthetic
    transcripts carry the SYNTHETIC marker comment (anti-ghost: they
    are never live captures; the s0-replay fixture precedent).
12. **W12 — scenario depth S3–S8** as producer families land in-engine
    (each S3+ unit pairs the engine producer with its scenario).
    **S3-PREP LANDED 2026-08-22 (D102, §7j.37):** the E-side
    weapon-fire COMMAND producer is in `bedlam-core::weapon` — the
    consumer (FUN_00409138's modeled subset: flags, the verified
    fire gates, the inline spawn cases field-exact, the family
    routing, auto-rearm, the recharge pass), the 400×0x36 weapon
    bank + the 50×0x22 projectile bank with their per-type ticks
    (FUN_00410823/FUN_00412010 subsets), and the damage table. The
    banks are exposed via read accessors (the S3 T2 rows read them;
    they stay out of `state_hash` — the W6 split). NO-INJECT
    INVARIANT pinned: the S0/S1/S2 chains are byte-identical and
    `advance_frame` draws no RandA without staged records.
    **S3 LANDED 2026-08-22 (D103):** the unit this prep named is
    complete end-to-end. Grammar v1.3 adds the `loadout` staging key
    (§7 note above) through `stage_robot_weapons`; the EXD twins for
    the two T2 banks are PINNED and REGISTERED (RE-EXD-MAP §5c,
    W12-S3 Ghidra hop: weapon-anim 0x980d4 — the free-slot finder
    FUN_00023295 bound 0x5460 = 400·0x36 exact + the tick twin
    FUN_000212f2 with the 0x17 3-clone split; projectile 0x10e174 —
    the tick twin FUN_00022a52's 50-slot walk + the +0x1A/+0x1E
    tail words beyond the 7 E-modeled fields, an O1-only coverage
    surface, never fabricated). `parity_harness --canonical` emits
    both banks as u32 count + the FULL records (the record field
    order IS the guest layout — no compaction; the W6 split keeps
    them out of state_hash); S3.scen stages 8 COMMAND volleys over
    133 records covering every class the modeled dispatch can
    INLINE-spawn (artillery 9/0xA/0xB, mines 0xF/0x13, grenades
    0x1A/0x1F, rocket 0x24) — bullets/shell/0x17/homing are
    documented E-gaps (their producers are the unmodeled AI-order
    families + the mortar; a live O1 firing them surfaces as differ
    coverage findings, never silence). Chain pinned
    49193732e6dbc546, byte-identical double run; S0/S1/S2 chains
    re-asserted byte-identical. The differ normalizes both banks on
    BOTH channels (E: count+records; O1: the bare guest span — no
    count cell, the free-slot walk is the bound) through the SAME
    field walk; differ_gate S3 = cross PASS-WITH-NOTES (exactly the
    2 E-only rows at landing — blink-cursor + move-target-words; the
    D132 blink-cursor alignment of 2026-08-23 fabricates the row on
    the gate's O1 side and drops it to 1, move-target-words only —
    zero field gaps, zero T2 diffs). S4 (the destroy
    family onto destructibles) is the next scenario unit, gated on
    the S3 finding set.
    **S4-PREP LANDED 2026-08-22 (D104, §7j.38 + §7j.39):** the
    E-side destroy family is in `bedlam-core::destroy` — the
    mission-load STAGING (the .BDG type table ≤282 rows + the .POS
    2000×16-B instance list with the footprint/hp re-stamp + the
    .TRT terrain-structure bank, all host-seamed per the D51
    pattern; the 0x7d2/0x7d3 hazard stamper FUN_00422f18), the two
    RESOLVERS (FUN_0041a894 objects incl. the platform 0x7d4 entry
    FUN_00422693 destroy/weaken + FUN_0041bc1c structures with the
    rubble stamp), the destroy TAIL (objective notify → the GER
    gate → the template-bank terrain RESTORE +0x46/+0x4A → the
    five-effect loop with the §7j.38 draw table 8/8/8/8/8/0/0/72/9
    → the score award → the four perimeter CHAIN walks with the
    §7j.39/5 corrected geometry), the widened 20-kind debris stager
    (the 11 seq walks + the ring/center scorch classes + the LRU
    allocator), the splash stager + the water-z probe, the script
    blast FUN_004244a1, the tile-0x62 trap lane, and both disbursers
    (FUN_004124a4/FUN_004126dc — the §7j.14 0xF/0x65 corrections
    landed). The weapon-tick IMPACT LANES are wired (the
    §7j.39/2-verified call orders: bullets/shell/0x24/0x29 floors,
    the artillery burst pairs, the mortar 3-cell, the class-0
    quadrant body, the projectile type-1/2/3 branches). NONE of it
    enters `state_hash` (the W6 split — debris/splash are T3
    rows). NO-INJECT INVARIANT: S0/S1/S2 chains byte-identical; S3
    re-pinned ONCE to e29f76f5585401e1 (the burst pairs draw the
    shared stream whether or not destructibles are staged — no O1
    S3 capture exists yet; the dbx-plan T2-tier unit landed D109,
    the plan awaits the operator session). S4.scen (S3 volleys onto
    staged destructibles) is the next unit.
    **S4 LANDED 2026-08-22 (D105):** grammar v1.4 adds the `destroy`
    staging key (§7 note above — an EQUIVALENCE seam: the original
    loads the same .BDG/.POS/.TRT natively, so no O1 write exists to
    fabricate; dbx-plan records the key in `_e_staging`, and the
    recorded pre-S5 divergence is E's EMPTY-staged mirror banks).
    S4.scen on ZONEA/MISSION1 (49 records, chain pinned
    2ddd15ea50c8a14d, byte-identical double run) covers the S4 row
    legs: the TRAP (a marker robot standing on the tile-0x62 cell —
    resolver-100 no-score destroy at the anchor frame, 5× k12 trap
    debris + the sel-9 k20 + the 3×3 splash ring + the restore into
    the empty-staged mirror bank), the ARTILLERY burst pairs landing
    on footprints (a marker gunner firing 9/0xA/0xB at its own tile
    — ring 0 script-blasts the .TRT turret at frame 32 with the
    rubble stamp, the rings reach the chainable cluster at 35..38
    cascading recursive 1000-damage detonations, the blast box also
    damages the gunner itself — the faithful §7j.23 robot lane), and
    the SURVIVOR (two bouncy-grenade volleys on a 900/1800-hp
    structure — pure multi-hit subtract, monotone, never destroyed,
    no score). The canonical destroy rows ride T1/T3 as their own
    blobs (object-instances 23-B records keyed by .POS slot,
    trt-array 20-B, the shared-span tile-word-grid +
    platform-strength, typedb-mirror-rows COMPACT-ACTIVE {tile,
    8×(word, seen)} with the same nonzero-tile filter canonicalizing
    the O1 full 0x1E-stride span; debris-stager 42-B FULL bank +
    splash-records 10-B FULL bank — the T3 pair has NO EXD alias
    yet: E-only rows, differ coverage findings, never fabricated).
    The differ normalizes both channels through the same field
    walks (the guest object 0x14-stride count-bounded walk skipping
    dead id==-1 slots, the TRT 0x20-stride stride-offset map, the
    mirror tile filter); differ_gate S4 = cross PASS-WITH-NOTES
    (exactly the 4 E-only rows at landing — blink-cursor,
    move-target-words, debris, splash; the D132 blink-cursor
    alignment of 2026-08-23 fabricates the row, dropping it to 3 —
    move-target-words, debris, splash —
    zero field gaps, zero T2 diffs beyond the
    single counter note). The score fold landed in the MissionShell
    (the destroy award folds into the campaign score cell — zero
    without staged destructibles, the no-inject invariant):
    S0/S1/S2/S3 chains re-asserted BYTE-IDENTICAL
    (8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 /
    e29f76f5585401e1). ~~A live S4 capture needs the dbx-plan
    T3-tier unit first~~ (landed D109 — capture-plans/S4.json).

    **S5/S5B LANDED 2026-08-22 (W12-S5, D108):** grammar v1.5 adds
    `zone = "B"` (the episode-slot host seam `stage_episode_slot` —
    the campaign-advance/save-load shells the host stands in for;
    zone letter → stage, mask 0 → MISSION1, linear stays 0 ~~linear
    stays 0~~ [superseded by the S0-12b/D154 derived-cell seam: the
    emitted row is clamp(5·(zone−2)+mission−1, 1, 26) = 1 on
    ZONEB/M1]) and
    `pickup = 1` (the mission's OWN .TOT through
    `stage_pickup_surface` AFTER any destroy staging — the engine
    load-order note — then the §7j.12/6 hazard stamper, matching
    the original's mission-load order). S5/S5B run ZONEB/MISSION1
    set 2 with destroy staged too, so the typedb-mirror-rows carry
    the REAL staged surface (15,102 words / 52,715 seen of
    80,000 — every tile active, the compact row is the honest full
    form) and the recorded S4 empty-mirror divergence closes for
    S5-class scenarios (S4's chain is untouched — it sets no pickup
    key). THE TWO-SCENARIO SPLIT is a dump-budget decision (D108):
    one scenario cannot walk both corridors — the nearest case-1
    and case-3 cells are 61 octagonal tiles apart (beyond any
    order's reach), and two sequential orders need the first order
    CLEARED (all-alive-state-3 — impossible while the second leg's
    robots stand idle — or the 0x197-frame window expiry, whose
    ~407 idle frames × ~340 KB/record of REAL mirror rows is not a
    shippable dump). S5 = the row-21 z3 trio (c1/c2/c4, arrival
    frame 5, 16 records), S5B = the row-10 z3 five (c3 + 4× c4,
    the (78,10) diagonal probe reach, arrival frame 12, 19
    records); chains pinned in canonical_dump_gate, differ_gate
    rows joined, dbx-plan compiles both tiers T0/T1/TS (no T2/T3
    tier needed — nothing fires/dies/explodes in the walks) with
    the zone + pickup seams recorded in `_e_staging`. The zone-row
    O1 normalizer (cell−1) landed with them — the first non-A-zone
    scenario exposes the 1-based guest cell vs E's 0-based slot
    index (§6a zone convention, D108). S0..S4 chains re-asserted
    BYTE-IDENTICAL.
    **S5C LANDED 2026-08-22 (W12-S5C, D110 — the case-3
    observability variant):** S5B's walker spawns AT the hp clamp
    (5000), so apply_pickup case 3's +2500 was value-invisible
    there. S5C spends the walker below the clamp BEFORE the walk
    with the S4 artillery pattern: a THIRD marker stages the
    gunner ON the walker's tile (73,10,3 — ≤5 tiles from the order
    tile, inside ORDER_RADIUS), its loadout arms 9/0xA/0xB (1 ammo
    each), and the frame-1 `command` fires all three records at
    its own tile (bursts land at the FIRING robot, §7j.38/5). The
    §7j.23 robot lane (312/pair) box-reaches a marker-staged robot
    (+0xF00 = Q5 offset 15) from FOUR list-0 pairs
    ({T,T+1}×{Ty,Ty+1}) per burst: 3 records × 4 × 312 = 3744
    spend at frame 32 (tick 0x20) on the walker AND the gunner —
    both survive at 1256; the 0xB's outer rings spend the CLICKER
    624 at frame 36; all damage lands pre-order (state 0/3 — the
    hp path; a state-4 robot converts damage to a shield tick).
    `order 78 10 3` arms at frame 37; case 3 fires at frame 41:
    hp 1256 → 3756, the EXACT +2500 (PICKUP_HEALTH) UNCLAMPED —
    the D108 value-invisibility gap closed. The gunner claims its
    own spread slot and walks one robot behind the whole way (it
    reaches no unconsumed cell, hp 1256 through the tail — lower
    index moves first, deterministic). CORRIDOR CENSUS CAVEAT vs
    S5B: the burst rings + the destroy CHAIN CASCADE rewrite many
    off-corridor mirror words/seen bits (the 5000-damage resolvers
    detonate the chainable cluster around (73,10) — S5B's
    "exactly six cells" census does NOT hold; all of it is
    deterministic destroy-family state, chain-pinned; the asserts
    target the corridor cells + the hp schedule). 55 records,
    chain e0999fcb3455d3ef pinned in canonical_dump_gate,
    differ_gate row joined (same 2 S1-class findings — the
    cascade rides the SAME aliased T1 rows), dbx-plan compiles
    tiers T0/T1/TS (4 inject rows: the command append + the
    frame-37 order triple; the loadout seam in `_e_staging`),
    capture-plans/S5C.json committed + byte-pinned. S0..S5B
    chains re-asserted BYTE-IDENTICAL.
    **dbx-plan T2/T3 TIERS LANDED 2026-08-22 (D109):** SUPPORTED_TIERS
    widens to T0/T1/T2/T3/TS — S3 (T2) and S4 (T0/T1/T3) plans
    compile (capture-plans/S3+S4.json committed, byte-pinned by
    tests; S1/S2/S5/S5B regenerated). The two aliased T2 banks emit
    as the FULL fixed spans (weapon-anim 0x980d4 × 0x5460,
    projectile 0x10e174 × 0x6A4 — no count cell on the guest, the
    free-slot walk is the bound); every unaliased T2/T3 row (mortar/
    critter/POI + all 14 T3 rows incl. debris-stager and
    splash-records) stays an explicit `_deferred` coverage gap —
    never emitted on O1, the differ reports them E-only. THE
    COUNT-PREFIX GRAMMAR landed with it: the differ's O1
    normalizers pin bank rows as u32 count + records
    (trt-array walks 0..count; object-instances walks the whole
    span skipping dead id==-1 slots), but no contiguous guest span
    carries the count cell — capgen watch rows gain a `prefix`
    {addr, len} sub-row (dump the 4-byte cell first, concatenate;
    headless-proven in the flow probe), and dbx-plan emits it for
    trt-array (prefix 0x11949c) and object-instances (prefix
    0x119554 + the FULL 2000×0x14 bank — the D108 live-past-dead
    .POS holes: a count-bounded span dropped 32 live objects and
    broke the count field). robot-bank stays the bare span its
    normalizer defines. BONUS FIX: the D103 loadout `_e_staging`
    mask was a JSON hex literal (unparseable — S3 is the first
    compilable loadout plan); masks are decimal now.
    **S6 LANDED 2026-08-23 (W12-S6, D112):** the extraction scenario —
    the .PAD step-on arms the beacon through the REAL producer (see the
    §7 row). The trigger chain decode §7j.40 (631bd28) + the engine
    extraction family (edafd02) landed first; the scenario leg adopts
    the interrupted harness WIP. THE .PAD TERMINATOR BUG fixed with it
    (the dead `x == -1` break on a `u16 as i32` read — the slot bank
    collected the 0xFFFF fill past the live run; now u16-vs-0xFFFF,
    ZONEA/M1 = exactly 114 live slots). S6.scen: T0/T1/T3/TS, 75
    records, chain c96f0735df1059ea, corpus_s6_pad_extraction gates the
    full timeline (the dropship-frame T3 row is E-only — the differ
    normalizer lands the 7-leaf field walk, the cross compare reports
    the coverage finding, never fabricated); capture-plans/S6.json
    committed + byte-pinned (3 injects — the pad op + the two command
    records; NO staging seam rows). S0..S5C chains re-asserted
    BYTE-IDENTICAL.

    **S7 LANDED 2026-08-23 (W12-S7, D113):** the platform-dynamics
    scenario — the build/spread/creep/destroy lifecycle in one run
    (see the §7 row). The §7j.41 decode (984a078: the trigger
    dispatcher whole + the zone code tables, the ring gates
    instruction-exact, the weaken ring-gate + site-latch gloss
    corrections, THE PER-FRAME RandA GATE-DRAW finding) + the engine
    producers (ea2f259: platform_ring_build/platform_tile_build,
    platform_creep_tick, the FUN_00422600 destroy-tail trigger, the
    volume-2 stage_platform write) landed by the predecessor
    (56d80c42); the scenario leg (b9cbcf3: S7.scen + the
    corpus_s7_platform_dynamics timeline gate, chain
    b41db389f3ad8947) landed the same session; THIS run completed the
    differ/plan/docs legs (4c6c068 + 13bae85): the differ_gate S7 row
    (cross PASS-WITH-NOTES, exactly the 2 S1-class findings at
    landing + the debris/splash E-only pair — 1 S1-class after the
    D132 blink-cursor alignment of 2026-08-23 —
    zero field gaps; the platform rows
    fabricate as the identity spans) + dbx-plan's grammar-v1.6
    platforms arm note in _e_staging (the RNG-stream equivalence,
    never a fabricated write) + capture-plans/S7.json (34 anchor +
    25 per-frame, 5 command injects, byte-pinned). S0..S6 chains
    re-asserted BYTE-IDENTICAL (the arm key keeps the epilogue tick
    off the unarmed paths).

    **W12-S8 LANDED 2026-08-23 (D114)** — the critter-engagement
    family, end-to-end: the §7j.42 decode + the engine
    bedlam-core::critter leg (8786c9e — the bank, the .NME staging
    host seam with fail-loud kind refusal, the k4 seek steppers +
    the k5/6 mixed-AI body instruction-exact per the §7j.43
    corrections, the 0x68 fire cycle, the FUN_004197d4 odd-pass
    walker, the FUN_004190bc applier at the bullet-substep and
    script-blast lanes, the §7j.24 death handlers + the
    FUN_0041a14f effect-row bank, the bounty gate) + the scenario
    leg (S8.scen, chain b5ae3f8be91c7449, corpus_s8 gates the
    lifecycle) + the differ legs (the critter-bank/effect-rows
    normalizers as E-ONLY coverage rows; the differ_gate S8 row —
    cross PASS-WITH-NOTES, exactly the 2 S1-class + the
    critter/effect pair at landing — 1 S1-class after the D132
    blink-cursor alignment of 2026-08-23 —
    zero field gaps) + dbx-plan's grammar-v1.7
    critters seam note + capture-plans/S8.json (36 anchor + 27
    per-frame, 1 command inject, byte-pinned). The S0..S7 chains
    re-asserted BYTE-IDENTICAL (the staging+arm key keeps the
    controller + the loader draws off the unarmed paths).

    **DEBRIS-PHYSICS LANDED 2026-08-23 (D115, §7j.44)** — the
    FUN_0040de9c family, the last Backlog producer gap: the tick
    FUN_00420549 (delay/anim/free lifecycle + the phys gate, the
    MissionShell epilogue slot) + the three collision walks
    (robot lane via the FUN_0040db9e dispatcher; the
    terrain-gated critter lane with the §7j.24 register-gloss
    correction; the POI squash lane E-only) by d467471/cebc178.
    RE-BASELINE (b2c89af): turning the pass on makes
    physics-class chunks mutators on the aliased robot bank, so
    FIVE chains move — S3 9a11efa03baafb64 (mine/grenade
    expiries stage k12 mag-25/k3 chunks; the k11 artillery gate
    stays phys-0), S4 35fa3a9234cbff37, S5C 786fd87565b67f4a,
    S7 ecdce5472df6a324, S8 44d806b81bd1b1ff; S0/S1/S2/S5/S5B/S6
    BYTE-IDENTICAL. The §7 table's landing notes above carry the
    AT-LANDING chains — the gate tests are the pinning authority.
    The debris-damage observability lives on corpus_s4 (the
    knock-widened cascade + the freed-ring lifecycle), corpus_s7
    (the standing gunner's chunk-field schedule), corpus_s8 (the
    burst-window chips); the S5C case-3 consume order flips to
    the gunner (heal value still exact — an O1 capture
    arbitrates). The differ contract is unchanged (zero new
    rows; the lanes surface through the aliased banks).

## 11. Risks

- **Debugger surface uncertainty** ~~[pin-unverified] — the single biggest
  unknown; DH-G0 exists to retire it early.~~ RESOLVED NEGATIVE for the
  pinned flathub runtime (2026-08-22 W4 audit: no debugger compiled in;
  startup.js log-only — RUNTIME.md). The risk moved one step earlier: the
  channel itself must be re-pinned (self-build/GameLink/O2-ptrace options).
  Fallbacks: O2 ptrace channel as the escape hatch (all addresses verbatim).
- **EXD↔EXW logic divergence polluting diffs** — bounded by the
  `original-divergence` class + O2 arbitration; expected only in
  hardware-coupled categories per PLAN §0; anything in pure game rules is a
  finding, not noise.
- **Watch-mode perturbation** — interpreter core + watch mode per D29 pins;
  DH-G1's double-run digest check quantifies capture determinism before any
  diff is trusted.
- **Dump = asset-derived data** — runtime/-only, fingerprints in git
  (§3 hygiene), manifest checks bracketing every corpus-touching run.
- **Scope creep into pixel/audio parity** — explicitly out: T2 render
  goldens live in DESIGN-RENDER/D24 land, audio byte-identity in
  DESIGN-AUDIO (D17b). This harness owns STATE only.
- **Uninjectable behaviors** (e.g., volume-key paths, pause) — scripts avoid
  them; documented as non-scenario surface.

## 12. Provenance summary

All §4/§5 addresses are [verified] via their cited ledger rows (RE-EXW-SIM
§8 + §7 series, RE-EXW-INPUT, RE-EXW-GAMETHREAD, RE-EXW-PACER) — the
underlying disasm evidence lives there, not here. Architecture choices
(channel roles, injection seams, tiering, dump hygiene) are [derived]
decisions recorded as D77 in docs/DECISIONS.md. Runtime pins inherit
RUNTIME.md (D19/D29 upgrade + re-baseline policy).
