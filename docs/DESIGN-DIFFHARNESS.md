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

- **Trigger**: [pin-unverified] the exact DOSBox-X debugger command surface
  (BPINT/BPLM/D forms, watch-mode logging, startup.js automation route) is
  verified at the first interactive session per the RUNTIME.md skeleton
  checklist — that session converts this section's UNCERTAINs into committed
  runbook facts. Fallback trigger class if watch mode can't bulk-read on
  demand: a single linear breakpoint at the frame-tail site whose handler
  reads the whole tier list (the B2 skeleton already assumes this shape).
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
| blink-cursor selector | 0x4dc5d0 | u32 (0 or slot+1) | §7j.7 item 6 [verified] — the S1 hypothesis watch |
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

## 7. Scenario corpus (build order; each names its hypotheses)

| id | scenario | script sketch | tiers | arbitrates / proves |
|---|---|---|---|---|
| S0 | boot→mission | no injection; run to ZONEA/MISSION1 first frame | T0 + statics | loader parity (statics hashes); frame-trigger stability; the W1 EXD map on real memory |
| S1 | mission-start passive | no injection; record ~400 frames | T0/T1/T3 | **pod-descent stagger** (watch pod timer w@+0x2C ≡ 0x4c6a10 vs the `1+k·(2000−m·1000/27)` formula, pod ring phase→release, descent ≈41 frames, pod phase 2 = one tick, release = state 6 — §7j.20/§7j.27); **blink-cursor-from-spawn** (does 0x4dc5d0 go nonzero with no click, from which frame); **debris 2k start-delay** (which 0x476fbc +0x24 values actually get staged on the corpus path — is any ≈0x7D0?) |
| S2 | order→walk (the P4 slice) | ORDER steps moving one squad member across ZONEA/MISSION1 (mirrors engine/bedlam-core/tests/mission_corpus_gate.rs) | T0/T1 | slice field parity (positions, arrival snap, spread claims, move-target words); the engine's biggest existing seam |
| S3 | weapon fire family | COMMAND records: one per weapon class (bullet 2..4, shell 5, artillery 9..0xB, ballistic {0xE,0xF,0x13,0x17,0x1A,0x1F}, rocket 0x24, homing 0x29) at fixed targets | T0/T1/T2 + T4 | fire cadences, damage application (FUN_00419aff table), bank record lifecycle; the corpus-off weapon producers; T4 SFX events seed the SFX-family walk |
| S4 | destroy family | S3 fire onto destructibles (tile 0x62 traps, platform 0x7d4, chainable objects) | T0/T1/T3 + T4 | destroy resolver → terrain restore → 5-effect loop → chain walks end-to-end (§7j.25); **five-ring overlap read** (0x4796d4 bytes around overlapping corpse rings — statically mooted by §7j.10's ≤7-frame fade; the harness read is the confirming observation); debris producer kinds/delays (stager widening input) |
| S5 | pickups & pads | walk over pickup tiles + armor-pad rings | T0/T1 | pickup_case dispatch vs the type-DB mirror rows (the 7h.3 producer question gets its observation instrument) |
| S6 | extraction | ORDER onto the extraction .PAD (step-on) | T0/T1/T3 | **arm-extraction via scripted .PAD step-on** (beacon family, exit ring phases, dropship deploy, objective counters — §7j.19/§7j.20/§7j.27) |
| S7 | platform dynamics | repeated fire on platforms (build/spread/creep/destroy) | T0/T1/T3 | platform family field parity (§7j.12) |
| S8 | critter engagement | walk into critter aggro ranges | T0/T2/T3 | critter states/AI per difficulty; death handlers; bounty gate (§7j.17/§7j.23/§7j.24) |

The **mid-flight draw blit sequences** (7j.28) are render-side and stay OUT of
state diffs (T2 perceptual per §0b — they belong to frame goldens,
DESIGN-RENDER). What S3 DOES watch is their input data: the two projectile
banks + trail bank + draw counters (d@+0xE wrap, tick fields), which pins the
draw-relevant record semantics without pixel diffing.

**Per-zone case tables** (FUN_00433980, the ~25 extraction pads, the
0x424a6f message table) are decoded on demand as scenarios need them (S6
first) — the harness is what makes that decode mechanical (watch the armed
records instead of guessing the case).

## 8. Open hypotheses ledger (what this doc does with each)

| hypothesis | source | disposition |
|---|---|---|
| pod-descent stagger formula + release semantics | §7j.20, §7j.27 (static decode) | S1 watch confirms/refutes numerically; engine modeling follows the observation |
| weapon fire needs COMMAND records, not raw input | 7j.22 (queue note) | adopted as the injection design (§5.3) — S3 validates |
| destroy family end-to-end | §7j.25 (decoded, corpus-off) | S4 is its first live observation |
| mid-flight draw blits | §7j.28 (decoded) | out of state-diff scope (T2); S3 watches the record data (§7 note) |
| debris 2k start-delay | queue phrasing at da4bf20 (after §7j.7/7j.8, the +0x24 start-delay field discovery) | S1/S4 record every staged +0x24 value on real runs |
| blink-cursor-from-spawn | §7j.7 item 6 (producer decoded; from-frame-0 behavior unobserved) | S1 watches 0x4dc5d0 from the first mission frame |
| five-ring overlap last-write-wins | §7j.9, then §7j.10 declared it moot (fade ≤7 frames) | recorded as statically CLOSED; S4 takes the confirming byte-level read anyway (cost: one watch row) |
| arm extraction via .PAD step-on, not a click | 7j.20 (queue note) | adopted as injection design (§5.4); S6 validates |
| +0x18 armor-pad ring read semantics (raw byte ≠ 0 arms pads) | §7j.8 item 8 | already wired in-engine; S1/S4 regression-watch 0x4796d4 |

## 9. Gates

- **DH-G0 watch-proof** (interactive, one session): first interactive
  DOSBox-X run per the RUNTIME skeleton checklist verifies debugger command
  names, the linear-address conversion, and produces S0 dumps whose digests
  reproduce across two runs. Converts [pin-unverified] items to runbook facts.
- **DH-G1 runner-determinism**: headless S1 run twice → identical dump
  chains (same pin, same scratch corpus). No CI; desktop/local only, results
  committed as fingerprints.
- **DH-G2 structural parity (the P4 acceptance slice)**: S0–S2 STRUCTURAL
  mode green (loader statics + bank layouts/counts + occupancy shapes).
- **DH-G3 field parity budget**: S2 T1-exact green; S3/S4 T1-exact on the
  producer paths as they land in-engine; meter thresholds pinned per
  scenario at first green.
- **CI wiring**: original-side runs never run in CI (pinned emulator,
  desktop-gated). CI runs the ENGINE dump emitter + differ against committed
  reference fingerprints (corpus-gated, skip when game-data absent — the
  mission_corpus_gate pattern).

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
5. **W5 — injector.** The §5 vocabulary as runner-side writes (script
   grammar extension shared with the engine emitter).
6. **W6 — engine dump emitter.** parity_harness gains `--canonical`:
   per-tick canonical records in the W3 schema (MissionSim/MissionScene
   field maps for T0/T1 first).
7. **W7 — differ.** Normalizer + the §6 comparison modes + report writer +
   fingerprint manifest output.
8. **W8 — scenarios S1/S2 wired end-to-end** (first full O1↔E diff; DH-G2).
9. **W9 — gates/CI wiring** (DH-G3 + corpus-gated CI job).
10. **W10 — 8street instrumented comparator (O3).** Rebuild 8street at the
    pinned commit with state-dump hooks emitting the W3 schema (test-only
    comparator; no code enters this repo).
11. **W11 — Wine/EXW spot-check channel (O2).** Host ptrace driver:
    frame-tail breakpoint at the EXW site + process_vm_readv bulk reads of
    the same registry rows; used to arbitrate every `original-divergence`
    finding and for canon-only EXW behaviors.
12. **W12 — scenario depth S3–S8** as producer families land in-engine
    (each S3+ unit pairs the engine producer with its scenario).

## 11. Risks

- **Debugger surface uncertainty** [pin-unverified] — the single biggest
  unknown; DH-G0 exists to retire it early. Fallbacks: linear-breakpoint
  handler reads (skeleton's assumed shape); O2 ptrace channel as the
  escape hatch (all addresses verbatim).
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
