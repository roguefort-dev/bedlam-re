# STATE - project state snapshot (rewrite the head when the phase moves)

  - 2026-08-24 P4.2/W11-prep THE DBX-STITCH O2 TRANSCRIPT CHANNEL
    SUPPORT unit COMPLETE (worker 74bae49c claim 2, commits 1cc53b4
    + ab0738b, both PUSHED; D139): runner::stitch threads the dump
    channel through the anti-ghost validation — O2 transcripts
    validate ids against exw_addr (NoExwAddress; the EXD-only row
    static-cursor-clamp rejects LOUD; the rules are PER-CHANNEL
    MIRRORS: the T3 EXD-gap rows with live EXW cells dump legally
    on O2); dbx-stitch --channel o1|o2. Verified by the new runner
    unit + a new differ_gate corpus lane (the REAL S0 run
    fabricated under O2 through the enforced rule, the 8-byte D138
    map-wh span intact, the EXD-only row refusing, the same row
    legal on O1); full differ_gate 3/3 (829s), canonical_dump_gate
    13/13, diffharness 99, fmt+clippy clean, MANIFEST clean pre+
    post, no Ghidra run. THE O2 HEADLESS TRIANGLE (plan D138 <->
    differ D137/D138 <-> stitch D139) IS CHANNEL-COMPLETE — the
    remaining W11 work is the operator-gated ptrace driver + the S0
    live session (item 1, [BLOCKED]-on-operator). Queue: item 2 =
    the capgen O2 transcript emitter skeleton (plan->driver->
    transcript->stitch proven headless on a synthetic feed).
  - 2026-08-24 P4.2/W11-prep THE DBX-PLAN O2 CHANNEL SUPPORT unit
    COMPLETE (commits c57eae3 RE notes + b199ece impl, both PUSHED,
    worker c44a3c8b claim 2; D138 + D137-CORRECTION): dbx-plan
    --channel o2 compiles the O2-side plan (exw_addr canon cells in
    flat 0x form, the trigger object replacing the DOSBox boot/arm
    machinery, walk scenarios refused, steps on EXW seam cells);
    capture-plans/S1-o2.json byte-pinned; o1 output byte-identical
    to all 12 committed plans. HEADLINE: the new registry-derived
    span assert CAUGHT D137's static-map-wh arithmetic being
    IMPOSSIBLE ("0x24 apart / 0x28 span" vs the cells' actual
    0x4eddf0−0x4eddec = 4) — corrected everywhere to the 8-byte
    span @0x4eddec w@+0x00/h@+0x04 (differ arm + fixtures + gate
    fabrication included; the field-order asymmetry story
    survives). Two registry corrections: robot-bank/no-extract-
    latch EXW count-cell 0x46ccbc→0x46cbd8 (the per-player twin,
    W8-prep) + selection-triple's EXW pick cells[1] 0x46cbdc
    (D132). diffharness 98, differ_gate 2/2 corpus (696s),
    canonical_dump_gate 13/13, fmt+clippy clean, MANIFEST clean
    pre+post. The W11-prep headless surface is now plan+differ
    COMPLETE; queue: item 2 = dbx-stitch O2 transcript support
    (the last headless piece), then only the operator-gated live
    work remains (S0 session item 1 + the W11 ptrace driver).
  - 2026-08-24 P4.2/W11-prep THE O2 STATIC-MAP-WH PIN unit COMPLETE
    (commits 1438ca6 RE notes/D137 by predecessor a3532435 +
    0ea13b8 impl by 05178a0c claim 2, both PUSHED): the LAST
    deliberate zero-field differ row closes. normalize_o2_row's
    static-map-wh arm parses the D137-pinned EXW form (the 0x28
    span @0x4eddec, w@+0x00/h@+0x24 — exact-length need rejects the
    EXD 0x30 span on O2 and vice versa on O1); the differ_gate
    fabrication is CHANNEL-SPLIT (inv_frame emits the EXD 0x30
    span under O1, the EXW 0x28 span under O2); a NEW direct
    E-vs-O2 cross in s1_o2_tiebreak_arbitration proves the row
    compares CLEAN through the real O2 normalizer (coverage
    exactly 1 = move-target-words; S0 stays 0); new o2_row_forms
    unit + tools/differ o2_frame fabrication. All four tiebreak
    lanes re-verified on their own channel forms. The W11-prep
    DIFFER side is now COMPLETE — what remains for W11 is the
    live ptrace driver (operator/Wine-gated) + the plan side.
    differ_gate 2/2 green (697s, corpus), diffharness 43, fmt+
    clippy clean, MANIFEST clean pre+post. Queue now: item 1 =
    [BLOCKED] S0 live session (operator), item 2 = dbx-plan O2
    channel support (the headless W11 prerequisite).
  - 2026-08-23 P4.2/W11-prep THE DIFFER_GATE O2 TIEBREAK FABRICATION
    unit COMPLETE (commits 04cd6b0 + 4591f52 PUSHED, worker
    7956a0e8 claim 2): all four compare_field T1-exact arbitration
    lanes now driven headless (s1_o2_tiebreak_arbitration) — one
    inv_frame fabrication stitched under BOTH O1 and O2 channel
    tags (normalize_o2_row alias list = EXD-identical forms;
    EXW_ROBOT_MAP == EXD_ROBOT_MAP; the O2 static-map-wh row is
    zero-field pending the W11 pin), the engine-is-wrong lane
    re-stitches the REAL E frames under Channel::Engine with money
    perturbed (stitch_o1 generalized to stitch_chan). Lanes assert
    class+detail+a/b verbatim: O2-with-O1 -> EngineBug "engine is
    the outlier" FAIL; O2-with-E -> OriginalDivergence
    PASS-WITH-NOTES (budgeted); all-three -> EngineBug "wrong
    against both oracles"; none -> "provisional"; idle-tiebreak
    baseline = nothing changes. NO production change — the W7/D87
    arbiter verified as-written (W11's live channel inherits it).
    differ_gate 2 tests green (693s, corpus), fmt+clippy clean,
    MANIFEST clean post-run. Queue now: item 1 = [BLOCKED] S0 live
    session (operator), item 2 = the O2 static-map-wh pin (the
    last deliberate zero-field W11-prep row).
  - 2026-08-23 P4/W7-followup THE DIFFER_GATE BLINK-CURSOR
    FABRICATION ALIGNMENT unit COMPLETE (commit 2d53aaa PUSHED,
    worker 9035ca6a claim 2): the gate's fabricated O1 side now
    carries blink-cursor like every real post-D132 capture plan
    (inv_frame identity u32; the differ O1 normalizer named u32
    arm + the O2 alias — raw passthrough would name-join "value"
    vs "raw" and fabricate 2 field-level findings; the D136 sfx
    precedent). Coverage pins dropped one per scenario: S1/S2/S3/
    S5/S5B/S5C = 1 (move-target-words only), S4/S7 = 3, S6 = 2,
    S8 = 3 (S0 = 0); a new guard asserts blink-cursor never
    appears as any finding. No chains moved (E + engines
    untouched); DESIGN §6a row note + landing-paragraph
    amendments. Verified green on corpus: differ_gate 692s,
    canonical_dump_gate 13, diffharness 76, engine libs 132+76,
    fmt+clippy clean, MANIFEST clean. Queue now: item 1 =
    [BLOCKED] S0 live session (operator), item 2 = the
    differ_gate O2 tiebreak fabrication (W11-prep; the W7/D87
    arbitration path has no gate coverage).
  - 2026-08-23 P4/W6 THE SFX-MASTER-GATE + NO-EXTRACT-LATCH E-GAP
    EMISSION unit COMPLETE — DECIDED EMIT NOW + LANDED (D136;
    decision commit 3e3bace + impl commit cfc6b4c + docs commit
    6967d3c, all PUSHED, worker ec979f34 claim 2): E emits
    sfx-master-gate := constant 1 (T0; sound-on construction
    assumption) and no-extract-latch := u32 count + count zero
    words (T1; MP-lobby-claimed only — D133; count = the
    robot-bank count). The differ normalizes both on E/O1/O2 (the
    latch count-prefixed, the O1/O2 bare span prepended len/4; the
    count field STRUCTURAL). ALL canonical chains RE-BASELINED
    deliberately (S0 dac1cfd17bc7ede3 / S1 a18cb11ac8e4314e / S2
    d6649ce272ad6d96 / S3 f4f5b4351e976ed5 / S4 63ab5ac7679f6de7 /
    S5 8a718339e0702fd6 / S5B b72f57e0b8e7042b / S5C
    de5b80a6177aecdd / S6 c27bff339929339d / S7 b0db22840310e82a /
    S8 29fa2f400a10974b / synthetic 6517d1c0b7169446) — the live
    S0 session (item 1) compares against THESE (its NEXT entry
    carries the supersession note; DESIGN §9 examples updated).
    DESIGN §6a E-gaps list amended (both rows leave it; the D85
    staleness corrected — destroy-family five + T2/T3 staged rows
    emit since W12). Coverage counts UNCHANGED on every scenario —
    both channels carry both rows clean (the cleanest-for-S0
    outcome the decision wanted). Verified: 93 diffharness + 13
    canonical_dump_gate + differ_gate + 76+132 engine lib tests
    green, fmt+clippy clean, MANIFEST clean pre+post. Queue now:
    item 1 = [BLOCKED] S0 live session (operator), item 2 = the
    differ_gate blink-cursor fabrication alignment (the fabricated
    O1 still skips the row the D132 plans actually carry).
  - 2026-08-23 P4/RE THE EXW BANK-CELL TWIN CROSS-CHECK unit
    COMPLETE (D135; commit 115e240 PUSHED, worker 9a48b338 claim
    2; docs-only, RE-EXD-MAP §5g-bis): all 17 §5g leftover
    aliases pinned with 1:1 reader-count parity on every cell —
    the two mission walks are store-for-store ORDINAL-IDENTICAL
    (EXW FUN_0043a1d3 ⟷ EXD FUN_0004c121, 27 registers same
    order; head walks EXW 0x447bb7.. ⟷ EXD 0x59b83.., 9 stores);
    MIDIGUN dup consumer-less BOTH sides; BEEP5 cells paired BY
    ORDINAL (briefing re-registration twins confirm); ELEV1/2 =
    TRT structure move readers. Engine consequence NONE (SFX
    cells presentation-tier, out of the hashed core); the §5g
    alias ledger is COMPLETE for every bank-walk cell. Method:
    D94's EXW walk independently re-verified (idioms in
    exw-text-objdump.txt + 20 DGROUP strings re-read from
    BEDLAM.EXW); objdump-only, no Ghidra run; MANIFEST clean
    pre+post; registry_anchors 2/2 green. Queue now: item 1 =
    [BLOCKED] S0 live session (operator), item 2 = the
    sfx-master-gate + no-extract-latch E-GAP EMISSION decision.
  - 2026-08-23 P4/RE THE EXD SFX-MASTER-GATE TWIN CENSUS unit
    COMPLETE — **W1 IS NOW FULLY CLOSED (registry gap set
    EMPTY)** (D134; RE-notes commit 5178420 + impl commit d341c65,
    both PUSHED, worker 2a9f1b9f claim 2; LANDED + RE-VALIDATED +
    COUNT-CORRECTED by the respawned slot worker e104cbd0 claim 2
    (commit b0e105a: every family independently re-derived; census
    counts corrected to EXW 18 / EXD 17 — 13 reader sites
    one-for-one; two gloss fixes: 0x43a79e = the options-handler
    drop-flag pair NOT inside FUN_0043a48e, and the "arg order
    swapped" 0x4c593 mispairing retracted); objdump-only from the
    committed exd/exw listings + read-only game-data string
    probes, no Ghidra run; MANIFEST.sha256 clean before AND
    after; 93 diffharness tests + 13 canonical_dump_gate green,
    fmt+clippy clean): the sfx-master-gate EXD twin = [0x10743c],
    pinned by the BOOM-trio twin FUN_00032de9 @0x32df1 ⟷
    FUN_00421e60 @0x421e68 (play twin FUN_0004c584 ⟷
    FUN_0043a48e @0x32f95); EXW 18-site / EXD 17-site censuses
    one-for-one (arrival family five, music-sequencer trio,
    radio-warning consumer, driver-sync, play gates,
    MissionShell volume keys). HEADLINE = the CONFIG DIVERGENCE:
    EXW value = REGISTRY "SOUND" (boot 0x42530a, saver 0x4253f3;
    init FUN_0043a144, sole caller GameMain) ⟷ EXD value = FILE
    CONFIG.BDL (init FUN_0004be7d, callers boot 0x2cc70 + title
    0x5b03f); both branch pairs write identical tandem cells
    (sister gate 0x107444≡0x4ede5c, SPEECH 0x10766c≡0x4eb93c,
    arena 0x119620≡0x46ae84, voice-table loop 0x8b938≡0x4eada8
    instruction-exact). BY-PRODUCTS: the FUN_0004c121 bank-name
    walk + the MissionShell-head GRUNT/BEAMIN/THROW/PEXPLODE/
    BIOFIRE/CACODETH/SQUAWK walk + 19 §5g cascade aliases
    (volume 0x1081f0≡0x4ddb2c, music gates 0x107578/0x107570,
    drop-flag 0x1195f4≡0x46ae78, the six bank-cell
    trios/quads). REGISTRY/PLANS: gap set EMPTIED with a hard
    no-gap check; dbx-plan emits the T0 row everywhere; ALL 12
    capture plans regenerated (S0/S0W included — deferred
    7→6/10→9/21→20/24→23); E's W6 list untouched (sfx stays a
    documented E-gap like no-extract-latch — the emission
    decision queued). Queued: item 2 = the EXW bank-cell twin
    cross-check (the §5g leftovers), item 3 = the E-gap emission
    decision.

  - 2026-08-23 P4/RE THE EXD NO-EXTRACT-LATCH TWIN CENSUS unit
    COMPLETE (D133; RE-notes commit fe1d1d9 + impl commit
    85d7954, both PUSHED, worker 36c6f950 claim 2; objdump-only
    from the D132 substrate ghidra-project/exd-text-objdump.txt,
    no Ghidra run; MANIFEST.sha256 clean before AND after; 93
    diffharness tests + 13 canonical_dump_gate green, fmt+clippy
    clean): the no-extract-latch EXD twin = [0xf929c+i*4], 12
    .text sites with 8 readers ONE-FOR-ONE vs the EXW 8-reader
    census (0x19c71⟷0x408ef7 death-anim walk w/ 0x65 + tables
    0x82e5a/0x82e8a; 0x1f4cf⟷0x40e7a1 death-core MP respawn gate;
    0x30c87⟷0x4200db pod animator; 0x5b1cc/0x5b34a/0x5b51c ⟷
    0x449dc8/0x449ee8/0x44a08c MP cycler trio; 0x5b7ea ⟷
    0x44a322; 0x5b89c ⟷ 0x44a3d2 endgame census) + the boot
    memset(0x30) pair 0x2cd41⟷0x41c412. HEADLINE = the WRITER
    ASYMMETRY: EXD-only setter FUN_0005bb71 @0x5bba0 :=1 (the
    DOS MP lobby robot-pick) + lobby type-tally 0x5ba83; EXW
    setter set EMPTY — the §7j.19/§7j.27 writer lists corrected
    in place (four fns are READERS); semantics = per-robot
    CLAIMED flag; engine consequence NONE for the SP corpus
    (reads 0 on both channels). BY-PRODUCTS: 14 §5f cascade
    aliases + robot-bank base 0xf6d34 triple-confirmed via the
    respawn tail ([0xf6dd0]:=1 death-flag ⟷ [0x4c6a80]).
    REGISTRY: watches.toml gap set now {sfx-master-gate} ONLY
    (the last W1 gap = queue item 2, the SFX-master-gate census);
    dbx-plan emits the latch as $robot_count*4; capture-plans
    S1..S8 regenerated. NOTE: .state/PAUSE (operator
    menu-pointer fix) appeared mid-run 20:28 — this unit staged
    diffharness/docs paths only, zero overlap.


  - 2026-08-23 P4/RE THE EXD BLINK-CURSOR TWIN CENSUS unit
    COMPLETE (D132; docs+registry, commit f9986b0, worker
    c653b51a claim 2 — adopted + re-validated at HEAD the
    interrupted 4fe7f1e9 WIP, substrate committed as
    tools/exd-relod.py 8f641d3; objdump-only from the
    committed-tool relocation listing, no Ghidra run, no corpus
    write; MANIFEST.sha256 clean before AND after;
    registry_anchors 2/2 green, dbx-plan 31/31, fmt+clippy
    clean; PUSHED): the blink-cursor EXD twin = [0x10e108],
    EXACTLY 7 .text sites one-for-one with the §7j.59/D131 EXW
    census — writers: the three idle-arm strips 0x1cef1/:=
    0x1cf2c/:=0x1cf72 (⟷ 0x40c1d7/0x40c217/0x40c254, warning
    posts via FUN_00034972 ≡ FUN_004239ef, size gates
    [0x11958c]>1/>2) + the impact clear 0x34f7f (⟷ 0x423fef,
    in the shell-resolver FUN_00034d89 ≡ FUN_00423e1c) + the
    MissionShell reset 0x59842 (⟷ 0x447871); readers: the
    portrait gate 0x186dc (⟷ 0x407428, in FUN_000180a1 ≡
    FUN_004072bf — identical (frame&3)+0x51 sprite + literal
    1/2/3 x-dispatch 0x1F0/0x222/0x254, y=0xD, 0 AND >3 draw
    nothing) + the chase gate 0x34e25 (⟷ 0x423e91 — identical
    ([base]+[cursor]−1)·0xA8 kind-vs-player-type arithmetic);
    idle table 0x8105c {400,300,200,5000} byte-identical.
    BY-PRODUCTS: the SELECTION-TRIPLE LABEL-SWAP correction
    (0x11954c ≡ 0x46cbdc selected slot / 0x11955c ≡ 0x46cbd4
    squad base / 0x11958c ≡ 0x46cbd8 size — all three cells
    mapped, W1 gap closed) + TEN §5e cascade/asset aliases
    (salvo latch 0x1081fc; 8-shell bank 0x8f0b4 with the
    record grammar pinned BOTH sides {x@+0,y@+2,fall@+4,
    start@+6,valid@+8} — landing-run correction of the first
    draft's 0x8f0b2 convention; map-overlay 0x1075bc; zoom
    0x107448; idle table 0x8105c; GENERAL.BIN ptr 0x1074fc;
    + the FUN_00034972/00034d89/0003552e/000180a1 twins).
    watches.toml filled, dbx-plan emits the 4-B cell,
    capture-plans S1..S8/S5B/S5C regenerated; the registry
    gap set is now {sfx-master-gate, no-extract-latch} only.
    Engine consequence NONE. Queued: item 2 = the EXD
    no-extract-latch twin census, item 3 = the EXD
    SFX-master-gate twin census.
  - 2026-08-23 P4/RE THE [0x4dc5d0] BLINK-CURSOR PRODUCER
    CENSUS unit COMPLETE (D131; docs-only, §7j.59, commit
    64543b6, worker 0329338f claim 2; objdump-only from
    ghidra-project/exw-text-objdump.txt, no Ghidra run, no
    corpus read; manifest clean before AND after;
    registry_anchors 2/2 green; PUSHED): [0x4dc5d0] = the
    BOMBARDMENT-WARNING squad-slot selector. EXACTLY 7 .text
    sites (whole-objdump grep): 5 writers — the three UNROLLED
    idle-arm strips 0x40c1d7:=1/0x40c217:=2/0x40c254:=3
    (each after its DANGER/BOMBARDMENT warning pair
    FUN_004239ef(0xC+k,k)+(0xF,k); k=0 ungated, k=1/2 size-
    gated [0x46cbd8]>k; shared tail +0x70:=0 +
    [0x4de658]:=0x80 + the 8-shell scatter) + the shell
    impact-completion clear 0x423fef (per-record tail, also
    frees the record; FIRST impacting shell clears) + the
    MissionShell per-mission reset 0x447871; 2 readers — the
    §6c.6d portrait gate 0x407428 (LITERAL x-dispatch:
    1/2/3→0x1F0/0x222/0x254 sprite (frame&3)+0x51 GENERAL.BIN
    y=0xD; 0 AND >3 both draw NOTHING, >3 dead-defensive) +
    the §7j.54 chase-camera gate 0x423e91 (arithmetically
    indexes the bank [0x46cbd4]+(cursor−1)×0xA8 — PROOF the
    value is a 1-based slot index). VALUE = the ENDANGERED
    robot's squad slot+1 (item-6 "selected slot" gloss
    corrected — SP coincidence only; MP writes the tripped
    robot's own slot). DISJOINT from the 0x4dc5d4 effect-row
    array (allocator scans 0x4dc5e0+k*0x10 only); §6c.6d
    "sprite-list field" renamed. LIFECYCLE 0 → arm → first
    impact → 0; ordering resets the idle counter. ENGINE
    CONSEQUENCE NONE (SP-UI presentation, zero sim reads/RNG);
    S1 blink-cursor-from-spawn now STATICALLY decidable =
    constant 0 on every corpus scenario; DESIGN watch +
    hypothesis rows annotated. Queued: item 2 = the EXD
    blink-cursor twin census (§7j.59 as the anchor template;
    closes the last sidebar-family W1 gap).

  - 2026-08-23 P4/RE THE [0x4ede34] DEATH-WIPE/TEMP-VIEWPORT
    CENSUS unit COMPLETE (D130; docs-only, §7j.58, commits
    0909683 + c67b007 + c9e3810, worker 27b33f6c claim 2;
    objdump-only from ghidra-project/exw-text-objdump.txt, no
    Ghidra run, no corpus read; manifest clean before AND
    after; registry_anchors 2/2 green; PUSHED): [0x4ede34] =
    the CLOSING-IRIS death-wipe cell. VALUE GRAMMAR: 0
    inactive; :=1 ARM at selected-robot SP death (sole
    0x40ea8b; MP NEVER arms — posts the sibling marker latch
    instead); +=0x28/frame (sole writer, MissionShell frame
    cluster 0x4480af); terminal :=0x1E0 @0x4480d6 + the
    AUTO-RESELECT pass (last ALIVE player-type squad slot →
    select it, flash :=3, cancel via xor-of-equals 0x448121;
    no eligible mate → parks at 480 = the D129 fail-detector
    conjunct — SP "no cancel" ⟺ squad wiped); cancels = 3
    click-select strips + per-mission 0x44787d. THE TEMP
    RENDER = fill-0 full-screen + centered v×v SHRINK of the
    FROZEN world frame (v := 480−min(cell,479); row routine
    0x401430 = inverse twin of the normal zoom's 0x4013e8;
    FUN_00403938 head 0x403952 skips its render body during
    the wipe) — a 13-frame closing iris 479×479→1×1, user
    zoom save/restored. SIBLING [0x4ea8f8] = the MP
    death-position marker countdown (:=0x20 @0x40e7ef, dying
    x/y/z → the §7j.20 selected-anchor ring 0x4c71c4 —
    consumer = the §7j.54 chase camera; decs in the
    FUN_00403938 head; zeroed in tandem at every cancel).
    CORRECTIONS: §6c.6e flash "ebx(2)" → 3; §7j.56/B
    0x403952 belongs to FUN_00403938. ENGINE CONSEQUENCE
    NONE (presentation-only; iris grammar recorded for future
    E render parity). Queued: item 2 = the [0x4dc5d0]
    blink/effect-list producer census (the §6c.6d open
    producer, fed with this unit's reset-block facts).

  - 2026-08-23 P4/RE THE ROBOT +0x9C DEATH-FLAG READER CENSUS
    unit COMPLETE (D129; docs-only, §7j.57, commit 6a3abcd,
    worker 18039414 claim 2; objdump-only from
    ghidra-project/exw-text-objdump.txt, no Ghidra run, no
    corpus read; manifest clean; registry_anchors 2/2 green;
    PUSHED): BOTH PRODUCERS PINNED = 1 (SP tail 0x40eac0
    edx=1; MP respawn tail 0x40e82a edi=1 — the queue
    "MP-respawn reset" was a MISNOMER: the respawn re-init
    does NOT clear +0x9C, the MP slot stays death-flagged,
    harmless because the sole reader is SP-only). SOLE READER
    = the SP SQUAD-WIPE FAIL DETECTOR FUN_0044764c (sole
    caller MissionShell 0x44870d gated [0x4dc67c]==0 =
    extraction incomplete): walks squad [0x46cbd4]..+
    [0x46cbd8]−1, first +0x9C==0 → alive ret 0; all dead ∧
    [0x4ede34]==0x1E0 (death wipe at terminal 480) → fail
    sequence → MissionShell ret 3 (fail/debrief; ret 2 =
    launch) — +0x9C = the MISSION-FAIL liveness oracle,
    distinct from +0x7C alive / +0x78 hp. LIFECYCLE CLOSED:
    no zero-writer exists; the clear = the mission-staging
    WHOLE-BANK ZERO-FILL FUN_00402965(ecx=0x7E0, edi=0x4c69e4)
    @0x40cd38 (0x7E0 = 12·0xA8 — NEW FACT: the bank is 12
    slots; the only immediate-load of 0x4c69e4 in the binary;
    no save-load bulk copy). §7j.55 SIDEBAR QUESTION ANSWERED
    NO ([0x46ccec] = a flash-countdown, sole reader 0x407205;
    the sidebar pass never reads +0x9C). ENGINE CONSEQUENCE
    NONE (E conforms: death_flag := 1 SP subset + fresh
    per-mission records; already a T1 robot-bank differ field
    leaf). Queued: item 2 = the [0x4ede34] temp-viewport
    census (renumbered from 3, fed with this unit producer/
    consumer facts).

  - 2026-08-23 P4/RE THE [0x4edbd8] CAMERA-GATE CELL + [0x4ede54]
    ZOOM CELL unit COMPLETE (D128; docs-only, §7j.56, commit
    d80fd8b, worker 21e88d3b claim 2; objdump-only + read-only
    string/import probes of BEDLAM.EXW — .idata parsed to name
    IAT 0x4f010c = RegQueryValueExA; no Ghidra run, no corpus
    write; manifest clean both sides; registry_anchors 2/2
    green; PUSHED): [0x4edbd8] = the "ACTIONPAN" value of the
    REGISTRY key HKCU\Software\Mirage\Bedlam\1.00 — 4-site
    census (the two §7j.54 readers 0x4039b0/0x40b875 + the
    boot loader 0x42535c + the saver read 0x42545c; NO
    game-state writer; .bss; bounds [0,1] DEFAULT 1 = pans
    ENABLED on default installs); the config family is
    REGISTRY I/O (FUN_0044ed40 RegCreateKeyExA / FUN_0044ede4
    bounded loader writing the cell directly, default-on-
    absent / FUN_0044ed98 RegSetValueExA self-heal /
    FUN_0044eee0 REG_SZ create-if-missing) — the "CONFIG.BDL"
    gloss RETIRED (zero binary refs; on-disk CONFIG.BDL/
    OPTIONS.BDL = DOS leftovers; TITLEMENU corrected history-
    preserved). [0x4ede54] = the VIEWPORT ZOOM height (clamp
    [240,480] backbuffer rows) — 26-site census: ±0x10 zoom-key
    handler (FUN_0042034c tail 0x4204ea..0x420548, scan
    0x4E/0x0D vs 0x4A/0x0C), MissionShell leftover-edx init
    0x447883 (benign — ≥480 dispatches 1:1), the [0x4ede34]
    temp save/restore pair in FUN_00401107; readers = the Q16
    magnify zoom blitter (FUN_00401107, cells 0x45405c..68),
    the camera-recenter speed (cursor−240)·v/480
    (0x40b89e/0x40b8c5), the cursor un-zoom mappers
    (0x4106a1/0x4106d4/0x419a41). [0x4ede34] census pointer
    recorded (9 sites, identity open — follow-up queued).
    DIFFER: zoom = no rows (no corpus keypresses, zero RNG);
    ACTIONPAN = one live-channel confund recorded (the S0
    session fingerprint step should record [0x4edbd8] + the
    five sibling config cells once; D128 folded into queue
    item 1). Queued: item 2 = the robot +0x9C death-flag
    reader census (D129); item 3 = the [0x4ede34] temp-
    viewport/cinema census.

  - 2026-08-23 P4/RE THE 0x4ea238 MARKER FAMILY + [0x4de658]
    CENSUS unit COMPLETE (D126; docs-only, §7j.54, commit
    51800a0, worker ed78ecdc claim 2; objdump-only, no Ghidra
    run, no corpus read, manifest clean both sides; ADOPTED +
    re-verified interrupted same-item WIP): the "8-jittered-
    marker scatter" = a FALLING-SHELL salvo — bank 0x4ea238,
    8×10 B records {x, y world-px, fall-z 0xFF −0x20/frame,
    start-delay 0x20+2i, valid}; writer = the robots() idle arm
    0x40c25e..0x40c351 (1 RandA/shell, x-jitter ±0x3F only, y
    fan py−0x80+i·0x20, tile-bounds drop); resolver
    FUN_00423e1c (MissionShell 0x447ffa; the "selection
    chaser" gloss RETIRED): fall → get_z_pos ≥ z → SIX kind-6
    debris + NINE FUN_004244a1 5000-damage blasts (3×3 patch)
    + cursor clear; renderer 0x4066e4..0x4067a6 (GENERAL.BIN
    0x12C, 32 px/frame descent). SIBLING: FUN_004245c9 = the
    15-frame CHASE-CAMERA OVERRIDE STAGER (0x4de648/4c/50 +
    0xF→0x4de654; consumer FUN_00403938 0x4039b0..a42 swaps the
    0x4c71c4/c8/cc anchor slot; 4 callers: door 0x422427,
    trigger 0x422e55, artillery 0x41173a, bombardment
    0x423ed5) — the "wall-strip redraw" gloss family corrected
    in place. [0x4de658] = the salvo COOLDOWN latch, census
    closed (arm 0x40c27f / gate 0x40c18b / dec 0x423e25..32 /
    clear 0x447877; 0x442ba7 = D89 alias). D125 ARBITRATED:
    OFFENSIVE bombardment centered ON the idle robot (SP:
    selected robot only; thresholds {400,300,200,5000} frames;
    ordering resets +0x70) — §7g.5 "reinforcement ARRIVAL"
    RETIRED. No engine consequence (no corpus scenario reaches
    the idle threshold). Queued: item 2 = heat-machine warning
    family (D127); item 3 = the [0x4edbd8] camera-gate census
    (D128).

  - 2026-08-23 P4/RE THE FUN_004239ef SFX-MESSAGE DISPATCHER unit
    COMPLETE (D125; docs-only, §7j.53, commit 38a8463, worker
    d1578d5c claim 2; objdump-only, no Ghidra run; read-only
    corpus probes of BEDLAM.EXW DGROUP + the six LANGUAGE.*
    files, manifest clean both sides): FUN_004239ef = the
    RADIO-WARNING poster — 4-channel queue 0x4eb954 (stride
    0x28: 8 id+1 words, insert idx wrap 8, voice handle +0x24;
    dedupe; ids 0x19..0x1B flush their channel; ch 0/1/2 =
    squad slots, 3 = system drained first); consumer
    FUN_00423a85 (MissionShell 0x447ff5/frame): voice leg
    (text-only 0xF/0x29, gates 0x4eb93c/0x4ede5c/0x4ede58,
    A/B take = RandA bit0 off 0x4ee014+8·id, 0x44c8c4 vol
    0x7f00, handle ret+1; still-playing poll keeps the slot)
    + consume leg (roll 4×0x26 display ring 0x4ea13c, stage
    0x46c18c+id·0x30 text, typewriter render tail). The 53-id
    map is CORPUS-NAMED from the LANGUAGE.* [WARNINGS]
    sections (all six locales, 53 records; GameMain loader
    0x41c2ff; [MENU_ITEMS] sibling → 0x46af5c); all 55 call
    sites reconciled. Corrections: §7f.6 "select SFX" gloss
    (the 0xC+k/0xF pair = the DANGER-TARGETTED/BOMBARDMENT
    warning), §7j.37 "SFX ids, not text messages" (both),
    §7g.5 content note (announcement = targeting warning per
    corpus; mechanism unchanged). FORMATS §22 = the LANGUAGE.*
    container grammar. No engine consequence (the spoken
    line's RandA draw joins T3/T4). Queued: the 0x4ea238
    marker family + [0x4de658] census (arbitrates the §7g.5
    tension) + the heat-machine warning family.

  - 2026-08-23 P4/RE THE DEBRIS ARRIVAL-SFX PAIR unit COMPLETE
    (D124; docs-only, §7j.52, commits 01d380b + 2728351, worker
    a553aa84 claim 2): FUN_00421e60 (118 B, 11 callers inside
    the FUN_00420608 kind legs) = the BOOM1/2/3 spawn trio
    ([0x4ede58]≠0 gate, RandB idiv-3, cells 0x4edf64/68/6c,
    play priority 2); FUN_00421dec (116 B, k2/k8) = the
    RICOCHT1..4 quad (RandB&3 jump table @0x421ddc, cells
    0x4edf98/9c/a0/a4, priority 1 — one steal class below
    BOOM); every cell named via §7j.30. RNG CORRECTION: item
    4's "RandA()%3" named the wrong draw — the pick is RandB
    (0x4029b6/0x4ede4c; RandA 0x402975/0x4ede48 only gates
    k11's ~50% al&1 play) — corrected in place, history
    preserved; pick = T4 unmodeled, k11 gate = modeled RandA
    draw-count. TRIGGER: all 13 sites fire at DEBRIS-STAGE
    time (before the record fields; "arrival" = arrival on the
    field); 12/13 = in-map bounds recheck of the raw Q5 args
    then UNCONDITIONAL call; k11 alone adds the RandA&1 gate
    (two RNGs on one leg). Kind→leg map re-verified byte-exact
    vs jump table @0x4205b8. Caller census complete (raw-dword
    scan → zero refs). CORPUS REACH: k5 via apply_damage is
    the only reachable producer → the only reachable arrival
    SFX is k5's e60 leg @0x421364 (one RandB + one BOOM at
    the death position); FUN_00421dec unreachable. Adjacent:
    third sibling FUN_00421ed6 = GRUNT1/2/3 trio (callers
    0x413ba0/0x413f2a = the §7j.42 engage juice) — its
    [identity open] gloss closed in place (2728351); the
    arrival-SFX family is four decode-complete members.
    Engine consequence NONE today; the beyond-k5 E-side stager
    draws one RandB per staging (T4). Verified: registry_anchors
    2/2 green, manifest clean before AND after, objdump-only
    (no Ghidra run), PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 =
    the FUN_004239ef SFX-message dispatcher unit (17 cited
    call sites, zero body decode — the id→cell map is the
    deliverable; D125).
    NEXT: the FUN_004239ef dispatcher unit (item 2).

  - 2026-08-23 P4/RE THE FUN_00419756 IDENTITY unit COMPLETE
    (D123; docs-only, §7j.51, commit 224188f, worker 9a23356a
    claim 2): the TRT-bolt class-3 probe = a first-alive
    ROBOT-BANK OCCUPANCY BOX — walks 0x4c69e4/0xA8 (count
    [0x46ccbc], ALIVE +0x7C≠0), first record with
    |Δ(x>>8)|<0x10 ∧ |Δ(y>>8)|<0x10 ∧ |z@+8 raw − z>>8|<0x20
    (all axes Q5-normalized: ±<0.5 tile lateral, ±<1 level z; a
    BOX not octile; the FUN_004197d4 robot lane shares the
    identical box; robot z@+8 stored Q5 makes the raw compare a
    scale match). NOT critters/TRT structures/tile words; sole
    caller 0x4123ae. THE CLASS-3 VERDICT: the "hit an actor but
    no robot damage" leg CONFIRMED and stronger — NO damage
    query of any kind on the path (disburser → kind-8 debris +
    state := 0), so ALIVE ROBOTS are a pure BLOCKER for the 0x66
    bolt; the (d+1)·300 damage is exclusively the class-2
    terrain contact (the §7j.50 residual closed). Two §7j.50/6
    gloss fixes: the probe takes all THREE args (z = the record's
    unstepped z); "vz≠0 → break" = skip-height-probe-only
    (substeps continue; spawn vz 0x14 = a ~2-frame terrain-arming
    delay); the write-back reverts the contact substep BEFORE the
    class dispatch → class-3 debris pre-contact. Engine
    consequence NONE today (T2-class); the future E-side TRT fire
    routine must reproduce the blocker box + pre-contact debris +
    zero damage (death-position divergence otherwise). Verified:
    registry_anchors 2/2 green, manifest clean, objdump-only (no
    Ghidra run, no corpus read), PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    debris arrival-SFX pair FUN_00421e60/FUN_00421dec unit
    (7j.11 item 4, renumbered from 3).
    NEXT: the debris arrival-SFX pair unit (item 2).

  - 2026-08-23 P4/RE THE PROJECTILE-TYPE-0x69 DAMAGE-TABLE unit
    COMPLETE (D122; docs-only, §7j.50, commits 897f524 + e5596c7,
    worker 6bb948aa claim 2): FUN_00419aff ELSE PATH DUMPED — no
    memory table (inline jump tree; else = default eax=1 via 4
    fall-through stubs + the 0x418aa1 cross-function
    shared-epilogue gadget, 5 arms: 2 default + 3 carrying the
    d≠2 products 50/300/75·(d+1)). THE 0x69 VERDICT: the
    per-level BEAM column (k7 close-combat state @0x4135a2,
    {z=6, TTL 0x18}) re-keys its impact to the LITERAL 0x65
    (50/100/200 by d, terrain-only, PER-FRAME at the blocked
    level via the probe-counter oscillation k++/k−−); NEVER
    robots; no caller ever passes 0x69. The "(d+1)·300" key =
    the TRT-bolt state 0x66 alone (FUN_00417698 @0x417a5c,
    guided stepper, classes 1/2/3; also never robots —
    FUN_004197d4 admits 0x65/0x67/0x68 only, the 0x67/0x68 via
    own-state keys). Complete 25-site state census: 5 producers /
    12 zero-writes / 4 readers; tick dispatch = table 0x411ffc on
    state−0x65; disburser 0x69 arm = silent. Engine consequence:
    the future E-side k7 leg = per-frame 0x65-keyed terrain DoT,
    no robot damage; 0x66 TRT bolt terrain-only. Verified:
    registry_anchors 2/2 green, manifest clean, objdump-only (no
    Ghidra run, no corpus read), PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    FUN_00419756 identity unit (the §7j.50 class-3 probe; 126 B,
    1 caller, genuinely open), 3 = the debris arrival-SFX pair
    FUN_00421e60/FUN_00421dec unit (7j.11 item 4).
    NEXT: the FUN_00419756 identity unit (item 2).

  - 2026-08-23 P4/RE THE FUN_00440dc2 IDENTITY COMPLETE (D121;
    docs-only, §7j.49, worker 21c18e9e claim 2): FUN_00440dc2 =
    the BRIEF OBJECTIVE-MINIMAP SNAPSHOTTER (sole caller
    FUN_0043dc65 = the per-objective brief panel inside the BRIEF
    screen FUN_0043d00b, GameMain 0x41c4d5, ret 2 = launch):
    stages the 7×7 restamp list + materializes TOT→mirror + ZEROES
    the BRIEF's OWN 0x64000 backbuffer (FUN_00440a2d), draws the
    8-z iso stack dest−z·0x5000 (FUN_00440c34 = the REAL owner of
    the 7j.36 sites 0x440d1c/0x440d93), then a plain 2× downsample
    bb[(64+2r)·0x280+64+2c] → 256×256 cache [0x46cbb0] (alloc
    0x10100) + flag [0x4dc6c0], consumer = the flag-gated
    palette-remap blit FUN_00402a28 @0x43d9a2; objective bank =
    24×14 B @0x4e9628 (+0/+2 marker x/y, +4/+6 TOT row/col, +8
    counter, +0xA latch; parser 0x43e5b1..0x43e7b2). The caller
    census is CLOSED (one site + zero data refs — raw dword scan);
    the "jmp into the caller" = Watcom shared-epilogue gadgets
    (0x43c801/0x43c802/0x43f49e multi-entry pop variants). The §1
    mid-frame/terrain-pass ordering question CLOSED BY SCREEN
    LIFECYCLE: FUN_00403938 runs only under MissionShell
    (0x447c9b/0x448094) — the FUN_00440a2d/0x440c34 family NEVER
    runs in-game; 7j.26 gloss corrected ([0x4ede24] = per-screen
    cell reuse: BRIEF 49×12 list vs mission 1296×12 viewport
    cache from FUN_0041d954). Engine consequence NONE (BRIEF
    outside the P4 diff scope; no new watch rows). Verified:
    registry_anchors 2/2 green, manifest clean before AND after
    the read-only string/dword probes, objdump-only (no Ghidra
    run), no corpus write, PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    projectile-type-0x69 damage-table unit (7j.18 low-priority
    residue; pre-queue grep per D118 — genuinely open).
    NEXT: the projectile-0x69 damage-table unit (item 2).

  - 2026-08-23 P4/RE THE MISSIONVIEW §5d TAIL COMPLETE (D120;
    docs-only, §7j.48, commit dd8d5e2, worker 328b7651 claim 2;
    adopted + validated the interrupted predecessor WIP in
    RE-EXW-MISSIONVIEW.md §5d): §5d item-1 = TELEPORT.BIN beam
    (10 imgs, 0x46af38 — not "shield"; clamp 0..9 fits), item-3 =
    SHIELD.BIN (4 imgs, 0x46af44 — not "variant"; RandA()&3 spawn
    + (+1)&3 shimmer); TELEPORT/SHIELD/ROBNUMS alloc (FUN_0041d954
    @0x447860: 0x6d60/0x1b58/0xbb8) + LoadFile (FUN_0041df10
    @0x447b3f) at EVERY MissionShell head — SP included, NO gate;
    ROBNUMS.BIN = DEAD DATA (sole reader = its own load site
    0x41dffe); the MP name plates draw TINYFONT (0x46cdb0, 118
    glyphs, ASCII−0x21) at `sx + u32[0x4e44c8+id*4] + 6·i`, gate
    [0x4edb88]≠0 @0x403fb9 (SP never), filter g ≤ 0x40, centering
    table = 32−3·strlen per id (writer 0x447ce0..0x447d85: toupper
    + c−0x21 from raw names 0x4e43e0); the Backlog "unstaged-flush"
    clause RETIRED — no bank-zero skip anywhere in enqueue/flush
    (an unstaged bank would FAULT; can never occur) so E needs no
    unstaged-skip logic. Verified: registry_anchors 2/2 green,
    manifest clean before AND after the read-only GAMEGFX header
    probes (TELEPORT/SHIELD/ROBNUMS/TINYFONT = 10/4/9/118),
    objdump-only (no Ghidra run), no corpus write, PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    FUN_00440dc2 identity unit (the scroll/camera restamp drawer's
    own frame: caller census + frame flow; pre-queue grep per D118
    — drawer half pinned, caller census unpinned).
    NEXT: the FUN_00440dc2 identity unit (item 2).

  - 2026-08-23 P4/RE THE TOT PLANE-6/7 SEMANTICS COMPLETE (D119;
    docs-only, §7j.47, commit dc6f5bf, worker f29066bd claim 2):
    planes 6/7 of the .TOT word stack are ORDINARY z-levels 6/7
    (tall-structure tops, per-level sprite ids — ZONEA/M1 (17,25)
    = [454,1354,1355,1356] at z=4..7); they STAGE and DRAW like
    every other plane (NO z≥6 gate in any consumer: the
    FUN_00403938 restamp z-stack loop runs z 0..7 with a word-only
    restart gate at 0x406891 — every nonzero plane word draws;
    init_tiles stages all 8 planes; the overlay scanner + range
    consumer equally unbounded). Corpus: 36/37 missions, 8 016+
    2 882 words, domain ≡ planes 1..5 (35..1868). The FORMATS §2
    "~2000-entry target-table" hypothesis REFUTED (POS resolutions
    9 217 live/1 681 empty = coincidence; planes 1..5 reach 1868
    too; the words draw as sprite ids). Engine consequence NONE
    (E already stages every nonzero plane word per D107). Verified:
    registry_anchors 2/2 green, manifest clean before AND after
    the read-only corpus probes (TOT/DAT/POS × 37 missions),
    objdump-only (no Ghidra run), no corpus write, PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    MISSIONVIEW §5d tail unit (ROBNUMS name plates + Shield/Variant
    bank staging; pre-queue grep performed per the D118 discipline).
    NEXT: the §5d tail unit (item 2).

  - 2026-08-23 P4/RE QUEUE HYGIENE #3 (D118; docs-only, worker
    e26508a9 claim 2): the queued ".BDG template-bank ↔
    restore-word mapping" item REMOVED AS ALREADY-CLOSED — it was
    stale pre-D96 state copied from the Backlog's RETIRED-D93
    bullet (the closure sat in the Done log + D96 at queue-write
    time; the stale bullet parenthetical now annotated CLOSED in
    place so it cannot tempt a third re-queue). The D96/§7j.32
    closure re-verified genuinely green at HEAD with FRESH
    evidence: the loader bank disk order instruction-exact
    (+0x3E,+0x46,+0x42,+0x4A), the destroy-restore three writes
    instruction-exact (mirror word/seen/DAT volume ← the UNDER
    pair +0x46/+0x4A), the zero-reader census both legs (absolute
    + displacement scans; arena loader-only), and the corpus role
    proof byte-identical from a fresh parser (ZONEA/M1 435 cells:
    b1 434/435 ≡ shipped TOT, b2 11/435, b3 434/435 ≡ shipped
    DAT, b4 155/435, the one miss = the (14,29,z1) overlap cell,
    last-slot-wins). New method note: the TOT word-plane header is
    WORD-unit — byte-unit +4×2 double-counts it (a false 67/435
    first pass this run; the u8 DAT path immune). Verified:
    registry_anchors 2/2 green, manifest clean before AND after
    the read-only probes, no Ghidra run, no corpus write, PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator-gated), 2 = the
    TOT plane-6/7 semantics unit (pre-queue grep performed per the
    D118 discipline). NEXT: the plane-6/7 unit (item 2).

  - 2026-08-23 P4/RE THE FUN_00433980 CASE TABLE + FUN_00424a6f
    MESSAGE SYSTEM COMPLETE (D117; docs-only, §7j.46, commit fcf97c3,
    worker 0c2df9b4 claim 2): the 7j.19 item-6 residual CLOSED — the
    full per-zone pad-trigger case table (all zones A..G × SP/H2H ×
    missions × .PAD slots → rides/doors/beacons/exits/messages,
    committed as §7j.46 8-bis), the ride-record bank grammar
    (0x4dcdbc stride 0x24, 16 gates), the 21 SP beacon slots, the
    zone-F/G DOOR+EXIT pairs, zone E verified negative, and the
    message system decoded end-to-end (FUN_00424a6f = zone-A-M1-only,
    BOOT_CAMP_%03i sections of the LANGUAGE.* file blob, show-once
    latch 0x4eb5f8+2·id, timer 0x4eaac0/FUN_00425010 ticker + the
    COMMAND-dismissal semantics). The S6/live-message expectations
    are now fully pinned. Verified: registry_anchors 2/2 green,
    manifest clean both sides, PUSHED. Queue: 1 = [BLOCKED] S0 live
    session (operator), 2 = the .BDG template-bank ↔ restore-word
    mapping unit. NEXT: the .BDG mapping unit (item 2).

  - 2026-08-23 P4.2/debris-physics COMPLETE (D115; the §7j.44 RE
    decode d467471 + the engine leg cebc178 landed by predecessor
    a5ef2370 which died at session end mid-re-baseline; the
    uncommitted gate pins were ADOPTED + INDEPENDENTLY re-verified +
    COMPLETED by continuation worker 07ce0c25 claim 2, commits
    b2c89af (the five-chain re-baseline + the damage-lane
    assertions) + c4af24b (docs)): the tick FUN_00420549
    (delay/anim/free lifecycle + the phys gate, MissionShell
    epilogue slot) + the pass FUN_0040de9c in bedlam-core — the
    +0x20 phys word is a COUNTDOWN (the 0x454510 table disproof
    closes 7j.11/5), mag = kind==12?25:2, knock_mult = min(phys,3),
    radius = min(16·phys+0x20,0x60); the robot lane (the
    FUN_0040db9e dispatcher — damage + facing −1 + knock, five-k5
    death tail), the terrain-gated critter lane (the §7j.24
    register-gloss correction), the POI lane E-only documented.
    RE-BASELINE: S3 9a11efa03baafb64 (mine/grenade expiry k12
    chunks — NOT only destroy scenarios move), S4
    35fa3a9234cbff37, S5C 786fd87565b67f4a (case-3 consume flips
    to the gunner; the +2500 heal stays exact), S7
    ecdce5472df6a324, S8 44d806b81bd1b1ff; S0/S1/S2/S5/S5B/S6
    BYTE-IDENTICAL. Debris-damage observability: corpus_s4 (the
    widened cascade + the freed-ring lifecycle), corpus_s7 (the
    standing gunner's chunk-field schedule), corpus_s8 (the
    burst-window chips). Verified this run: workspace 54 suites
    green, fmt+clippy clean, manifest clean both sides, PUSHED.
    THE E-SIDE PRODUCER SURFACE FOR THE DIFFHARNESS IS COMPLETE
    (W1..W9 + W12 + debris physics; W10/W11 + the live DH-G1
    verdict remain, operator/external gated). Queue: 1 =
    [BLOCKED] S0 live session (operator), 2 = the RE-EXW-SIM §9
    remainder (FUN_00440e45 identity + robots() extra-phase/state-1
    producers). NEXT: the §9 remainder unit (item 2).

  - 2026-08-23 P4.2/W12-S7 COMPLETE (D113; the §7j.41 decode
    984a078 + the engine platform-dynamics family ea2f259 + the
    scenario leg b9cbcf3 by predecessor 56d80c42 claim 2, which
    died before the differ/plan/docs legs; this run adopted the
    pushed state + completed them, worker 0b66f6a6 claim 2, commits
    4c6c068 (differ) + 13bae85 (plan) + the docs commit): S7.scen =
    the platform-dynamics lifecycle in ONE ZONEA/M1 run — the
    FUN_00422600 zone-code trigger build (slot 74 (3,57,2), the
    gunner's quadrant blocks 3 of 8 tiles), the corrected weaken
    ring gates (300→150 spread, 150→75 site latch), the destroy
    (k7 census 5/20), the armed creep (first 199 tile f449, 22
    creep tiles by f1240, tail static). Grammar v1.6 `platforms = 1`
    (THE PER-FRAME RandA GATE-DRAW finding: the original draws one
    gate RandA per frame even unarmed — an E-side stream gap on
    S0..S6 until a deliberate re-baseline; O1 needs no staging).
    1361 records, chain b41db389f3ad8947 + double-run
    byte-identical; corpus_s7 + differ_gate S7 row (2 S1-class +
    the debris/splash E-only pair, zero gaps) + capture-plans/
    S7.json byte-pinned (5 command injects; the platforms arm note
    in _e_staging); the §8 ledger rows rewritten with the §7j.41
    corrections. S0..S6 chains re-asserted byte-identical; fmt+
    clippy clean, manifest clean, PUSHED. Queue: 1 = [BLOCKED] S0
    live session (operator), 2 = W12-S8 the critter-engagement
    producer + scenario unit. NEXT: W12-S8 (item 2).

  - 2026-08-23 P4.2/W12-S6 COMPLETE (worker 4d92bb13 claim 2,
    commits bcf5396 + 0545e2e, D112; the §7j.40 decode 631bd28 +
    the engine extraction family edafd02 by predecessor 8d32d85d —
    its interrupted harness WIP adopted + completed): S6.scen =
    the .PAD step-on extraction run — COMMAND-driven walk (bit0
    SELECT: state 1 + target, no pending order; a click never arms
    the beacon), pad slot 0x12 = (19,70,0) the census GROUND pad
    (the queue's pad-8 gloss was stale — slot 8 is LEVEL 1;
    deviation in D112), two legs cross the pad mid-walk, the
    sub-tick probe arms the beacon + halts the walker, the
    same-frame window-0 deploy → descent → sweep (state 3→5) →
    jittered dwell → departure → complete f69. 75 records, chain
    c96f0735df1059ea + double-run byte-identical; the .PAD
    terminator bug fixed (dead `x == -1` on a u16 read — 114 live
    slots now, the D86 rejection bites); corpus_s6 + differ_gate
    S6 row (2 S1-class + the E-only dropship row, zero gaps) +
    capture-plans/S6.json byte-pinned (3 injects). S0..S5C chains
    re-asserted byte-identical; 54 suites green, fmt+clippy clean,
    manifest clean, PUSHED. Queue: 1 = [BLOCKED] S0 live session
    (operator), 2 = W12-S7-prep the platform-dynamics producer
    unit. NEXT: W12-S7-prep (item 2).

  - 2026-08-22 QUEUE HYGIENE #2 (worker e444e1cd claim 2, commit
    6b6274f, D111): the claimed queue item 2 (the MISSIONVIEW §8
    water-flag/anim remainder) was found ALREADY CLOSED at HEAD —
    D100/§7j.35 (bee4336+60f7d3b) closed it ~15 units before S5C,
    but 105d9aa's queue note re-queued it by mistake. Closure
    re-verified green with independent spot-checks (objdump censuses
    for 0x456ca8 [2 readers/0 writers] + 0x4edbd4 [3 writers] exact;
    file image at 0x552a8 = the static {0..7,7..0} const byte-exact;
    registry_anchors 2/2; MANIFEST clean both sides). No engine/doc/
    tool change beyond the queue + D111. Queue: 1 = [BLOCKED] S0
    live session (operator), 2 = W12-S6 the extraction scenario
    unit. NEXT: W12-S6 (item 2).

  - 2026-08-22 P4.2/W12-S5C COMPLETE (worker 82d5a27f claim 2,
    commit c27b3db, D110): the case-3 OBSERVABILITY variant landed —
    S5C.scen (the S4 artillery pattern spends the walker to 1256
    pre-order: a gunner marker ON the walker's tile, loadout
    9/0xA/0xB, the frame-1 command; the §7j.23 robot lane hits a
    +0xF00 robot from 4 list-0 pairs/burst = 3744 at f32 on walker
    AND gunner, clicker −624 at f36, all at state 0/3 pre-order) →
    case 3 at f41 heals the EXACT +2500 UNCLAMPED (1256→3756; S5B
    could only show the dispatch). The gunner walks one robot
    behind, never heals (the negative control); the burst rings
    detonate the destroy chain cascade (232 off-corridor cells —
    S5B's six-cell census does not hold for S5C; the cascade rides
    the SAME aliased T1 rows, differ = the 2 S1-class findings).
    55 records, chain e0999fcb3455d3ef + double-run byte-identical;
    differ_gate S5C row; capture-plans/S5C.json byte-pinned (4
    inject rows). NO engine change — S0..S5B chains re-asserted
    byte-identical; 54 suites green, fmt+clippy clean, manifest
    clean, PUSHED. Queue: 1 = [BLOCKED] S0 live session (operator),
    2 = the MISSIONVIEW §8 water-flag/anim remainder (re-queued
    from D99), 3 = W12-S6 the extraction scenario unit. NEXT: the
    water-flag/anim remainder (item 2).

  - 2026-08-22 P4.2/dbx-plan-tiers COMPLETE (worker 33a28c84 claim
    2, commits a784e49 + 690d8b0 + 4db7ba1, D109): dbx-plan compiles
    the T2/T3 tiers — S3 (T2) and S4 (T0/T1/T3) capture plans land
    (capture-plans/S3+S4.json, byte-pinned; the two aliased weapon
    banks as FULL spans 0x5460/0x6A4; every unaliased T2/T3 row an
    explicit _deferred gap, never emitted — the debris/splash refusal
    pinned by tests). THE COUNT-PREFIX GRAMMAR: capgen watch rows
    gain a `prefix` sub-row (count cell first, concatenated — flow
    probe GREEN headless, all four dbgprobe modes re-verified) and
    dbx-plan emits Prefixed for trt-array/object-instances; the
    object row now dumps the FULL 2000*0x14 bank (the D108 ZONEB
    .POS live-past-dead holes — without it a live capture drops 32
    live objects AND fails row normalization structurally on
    trt/object; robot-bank stays the bare span by contract). BONUS:
    the D103 loadout _e_staging mask was invalid-JSON hex — now
    decimal (S3, the first compilable loadout plan, surfaced it).
    S1/S2/S5/S5B regenerated; S0/S0W untouched. Workspace 54 suites
    / 632 tests green, fmt+clippy clean, manifest clean, PUSHED.
    Queue: 1 = [BLOCKED] S0 live session (operator; the item's D109
    note records the re-stage requirement), 2 = W12-S5C. NEXT:
    W12-S5C (item 2).

  - 2026-08-22 P4.2/W12-S5 COMPLETE (worker c2aba48b claim 2, commits
    66ad013 (docs/RE notes first) + 3626010 (engine+grammar+tests+
    scenarios+plans), D108): the S5/S5B PICKUP SCENARIOS are landed —
    grammar v1.5 `zone = "B"` (the GameHost::stage_episode_slot D51
    seam standing in for the campaign/save-load shells) + `pickup = 1`
    (the mission's own .TOT through stage_pickup_surface AFTER the
    destroy staging + the §7j.12/6 hazard stamper — the original
    load order). S5 = the row-21 z3 corridor (cases 1/2/4, the only
    c1+c2 co-walkable spot in the corpus; 16 records, chain
    a4659f25d453b6a1), S5B = the row-10 z3 corridor (case 3 + 4× c4
    + the (76,9) diagonal side cell; 19 records, chain
    93e976587a98d2a1) — the TWO-SCENARIO SPLIT is forced by the
    order-window semantics (cases 1 and 3 are 61 tiles apart; a
    second order needs the first cleared = all-alive-state-3
    impossible mid-scenario, or the 0x197-frame window ≈ 407 idle
    frames × ~340 KB/record of REAL mirror rows). The mirror rows go
    REAL on S5-class runs (every ZONEB tile active; S4's
    empty-mirror divergence closes; the S4 chain untouched). DIFFER
    FIXES the ZONEB surface exposed: the O1 zone-row normalizer maps
    cell−1 (§6a zone convention — the guest cell is the 1-based set,
    E canonical the 0-based index), the O1 object-instances walk
    covers the WHOLE span (ZONEB .POS carries live slots past dead
    holes: 1128 max slot / 1096 live — a count-bounded walk dropped
    32 live objects), and the field-union join is hash-indexed (the
    mirror rows carry ~170k fields/frame; the linear union was
    quadratic — 5+ min → 3 s). dbx-plan compiles both tiers T0/T1/TS
    with the zone+pickup seams in _e_staging (strict JSON for
    multi-entry stagings; S0/S0W/S1/S2 plans byte-identical);
    capture-plans/S5+S5B committed. VERIFIED: workspace 54 suites /
    629 tests green (S0..S4 chains byte-identical:
    8901789a88cf61fe / 1c4e7b4c9d9b0947 / 809f4961b7757da4 /
    e29f76f5585401e1 / 2ddd15ea50c8a14d), differ_gate S5/S5B rows,
    fmt+clippy clean, registry_anchors green, manifest clean both
    sides, PUSHED. Queue: 1 = [BLOCKED] S0 live session,
    2 = dbx-plan-tiers, 3 = W12-S5C (the case-3 hp-observability
    variant). NEXT: dbx-plan-tiers (item 2).

  - 2026-08-22 P4.2/W12-S5-prep COMPLETE (worker f32193a2 claim 2,
    commits ad43c12 (RE §7h.5) + 7a2dfeb (engine+tests), D107): the
    E-side PICKUP PRODUCER is in the engine — stage_pickup_surface
    (init_tiles TOT-word + DAT-gated-seen staging + the zone/set
    cell), the clear→move→test→fire consume protocol in robots_phase
    (the latch clear is UNCONDITIONAL like EXW; the ZONEA walk never
    latches — all mission_corpus_gate hash pins survived), fire_pickup
    (DAT byte := 0 / mirror word := table C / seen := 1 → dispatch),
    apply_pickup case-4 score/money draws + the MissionShell fold,
    cases 8/9 host-seamed. §7h.5 settled the range/floor table
    INDEXING (zone_index 0-based — the contiguous 7-dword/0x1C-stride
    DGROUP family) and FLAGGED the pre-existing destroy.rs zone-table
    head-slot question (corpus-dead; S5/S7 differ arbitrates). Corpus
    gates: ZONEA ZERO fire traffic (the D99 census re-derived live +
    pinned; the staged walk hash-trace-identical) + the ZONEB/M1
    positive control (152 live cells). S0..S4 canonical chains
    BYTE-IDENTICAL (2ddd15ea50c8a14d in the pinned set); workspace
    green, fmt+clippy clean, registry_anchors green, manifest clean.
    Queue: 1 = [BLOCKED] S0 live session, 2 = W12-S5 (the S5.scen
    unit — grammar v1.5 pickup key + the ZONEB zone-staging
    question), 3 = dbx-plan-tiers. NEXT: W12-S5 (item 2).

  - 2026-08-22 QUEUE HYGIENE (worker 78203f4f claim 2, D106): the
    W12-S4 closure (b8925a9, D105) was RE-VERIFIED green at HEAD
    (differ_gate 7/7, destroy_gate 16/16, canonical_dump_gate
    full-chain assert incl. S4 2ddd15ea50c8a14d, weapon_fire_gate
    28/28, registry_anchors 2/2, MANIFEST clean) — no engineering
    change. The five stale "2. DONE ..." blocks left in NEXT.md's
    Now section are REMOVED (all were duplicated in the Done log);
    nudge-free-items.py now skips a first-word DONE marker
    (+ test-nudge-queue.sh case) so a finished item can never be
    respawned. Queue renumbered: 1 = [BLOCKED] S0 live session,
    2 = W12-S5-prep (E-side pickup producer), 3 = dbx-plan-tiers.
    NEXT: W12-S5-prep (item 2).

  - OPEN 2026-08-22 (P4.2/W12-S4 THE S4.SCEN + CANONICAL-CHAIN
    unit COMPLETE, commit b8925a9, D105; engine+tests by a
    predecessor session left uncommitted by session death —
    ADOPTED + FIXED + VALIDATED + COMMITTED + PUSHED by
    continuation worker 65f39dff claim 2): the destroy-family
    slice is live END-TO-END on E + the differ + the plan
    compiler. Grammar v1.4 `destroy = 1` (an EQUIVALENCE seam —
    the original loads the same .BDG/.POS/.TRT natively, so no
    O1 write exists to fabricate; dbx-plan records the key in
    _e_staging with the empty-mirror pre-S5 divergence noted).
    S4.scen = ZONEA/MISSION1, 49 records, chain pinned
    2ddd15ea50c8a14d byte-identical double run: the TRAP leg
    (resolver-100 no-score destroy at the anchor + 5×k12 +
    sel-9 k20 + 3×3 splash + the restore into the empty-staged
    mirror), the ARTILLERY leg (marker gunner 9/0xA/0xB at its
    own tile — ring-0 TURRET rubble stamp, rings 4..6 CHAIN
    cascade, the faithful gunner self-damage), the SURVIVOR leg
    (monotone multi-hit subtract, never destroyed). Canonical
    rows: 23-B objects keyed by .POS slot, 20-B TRT, the shared
    grid spans, COMPACT-ACTIVE mirror (nonzero-tile filter on
    BOTH channels), FULL-bank debris/splash = E-only T3
    coverage rows. Differ: the O1 guest normalizers (0x14/
    0x20-stride walks, dead-slot skip) + Structural count
    words; differ_gate S4 = cross PASS-WITH-NOTES (4 E-only
    rows, zero field gaps). MissionShell destroy-score fold
    (zero without staging — no-inject re-asserted: S0/S1/S2/S3
    chains BYTE-IDENTICAL 8901789a88cf61fe / 1c4e7b4c9d9b0947 /
    809f4961b7757da4 / e29f76f5585401e1). Continuation fixes to
    the WIP: the trt fabricator slice overrun, the count-cell
    stride guards, the mirror compact-tail parser layout cross,
    clippy erasing-op/doc-lints, the dbx-plan destroy leg.
    Workspace 617 tests green, fmt+clippy clean,
    registry_anchors green, manifest clean both sides. A live
    S4 capture needs the dbx-plan T3-tier unit (S3's T2
    precedent). NEXT: the W12-S5-prep E-side pickup producer
    (item 2).

  - OPEN 2026-08-22 (P4.2/W12-S4-prep THE E-SIDE
    IMPACT-APPLICATION + DESTROY-RESOLVER PRODUCER unit COMPLETE;
    RE 7j.38/7j.39 by worker 460d294e claim 2, commits dcc8865 +
    acf09ff; engine+tests built by worker d57a4dec claim 2 but
    left uncommitted by session death — ADOPTED + INDEPENDENTLY
    RE-VALIDATED + COMMITTED + PUSHED by continuation worker
    3e93a4b1 claim 2, D104): the E-side destroy family is live
    in bedlam-core::destroy — the mission-load STAGING (the .BDG
    ≤282-row EOF-exact parser pinned against all 37 shipped
    files / the .POS 2000×16-B instance list with the
    footprint+hp re-stamp / the .TRT turret bank, host-seamed
    per the D51 pattern; the 0x7d2/0x7d3 hazard stamper + the
    TOT-mirror/seen banks), the two RESOLVERS (FUN_0041a894
    objects incl. the platform 0x7d4 destroy/weaken entry +
    FUN_0041bc1c structures with the rubble stamp), the destroy
    TAIL (objective notify → the GER gate → the +0x46/+0x4A
    template RESTORE → the five-effect loop with the §7j.38
    draw table 8/8/8/8/8/0/0/72/9 → the score award → the four
    perimeter CHAIN walks with the §7j.39/5 corrected geometry),
    the widened 20-kind debris stager (the 11 seq tables + the
    LRU allocator + the scorch classes), the splash stager +
    the water-z probe, the script blast FUN_004244a1 (the robot
    box-lane), the tile-0x62 trap lane, both disbursers (the
    §7j.14 0xF-persist / 0x65-clear corrections), and the
    weapon-tick IMPACT LANES wired in the §7j.39/2 call orders
    (bullets/shell/0x24/0x29 floors — 0x29 REVERSED, the
    artillery burst pairs + the k6/k11 gates, the mortar 3-cell
    at the post-halving offsets, the class-0 quadrant body, the
    projectile type-1/2/3 branches). NONE of it enters
    state_hash (the W6 split — debris/splash are T3 rows).
    The D104 DIFFER CONTRACT: the armor-pad-reads/
    typedb-fade-byte rows canonicalize BOTH channels to the
    last-nonzero prefix (E's lazily-materialized +0x18 bank vs
    the full-size guest grid — identical content now
    canonicalizes identically; keeps the widened scorch writers
    differ-safe). VERIFIED: destroy_gate 16/16 (synthetic
    CI-safe core + the corpus BDG census row), weapon_fire_gate
    28/28, S0/S1/S2 chains BYTE-IDENTICAL (8901789a88cf61fe /
    1c4e7b4c9d9b0947 / 809f4961b7757da4 — the no-inject
    invariant), S3 re-pinned ONCE e29f76f5585401e1 BEFORE any
    O1 S3 capture exists (D103's dbx-plan T2-tier unit precedes
    a live S3), workspace tests green, fmt+clippy clean,
    registry_anchors green, manifest clean both sides.
    E-GAPS documented in D104/7j.39/8 (the splash tick body,
    the platform spread ring + creep, the trigger producers,
    the effects bank, the critter lane, the debris physics, the
    objective at-zero arm tail, all SFX). NEXT: the W12-S4
    S4.scen + canonical-chain unit (item 2) — volleys onto
    staged ZONEA destructibles + the destroy-family dump rows.

  - OPEN 2026-08-22 (P4.2/W12-S3 THE S3.SCEN + CANONICAL-CHAIN
    unit COMPLETE; engine+tests by worker 0bef7bae claim 2,
    commits 774eed4 + ae8be6b + a928ad8 + af5c2b8; the
    differ/registry leg left uncommitted by session death —
    ADOPTED + VALIDATED + COMPLETED by continuation worker
    16ebe0c4 claim 2, commits 51fa937 + f211684 + d407ca6,
    D103): the weapon-fire T2 slice is live END-TO-END on E +
    the differ + the plan compiler. (a) EXD twins PINNED and
    REGISTERED (ghidra-project/exd-projbank.txt): weapon-anim
    bank 0x980d4 (free-slot finder FUN_00023295 bound
    0x5460 = 400·0x36 EXACT; tick twin FUN_000212f2 with the
    0x17 3-clone split) + projectile bank 0x10e174 (tick twin
    FUN_00022a52 50-slot walk; the +0x1A clamp-0..7 / +0x1E
    −1-countdown tail words beyond the 7 E-modeled fields —
    O1-only coverage surface parsed on BOTH channels, a live
    tail is a finding, never silence). (b) the COMMAND payload
    off-by-one fixed (+7/+9/+0xB — the ae8be6b decompile
    re-verification). (c) grammar v1.3 `loadout` staging key
    through stage_robot_weapons (D51/markers discipline;
    dbx-plan RECORDS the seam in _e_staging, never fabricates —
    f211684). (d) canonical emits BOTH banks as u32 count + the
    FULL records (the record field order IS the guest layout;
    out of state_hash — the W6 split); S3.scen = 8 COMMAND
    volleys over 133 records / 132 frames covering every
    inline-spawn class (artillery 9/0xA/0xB, mines 0xF/0x13,
    grenades 0x1A/0x1F, rocket 0x24) + cadences, the artillery
    disarm, the per-record ammo gate, the auto-rearm cascade,
    the full spawn/active/free lifecycle; bullets/shell/0x17/
    homing are documented E-gaps (their producers are the
    unmodeled AI-order families + the mortar — live records
    surface as the differ's coverage class). Chain
    49193732e6dbc546 pinned, double-run byte-identical.
    (e) differ normalizes both banks on BOTH channels through
    the same field walk (E count+records fail-loud; O1 the bare
    span); differ_gate S3 = cross PASS-WITH-NOTES (exactly the
    2 E-only rows, zero field gaps, zero T2 diffs). S0/S1/S2
    chains re-asserted BYTE-IDENTICAL (the no-inject invariant
    holds). VERIFIED: workspace tests green, fmt+clippy clean,
    registry_anchors green, manifest clean both sides. NEXT: the
    W12-S4-prep E-side impact-application + destroy-resolver
    producer unit (queued item 2) — the 7j.25/7j.32 decodes are
    complete upstream, the engine work is staging + impact +
    restore + effects; a live S3 capture additionally needs the
    dbx-plan T2-tier unit (it still refuses T2 scenarios).

  - OPEN 2026-08-22 (P4.2/W12-S3-prep THE E-SIDE WEAPON-FIRE
    COMMAND PRODUCER unit COMPLETE, worker 95ab9206 claim 2,
    commits 5cf5078 + 5f2963a + 642be37, D102, engine+tests):
    the FIRST W12 producer family is in-engine. (a) RE NOTES
    FIRST (5cf5078, §7j.37 — dumps-only from the existing local
    artifacts, one read-only SINTABLE.BIN corpus probe, no new
    Ghidra run): the consumer dispatch decode-exact (fire gates
    mask ∧ cooldown==0 ∧ ammo≠0 VERIFIED; artillery 1×
    type=id pos+0x100 z=(z+0x15)<<8 cooldown 0 + UNCONDITIONAL
    disarm; mines 2/4/6× 0xF/0x13 4-RandA-draw shape class 4;
    grenades 4/6× 0x1A/0x1F 3D vz ttl 0x32∓/＋RandA&0xF trail:=0;
    rocket ttl 0 cooldown 5 arc=angle-pair NO RandA; auto-rearm +
    loop-exit recharge; the bit0 pointer-bump quirk documented);
    SINTABLE.BIN = the 256-word byte-angle sine ramp (cos/sin =
    word lookups at a/a−0x40, thresholds = words[2..66] of the
    same array); bullets = 2 tested sub-steps, NET TWO committed
    steps, free ONLY at tick>99 (corrects 7j.22); artillery
    duration table indexed BY TYPE; homing steering exact (diff·4,
    2·(sin>>4), LEFT-first avoidance, left-OOB climbs). (b) ENGINE
    (5f2963a): bedlam-core/weapon.rs — the command ring + the
    400×0x36 weapon bank + the 50×0x22 projectile bank (NOT in
    state_hash; the S3 T2 watch surface), the consumer + the
    per-type ticks (bullets/shell/artillery/ballistic incl. the
    0x17 3-clone split/rocket/homing) + enemy_tick + the pinned
    damage table (incl. the d=2 flat override); Robot gains
    weapons[7]+mask (host-seamed); AngleTable carries the full
    sine array; advance_frame = the MissionShell order (consumer
    top, the 4× enemy pass after the 6 phases); canonical.rs
    Step::Command CONSUMED (≥14 B fail-loud) + difficulty staged.
    (c) VERIFIED: weapon_fire_gate 28 tests (no-inject inertness
    incl. zero RandA draws, all flags/gates/bookkeeping, every
    spawn family's shape, all tick types, the frame pipeline);
    the corpus S0/S1/S2 chains re-run BYTE-IDENTICAL (the
    no-inject invariant pinned three ways); workspace 100% green;
    fmt+clippy clean; registry_anchors green; manifest clean both
    sides; PUSHED. E-GAPS documented (the AI-family spawn
    internals, the mortar family, the impact APPLICATION = S4,
    disbursers, SFX/messages = T4, the 0x22 producers, FUN_004197d4,
    the trail ring). NEXT: the S3.scen/canonical-chain unit (the
    loadout-staging scenario key + the weapon/projectile-bank dump
    rows + the differ normalizers + the chain pin) — queued item 2.
    (ADOPTED + INDEPENDENTLY RE-VALIDATED 2026-08-22 by continuation
    worker bae2e091 claim 2 — the 95ab9206 session died after push,
    before the state bookkeeping: workspace tests 100% green incl.
    weapon_fire_gate 28/28 + canonical_dump_gate 5/5 (the S0/S1/S2
    pinned chains re-asserted) + differ_gate + registry_anchors;
    fmt+clippy clean; manifest clean before AND after; HEAD ==
    origin/main == 642be37.)

  - OPEN 2026-08-22 (P4/RE THE [0x4ede1c] BIN-BANK CONTENT
    CONSUMERS unit COMPLETE, worker d6b238f4 claim 2, commit
    cd304c6, D101, docs-only): the 7j.16 residue is CLOSED.
    Container grammar pinned instruction-exact + corpus 11/11
    banks: u16[bank+0] = sprite COUNT → WRITE-ONLY cell 0x46cdb8
    (no .text reader; blits mask id&0xFFF); directory entry =
    bank+2+4·id, sprite = entry + u32[entry] SELF-relative;
    records = u16 fmt/dy/dx/gate/rows + stream (gate==0 or
    rows==0 → draws nothing; FUN_0040167a reads gate but IGNORES
    it; ALL real terrain = fmt 7; each bank carries exactly 9
    fmt-0 scratch records). MISSIONVIEW §4's directory gloss
    CORRECTED (the self-relative form; §5c was right); FORMATS
    §18 cross-ref assumed→VERIFIED. Reader census complete (12
    [0x4ede1c] sites): loaders + the terrain loop (4 ESI loads) +
    the scroll-restamp drawer FUN_00440dc2 + FUN_00401010 = the
    9-sprite RADAR STAMP, the bank's ONLY runtime content writer
    (5× downsample + 2:1 iso deshear of the 480×480 viewport at
    the camera into scratch ids u32[0x454b00+4·set]..+8
    {1168,1773,1592,1168,58,58,1773}) — and the STAMP IS
    VESTIGIAL: its stub records carry gate=rows=0 forever (the
    stamp writes only image+0x20..), LNK is IDENTITY on all 63
    family ids in all 7 zones, and no code ever draws them — the
    A/B/C/D TOT references render NOTHING (the stamp still runs
    every present; zero observable effect). §0b VERDICT: the
    bank is render-only presentation — NO differ watch row for
    the bank/directory/0x46cdb8 (all below the emptiness-rule
    threshold); the state surface stays the TOT words/type-DB
    mirror rows; E models only the 7j.35 seam list (u8-RLE +
    per-tile remap). Deliverables: RE-EXW-SIM §7j.36 + 2 new +
    2 rewritten ledger rows + MISSIONVIEW §1/§4 corrections +
    FORMATS §18 + D101. registry_anchors green; manifest clean
    before AND after the corpus probes; PUSHED. NEXT: the
    W12-S3-prep E-side weapon-fire COMMAND producer unit (queued
    item 2, engine+tests, unattended-safe) — then the
    operator-adjacent W10/W11 tail.

  - OPEN 2026-08-22 (P4/RE THE MISSIONVIEW §8 WATER-FLAG/ANIM
    REMAINDER unit COMPLETE, worker 57ba8753 claim 2, commit
    bee4336, D100, docs-only): §8 is now FULLY closed (the last
    open §8 row). The anim sequence u32[0x456ca8] is a STATIC
    DGROUP const {0,1,2,3,4,5,6,7,7,6,5,4,3,2,1,0} — a 16-phase
    ping-pong over the 8 PALTRAN ramps, ZERO .text writers, 2
    readers (0x40691a/0x406a2c in the terrain loop); the STATIC
    branch indexes the SAME ramps by the +0x18 scorch byte (scorch
    n → ramp n — scorch darkening IS ramp selection); the
    anim-window (+0x1b/+0x1c) branch is ZONEG-only. The water
    flag [0x4edbd4] ≡ 1 in EVERY mission: sole persistent writer
    = the campaign-boot defaults FUN_004252c0@0x4252d8 (:= 1,
    every "New Single Player Game"); one scoped save/restore
    (0x41c649/0x41c65a) around the SELECTOR screen FUN_0043e7d4;
    NO config/options/save/MP writer — the water-off arms of the
    0x12d/0x12e/0x12f dispatches + the remap-XLAT gate are DEAD
    CODE in shipped play (E may hard-code water-ON). 7j.12
    zone-table off-by-one CORRECTED (all tables set-indexed 1..7
    by the RAW [0x4edd8c]; entry 0 = the previous array's tail;
    one contiguous u32 array at 0x1C strides). CORPUS VERDICT:
    water sprites stage ONLY in ZONEB/M1 (12 cells), ZONEB/M6
    (78), ZONEC/M4 (33), ZONED/M1 (1), ZONEF/M7 (4824) —
    ZONEA/M1 ZERO in both the sprite range and the
    platform/splash word range (which appears in NO shipped
    file); side finding: 44 0x7d2 hazard cells in ZONEA/M1 → the
    load stamper pre-stages the 0x460dfa hazard grid in every
    gate run (the 7g.5 hazard path is live). Engine seam: NONE
    (D98/D99 pattern — the corpus path does not fire;
    DrawParams.remap stays the host seam, pixel-side). P4.2
    hooks on the DESIGN S5 row: the water leg shares the S5
    zone-walk staging and must run ZONEB/M1|M6, ZONEC/M4 or
    ZONEF/M7. Deliverables: RE-EXW-SIM §7j.35 + 2 new + 1
    rewritten ledger rows + MISSIONVIEW §8.2 closure + §1/§3/§4/
    §5c refreshes + DESIGN S5 row + D100. registry_anchors
    green; manifest clean before AND after the corpus probes;
    PUSHED. NEXT: the [0x4ede1c] BIN-bank content consumers
    (queued item 2) — then the operator-adjacent W10/W11/W12
    tail.

  - OPEN 2026-08-22 (P4/RE THE 7h.3 PICKUP TILE-WORD PRODUCER unit
    COMPLETE, worker f461ea05 claim 2, commit 187f0aa, D99,
    docs-only): the producer chain is decoded end-to-end —
    init_tiles@00407e11 stages EVERY nonzero TOT plane word into
    the 0x4796bc mirror (the DAT byte gates ONLY the seen flag;
    the §2/§7j.16 "word needs DAT==0" gloss corrected — that gate
    is the FUN_00440a2d restamp path); get_z_pos writes the
    trigger triple {z,x,y}→0x4dc688/8c/90 at FOUR sites gated on
    the probed DAT byte == 3 (last-write-wins, no auto-clear); the
    SOLE consumer = the robots() move-toward-target clear(0x40bef2)
    →robot_move(0x40bf06)→test(0x40bf0b)→fire protocol (DAT byte
    := 0, mirror word := floor word 0x454a90+4·set, seen := 1,
    MP-only 0x4dc6ac/b0/b4 stage, then FUN_0040eba0) — any of the
    9 probes of one move sub-tick collects (±0.34..0.38 tile
    reach). The terrain set [0x4edd8c] = zone_index+1 CONFIRMED
    (the path zone letter is 'A'+set−1; boot 1, campaign episode
    advance ++ walking sets 1..7 = zones A..G, save-load restore,
    MP picker MP-only). CORPUS VERDICT: ZONEA/M1 (set 1) stages
    ZERO pickup cells — S0/S1/S2 NEVER fire the machinery (80
    type-3 cells exist but their words are set-2/5 shapes, inert
    under set 1); ZONEB (set 2) stages 601 pickup cells, ZONEF
    (set 6) 149, zones C/D/E/G none. The engine seam stays
    host-seamed BY CORPUS FACT (D98 pattern); P4.2 hooks on the
    S5 row (the pickup leg must run ZONEB/ZONEF + the E-side
    producer list: TOT words in Terrain + set + the latch/consume
    protocol + apply_pickup). Deliverables: RE-EXW-SIM §7h.4 +
    ledger rows + FORMATS §2/§4 + DESIGN-DIFFHARNESS S5 + D99;
    registry_anchors green; manifest clean both sides; PUSHED.
    NEXT: the MISSIONVIEW §8 water-flag/anim remainder (queued
    item 2) + the [0x4ede1c] BIN-bank content consumers (item 3) —
    then the operator-adjacent W10/W11/W12 tail.

  - OPEN 2026-08-22 (P4/RE THE MISSIONVIEW §8 TYPE-DB TAIL
    PRODUCERS unit COMPLETE, worker a42c6027 claim 2, commit
    3530df5, D98, docs-only): the mirror-record tail is fully
    enumerated — +0x19 = the door/scenery TARGET-TAG byte,
    +0x1A = {bit7 phase, low7 frame counter}: the 15-frame
    SLIDING-DOOR machine FUN_00423081 (MissionShell epilogue
    tick @0x44808f; DAT door-frame bytes 0x40+2n even /
    0x5F−2n odd at the walk-down level; nibble wrap → z-stack
    DROP/PUSH-UP finish pairs; counter stops at low7==+0x19;
    state≥3 rects auto-cycle with a 0x14 pause, states 1/2 are
    the 86 pad-script stepper calls; renderer slide bias
    −nibble·0x500 @0x406c5c). The 0x4dcae8 rect grammar
    RESOLVED {+0 state,+2 x0,+4 y0,+6 w,+8 h,+0xA variant,+0xC
    cd,+0xE sfx} (7j.12 word@+2 qualifier + 7j.21 w/y/h
    permutation corrected). Reader anchors: scorch→damage
    0x40bc60, fire-anchor 0x4110cb, renderer adjacency
    0x406bd6/0x406bf9, neighbor test 0x4237c5/da, the second
    +0x1B/+0x1C objective stamp/clear walks; +0x1D zero traffic
    CONFIRMED (71-site absolute census). RE-EXW-SIM §7j.34 +
    2 rewritten + 1 new ledger row + MISSIONVIEW §2/§8.1
    CLOSED + FORMATS §2 + D98; registry_anchors green; manifest
    clean; PUSHED. The 7h.3 pickup tile-word producer is now
    UNBLOCKED (queued item 3). NEXT: the pickup producer unit —
    then the operator-adjacent W10/W11/W12 tail.

  - OPEN 2026-08-22 (P4/FORMATS THE .BLD RECORD WALK unit
    COMPLETE, worker fc88ecf3 claim 2, commit 6897326, D97,
    docs-only): the last FORMATS structure gap is CLOSED with a
    negative-result headline — "BLD" (case-insensitive) occurs
    in ZERO shipped executables; there is NO .BLD loader
    ("SAVED.BDL" @0x4597d6 = the savegame). .BLD = the
    EDITOR-SOURCE format compiled into .BDG (record j ≡ BDG
    non-empty record j; same H/hp/chain/type heads + the SAME
    four template banks; BLD 197 = 282 − 85 EMPTY rows ZONEA/M1).
    Grammar VERIFIED (RE-EXW-SIM §7j.33 + FORMATS §17):
    length = 137 + 64·W·H + variable tail (subsumes 201+64k);
    four 16·W·H template-bank slots == the BDG banks; name@+0x60;
    NOT self-delimiting (no terminator/count); zero fill EOF;
    7 286/7 907 records byte-validated (ZONEB/G + ZONEF M6
    desyncs = variable tails, bounded+documented). RUNTIME
    FILE-FAMILY CENSUS landed: FUN_0041dc5a (.TOT/.DAT/.CGR/
    .BIN/.MIN/.LNG-or-.LNK gate [0x4eba1c]/.PAD, tag table
    0x4587d9..0x4587fc) + path builder FUN_0044670c
    (EDITOR\ZONE\MISSION); editor-only set .BLD/.CTG/.COL/.MAP/
    .PTH/.TXT (FORMATS §0.2; .CTG never loaded). BONUS: zone D
    ships mission-level BLDs (§0 fixed); zone-level BLDs
    byte-shared A≡F, B≡G. NEXT: the MISSIONVIEW §8 type-DB
    tail producers (queued as the next slot-2 unit — unblocks
    the 7h.3 pickup tile-word producer) — then the
    operator-adjacent W10/W11/W12 tail.

  - OPEN 2026-08-22 (P4/RE THE .BDG TEMPLATE-BANK READER unit
    COMPLETE, worker ce347a0e claim 2, commit 4210f55, D96,
    docs-only): the 7j.25 open item is CLOSED with a
    negative-result headline — the .BDG template banks +0x3E/+0x42
    have ZERO code readers: they are the editor's CURRENT-state
    pair, already baked into the shipped .TOT/.DAT at every .POS
    footprint (bank1 ≡ TOT word / bank3 ≡ DAT byte, 434/435
    ZONEA/M1 cells; the single miss = a real footprint overlap,
    last-.POS-slot-wins) — the runtime spawn-stamp hypothesis is
    RETIRED (RE-EXW-SIM §7j.32 + 3 rewritten + 4 new ledger rows +
    FORMATS §2/§12/§16 + MISSIONVIEW §2 + D96). Loader disk order
    interleaved (+0x3E,+0x46,+0x42,+0x4A); the destroy restore
    consumes only the UNDER pair (+0x46 → mirror plane words
    +2·z; +0x4A → seen=(word==0) + DAT volume low byte), verified
    instruction-exact. BONUS: the 0x1E-B mirror tile-record
    grammar unified (plane words / seen bytes / +0x18 scorch /
    +0x19 variant / +0x1A door / +0x1B/+0x1C OBJECT-HEIGHT pair —
    the MISSIONVIEW §8.1 producer hunt closed / +0x1D unused);
    FUN_0044889a/FUN_00448b80 = the objective-building family
    (zone-7 gate, counter [0x46cce0], at zero SFX 0x28/0x29 +
    extraction-arm cells); .POS word 2 = base z level; TRT death
    stamp = per-zone rubble word 0x454a04. NEXT: the .BLD record
    walk (queue item 3) — then the operator-adjacent W10/W11/W12
    tail.

  - OPEN 2026-08-22 (P4/RE THE HOT-RECT RECORD unit COMPLETE,
    worker aa62f5ed claim 2, commit 5abeaad, D95, docs-only): the
    0x4787c4/0x47879c click-target family is CLOSED — ONE
    0x20-stride record array base 0x4787bc, count [0x46ccd8], cap
    0x77, per-frame reset @0x403a9a (RE-EXW-SIM §7j.31 + 3 ledger
    rows superseding the §7j.16 skeleton). Grammar: +0/+4 world
    corner, +8/+0xC hit-box ORIGIN (picker adds w/2,h/2), +0x10
    z, +0x14 w, +0x18 h, +0x1C type {plain id critter / |0x1000
    robot / 0x2000|id = picker TRT-scan return only}. Writers: 7
    sites ALL in renderer FUN_00403938 — w1 robots is MP-ONLY
    ([0x4edb88]==2 ∧ ≠local player) ⇒ SP click-orders are NEVER
    robot-targeted (E seam constraint); w2-w7 the critter .NME
    draw paths. Readers: picker FUN_00419943 (octile
    FUN_0041ebf8, early-out <4) + dispatcher FUN_00410644
    (MissionShell @0x448021; NEW pins [0x46cc00] type cell +
    [0x4ddb20]&2 order latch; the TRT bit-13 branch resolves via
    the −0xC-bias base 0x4cccec — the 7j.28 "critter" gloss
    corrected to TRT). NEXT: the .BDG template-bank reader unit
    (queue item 2), the .BLD walk (item 3) — then the
    operator-adjacent W10/W11/W12 tail.

  - OPEN 2026-08-22 (P4/RE THE SFX BANK-NAME WALK unit COMPLETE,
    worker 7972b334 claim 2, commit a0f291c, D94, docs-only): the
    bank→name map is COMPLETE — 202 durable assignments, ZERO
    unnamed durable cells across 0x4edfXX/0x46afXX (RE-EXW-SIM
    §7j.30 + 2 ledger rows; raw dump ghidra-project/
    exw-banknames.txt). Key structural facts: SFX cells hold
    VOICE-BASE handles (FUN_0043a36e/FUN_0043a39c = 1-/4-voice
    registers staging through scratch cell 0x46af0c), FUN_0043a48e
    = the play/steal function (position→pan/vol vs listener
    0x4edde4/0x4edde8, steal by the 0x4ee1c2/0x4ee2e2 voice
    arrays), speech = a 53-record {A,B} bank at 0x4ee014 (95
    files; pair slot-order FLIPS at SPCH16; playback bypasses
    FUN_0043a48e), language G-variants share cells (gate 0x4eba1c
    + edition [0x4edd8c]>4 → GRILLA family), palette cells are
    per-ROLE shared slots (0x4edbf8 = current-screen PAL ×6
    names), MIDIGUN registered twice (0x4edf70 consumer-less).
    All prior bank pins re-confirmed cell-exact; corrections none.
    The sec-9 mission-SFX tier's DATA prerequisite is met; the
    tier itself stays unimplemented. NEXT: the hot-rect record
    unit (queue item 2) — then the operator-adjacent W10/W11/W12
    tail.

  - OPEN 2026-08-22 (P4/FORMATS THE .MOFO LOADER unit COMPLETE as a
    NEGATIVE RESULT, worker 0a08a5e1 claim 2, commit 03e8c3b, D93,
    docs-only): the suspected fifth mission extension .MOFO NEVER
    EXISTED — 0x457a4c "MOFO\0" is the dead tail of the fatal
    string "Buggered direction in MOFO" @0x457a3c (zero code refs;
    no ".MOFO" bytes in EXW/EXD; no *.MOFO corpus file; manifest
    clean both sides). The loader-tag family is CLOSED at
    .NME/.TRT/.POS/.BDG @0x457a57..0x457a6d (the 7j.15 gloss
    corrected; FORMATS §0.1 + RE-EXW-SIM §7j.29 + 2 ledger rows
    landed). BONUS: the string's sole consumer FUN_00415490 pinned
    = the mode-9 SEEK per-step target-acquisition dispatcher
    (dual-purpose +0x10 heading/direction, tables 0x415480 +
    0x412ef8, four forward-acquisition cases, the standard fatal
    idiom on direction > 3). NEXT: the SFX bank-name walk (queue
    item 2) — the last unattended P4 slice before the
    operator-adjacent W10/W11/W12 tail.

  - OPEN 2026-08-22 (P4.2/W9 GATES/CI WIRING unit COMPLETE, worker
    cd3ebd73 claim 2, commit 5026afc, D92): the W-series unattended
    tail is CLOSED — the corpus-gated harness set runs in CI as a
    NAMED workflow job (.github/workflows/ci.yml `diffharness`:
    `cargo test -p diffharness` + `cargo test -p bedlam-game --test
    canonical_dump_gate --test differ_gate`). CI PROVES compile +
    the SKIP-CLEANLY property + the corpus-FREE tests (synthetic §6a
    fixture, dump schema, registry anchors, stitch replay, differ
    units); the pinned-chain corpus assertions run on corpus-present
    machines (same commands); original-side runs NEVER in CI
    (desktop-gated, unchanged). THE SWEEP was empirical: a fresh
    corpus-free git clone (faithful CI-checkout sim) +
    `cargo test --workspace --no-fail-fast` found exactly ONE
    non-skipping corpus dependency — menu_gate, 3 of 5 tests
    panicking on the absent corpus via corpus_host()'s expect —
    fixed with the corpus_present() guard (LANGUAGE.ENG marker, the
    file's own pattern). Post-fix: clone run 52/52 targets green;
    workspace run 565 tests green with all 5 menu_gate tests
    executing for real; fmt/clippy clean; manifest clean both sides.
    DESIGN §9 DH-G3/CI-wiring sections + §10-W9 LANDED; DECISIONS
    D92. Recipe recorded: re-run the corpus-free clone test whenever
    a new corpus gate lands (the named CI job enforces it).
    REMAINING P4.2 work is operator-adjacent: W10 (8street O3
    comparator), W11 (Wine/EXW O2 tiebreak), W12 (S3+ scenario
    depth as producer families land) — all need live-session or
    off-repo setup; plus the interactive item 1 (S0 live session).

  - OPEN 2026-08-22 (P4.2/W8-s2 THE S2 ORDER SCENARIO unit COMPLETE,
    worker 7faaeb53 claim 2, commits a9e6964 + 786c9fb, D91): the
    order→walk corpus slice is live end-to-end on E + the differ +
    the plan compiler. GRAMMAR v1.2 adds the scenario-level
    `markers = x,y,z[; ...]` staging key — the walk seam (the
    click-order moves only the OTHER robots in the order radius; the
    clicked robot snaps to spread slot 0, and D89 pins the SP squad
    at 1 robot on EXW/EXD/E alike, so a walk scenario stages its
    walker: E via the EXISTING load_mission(staged_markers) seam, no
    staging-rule change; O1 records it in the plan's `_e_staging`
    field and NEVER fabricates a robot record — the live robot-count
    diff is the scenario seam, not a finding). S2.scen = markers
    18,73,1 (the mission_corpus_gate walker) + `order 21 73 1`;
    canonical chain 809f4961b7757da4 pinned (17 records; arm frame
    beacon window 0x197−1 — the single-robot window-0 clear does NOT
    fire at 2 alive — claims slots 0+1, walker present=1 target
    (22,73) Q5, walk frames 1..6, arrival frame 7 snapped one tile
    short at the (21,73) origin, beacon/claims clear on all-state-3,
    target RETAINED post-arrival); differ_gate S2 row (present=1
    spans both ways through the D90 splice, cross PASS-WITH-NOTES
    with exactly the 2 E-only rows, zero robot field gaps);
    capture-plans/S2.json byte-pinned (order-target 3-cell write at
    frame 1 + `_e_staging`). Workspace 52 suites green (565 tests),
    fmt+clippy clean, manifest clean. NEXT HEADS: the operator S0
    live session (item 1, interactive-gated; S2 plan re-stages the
    same way) + W9 gates/CI wiring (item 3, unattended).

  - OPEN 2026-08-22 (P4.2/W8-prep THE ROBOT-COUNT OVERRIDE PIN unit
    COMPLETE, worker b0656949 claim 2, commit f106cf1, D89, docs-only
    — no new Ghidra run: the EXW disasm was extracted verbatim from
    the 7j.27 exw-text-objdump into local ghidra-project/
    exw-spawncount-asm.txt): ANSWERED — the original SP does NOT fill
    the 0x46cbe0 network-marker override. EXW FUN_0040cca0
    @0x40cd4c..0x40ce23: per_player [0x46cbd8] := zone rule, total
    [0x46ccbc] := per_player; the override is gated on [0x4edb88]!=0
    @0x40cd8d (network sessions only; EXD twin = the mode==0 branch
    of FUN_0001d9cd, instruction-for-instruction). Title menu pins
    SP: "New Single Player Game" @0x43aaa3 sets 0x4edb88=0 ∧
    0x46cbe0=1 (host marker only; MP lobby 1=Coop/2=Head2Head).
    SP ZONEA banks ONE robot in EXW, EXD, and E alike — robot-count
    parity holds, robot-count diffs in SP scenarios are a genuine
    finding class, NO E-side staging seam changes. CORRECTION: EXW
    0x46ccbc = TOTAL (EXD cap 0x11950c twin), EXW 0x46cbd8 =
    PER-PLAYER (EXD 0x11958c twin) — RE-EXD-MAP §5 robot-bank row +
    RE-EXW-SIM §7c.7 fixed (equal in SP; future MP bank dumps bound
    by the cap cell). Faithful quirk: the SP marker write hits
    record[12]+0x2A (stale MRK-copy counter, both twins) — harmless,
    re-stamped/dead-gap, no diff surface. NEXT HEADS: the move-target
    plan-row fill (item 2, unattended, coverage 3 → 0) + the operator
    S0 live session (item 1, interactive-gated).

  - OPEN 2026-08-22 (P4.2/W7-followup THE EXD ROBOT BACK-HALF PROBE
    unit COMPLETE, worker 03be9318 claim 2, commits 455ca41 + 206b776,
    D88): the robot-record canonical coverage is 31 of 34 leaves on
    BOTH raw channels. Two `-process BEDLAM.EXD -noanalysis` probe
    passes (EXDRobotBackhalf{,2}.java; dumps ghidra-project/
    exd-robot-backhalf{,2}.txt) pinned the 23 remaining fields with
    semantic twins EXACT vs EXW (damage applier FUN_0001ef61, spawn
    initializer FUN_0001d9cd with the stat 0x2A/0x2B/0x2C switch,
    robot_move FUN_0001d274, probes FUN_0001e440, portrait
    FUN_000180a1, all-dead sweep FUN_0005961c = the death_flag
    reader). CORRECTION: canonical drop_countdown = raw +0x80 (the
    phase-4/5 gate word), NOT +0x2C (the pod-descent timer, not
    engine-modeled) — both maps rebound. differ.rs maps widened
    (FieldKind enum, i16 armor); S1 differ-gate coverage re-pinned
    2+26 → 2+3 (blink-cursor + move-target-words rows + the target
    trio; pinned chains re-asserted green; workspace 52 suites,
    fmt/clippy clean). The move-target EXTENT is pinned (cap-bounded
    ≤12, 0x60-B span at 0xf75ec covers x[12]+y[12]) — the deferred
    dbx-plan row is fillable (queued). NOTE: EXD decrements
    alarm_ctr(+0xA4) per phase-0 pass — EXW 7g.1 documents no decay
    (evidence gap; divergence-seed candidate for the live S1 diff).
    NEXT HEADS: the W8 robot-count override pin (item 2, unattended)
    + the move-target plan-row fill (item 3, unattended, coverage
    3 → 0) + the operator S0 live session (item 1, interactive-gated,
    S0W calibration hook).

  - OPEN 2026-08-22 (P4.2/W7 THE DIFFER unit COMPLETE, worker
    c594df62 claim 2, commits a9d741f + 04d1d27 + 0dfdb0c, D87):
    the P4.2 comparison core EXISTS — `tools/diffharness/src/
    differ.rs` + the `dbx-diff` CLI. Channel normalizers (E parses
    the §6a canonical grammar; O1 converts raw guest bytes per the
    RE-EXD-MAP §8 EXD field map — only individually pinned robot
    offsets, every other field a metered coverage finding, never
    fabricated; O2 per the EXW RE-EXW-SIM §3/7f/7g table, the
    seed-#1 EXW-front conflict OPEN for W11). MODES: double-run (the
    DH-G1 verdict instrument — identical modulo frame-counter T2 +
    rng T3, draw-count checks still apply) + cross-channel (per-field
    classes + O2 arbitration engine-bug vs original-divergence; the
    `coverage` bucket notes-not-fails). Report + fingerprint
    manifest (git-carried digests; dumps runtime-only). The S0 live-session
    checklist step 4 now uses dbx-diff.

  - OPEN 2026-08-22 (P4.2/W5-pad THE CAPGEN PAD OP unit COMPLETE,
    worker 85dedea3 claim 2, commits fb92286 + b5d1920, D86): the
    PAD step is fully landed on the O1 side — the capgen
    `{op:"pad"}` inject form reads the .PAD slot record from the pad
    bank at the capture-frame stop (999×8 B, loader marks
    active==1/x!=0xFFFF, fail loud), then writes {x,y,z} i32-LE x3 to
    the order-target triple; dbx-plan compiles `pad <slot>` with
    every address registry-derived (static-pad-slots = the READ
    anchor, order-target = the write seam). The §7j.20 extraction-pad
    census is committed in DESIGN §7 (S6 slot picker). Verified:
    `dbgprobe pad` GREEN headless (positive + negative legs),
    gate/inject/flow/walk regression-green, workspace test/fmt/clippy
    green, byte-pinned plans unchanged. The E side still rejects pad
    steps (S6 engine seam, W12 pairs it).

  - OPEN 2026-08-22 (P4.2/W6 THE ENGINE DUMP EMITTER unit COMPLETE,
    design by worker 1f758667 claim 2 / adopted + completed by worker
    36f752cd claim 2, commit 54d781a, D85 + completion addendum): the
    E side of the differ EXISTS — `parity_harness --canonical
    --scenario <S.scen> --out <dump>` drives GameHost over the SHARED
    v1.1 scenario grammar (D82 seam: same runner::Scenario parser as
    O1) and stitches channel-E W3 dumps through the same
    stitch/encode path as O1 captures. T0/T1/TS canonical field maps
    = DESIGN §6a (the W7 normalizer's contract); every unmapped row an
    explicit E-gap. Walk phase accepts ONLY boot steps (difficulty
    seed; the WIP's blanket rejection fixed); command/pad name their
    missing engine seams; P-pause banned mid-scenario. Verified:
    hand-encoded grammar fixture + synthetic-sim run + corpus-gated
    S0/S1 (chains 8901789a88cf61fe / 1c4e7b4c9d9b0947, byte-identical
    double runs) in tests/canonical_dump_gate.rs. NOTE for W8: E
    stages no network-marker override (ZONEA = single-robot squad;
    0x46cbe0 override parity unpinned). NEXT HEADS: W5-pad the capgen
    pad op (item 2, small) + W7 the differ (item 3 — both sides of
    its input contract now exist) + the operator S0 live session
    (item 1, interactive-gated, S0W calibration hook).
  - OPEN 2026-08-22 (P4.2/W5-walk THE SCRIPTED-MENU-WALK DRIVER unit
    COMPLETE, worker 845abdc5 claim 2, commits 59ec9a5 + b67dcaa +
    33b2c17, D84): the human title-menu walk is now scriptable — the
    BPLM boot trap on the frame-counter cell 0x1195f0 doubles as the
    walk driver (one stop per counter-writing screen frame; SMV at
    stop i = screen frame i+1's input; keystore re-arm per input
    because AnyKeyWait consumes on read; anchor BP arms at the LAST
    walk stop). Compiler: walk-phase keystore steps -> stop-indexed
    "walk" plan rows + registry-derived walk_watches calibration trio;
    resolve_at=anchor for ALL dbx-plan plans (fixes the latent D81
    gap: loader statics are mission-load values, the arm-stop read
    was pre-mission garbage). S0W.scen + capture-plans/S0W.json
    committed (draft schedule; calibrates live via per-stop transcript
    comments). capgen plan v3 walk loop GREEN headless (`dbgprobe
    walk`; gate/flow/inject regression-green; 52 tests). The pad op
    queued as its own unit. NEXT HEADS: W6 the engine dump emitter
    (queue item 2, unattended) + the operator S0 live session (item 1,
    interactive-gated — now also carries the S0W calibration hook).
  - OPEN 2026-08-22 (P4.2/W5-followup EXD INPUT-TWIN CENSUS unit
    COMPLETE, worker ef11271c claim 2, commits 79362a9 + 110718d,
    D83): the four §5 seam aliases are PINNED and REGISTERED — EXD
    keystore 0x894d4 (AnyKeyWait twin FUN_00030792 + INT-9 hook
    KeySink + installer memset; ScanToChar tables 0x8077a/0x8097a;
    held-keys counter 0x107534), order target 0x10e0a4/a8/ac +
    order-active 0x10e140 (click-order twin FUN_00021112 + consumer
    FUN_00019ee9 bit1; EXD MissionShell trio position EXACT:
    FUN_00021112 → FUN_0005b066 builder → FUN_00019ee9 consumer),
    command ring 0x9255c stride 0x80 + count 0x119588 (record layout
    byte@+0 marker / id@+1 / spot@+3 / flags@+5 / xyz@+7/+9/+0xB
    EXACT), difficulty 0x119558 (172/236/300 formula + respawn-delay
    table twin 0x81050 in FUN_00023967; writers FUN_0002c6e3).
    Registry rows filled (TI rows now aliased where pinned; the
    emptiness rule narrows to T2-T4), dbx-plan order-target form +
    REAL-registry step tests, S0/S1 plans regenerated (difficulty +
    order-target now dumped). KEYSTATE/ORDER/COMMAND steps compile
    end-to-end for O1 (scratch-verified incl. the remapped arrow
    byte). Divergence seeds 6-7: EXD attack-break gates =
    frame-counter+timer masks vs EXW RandA (inverted mapping — a live
    T2/T3 class); EXD-only staging cells. BONUS T2-ready: projectile
    bank 0x980d4 ×0x36 field-exact. NEXT HEADS: the scripted-menu-
    walk driver unit (keystore alias landed) + the operator S0 live
    session (item 1, interactive-gated).
  - OPEN 2026-08-22 (P4.2/W5 THE INJECTOR unit COMPLETE, worker
    683a65d6 claim 2, commits c443207 + fa31828 + 5e882cd + 28ef5e7,
    D82): the DESIGN §5 injection vocabulary is LANDED end-to-end and
    headless-verified — scenario grammar v1.1 (keystore/order/pad/
    command/boot steps; until-anchor splits walk/mission phases), the
    capgen SMV emitter (boot_writes at the arm stop + frame-keyed
    inject rows applied before the watch dumps → `frame N 1` injected
    flags; the command-ring append op = count read → zero-extended
    record → count bump), and dbx-plan's T1 count-cell compiler
    (robot 0x11958c / TRT 0x11949c / object 0x119554 resolve rows +
    count·stride extents + map-w/h grid exprs; S1.json committed +
    byte-pinned). `dbgprobe inject` GREEN headless (no game; gate +
    flow regression-green). [SUPERSEDED by D83: the seam aliases are
    no longer registry gaps — keystore/order/command/difficulty are
    pinned and the steps compile.] The scripted-menu-walk driver
    (BPLM-on-frame-counter walk stops + mission-start detect) is the
    follow-up unit — now unblocked.
  - OPEN 2026-08-22 (P4.2/DH-G0-live prep, worker fa49e9cf claim 1,
    commits f659db5 + d5550a3 + ee2f0d4, D81): the S0 live-capture
    MACHINERY is landed and headless-verified; only the interactive
    desktop session remains (RUNTIME.md "S0 LIVE SESSION CHECKLIST"):
    capgen plan v2 (BPLM boot trap on the frame-counter cell → SELINFO
    flat-CS guard with loader-stub retry → BP CS:0005A6EB arm → runtime
    resolve of map w/h + TOT/DAT/claim pointer cells → anchor/per-frame
    capture), proven headless by `dbgprobe flow` (real IVT/BDA bytes
    through expression addr/len from a live cell read; no game).
    KEY D81 FACTS: GetHexValue resolves REGISTER NAMES in the default
    MEMDUMPBIN/BP parse path — `CS:001195F0` addressing eliminates the
    numeric-selector parameter entirely (the BP ack echoes it = the
    per-run pin); BP locations resolve EAGERLY at arm time (pre-boot
    arming mis-resolves) while BPLM is LAZY per-instruction (the boot
    trap); SELINFO output rides the logfile (the guard); the staged
    diff conf flips debuggerrun watch→debugger (watch free-runs past
    the parked halt). dbx-plan (tools/diffharness) compiles scenario
    tiers + watches.toml into the plan — every address DERIVED from
    registry rows (anti-ghost asserts; byte-equality test pins the
    committed capture-plans/S0.json); 6 TS rows deferred with explicit
    unpinned-extent reasons; TOT/DAT extents cross-check = 30004/15004
    (the FORMATS-pinned ZONEA/M1 file sizes). EXPECTATION SET: no
    counter reset exists (14 INC sites incl. menu screens) — the live
    double-run is identical-chains-modulo frame-counter/RNG cells
    (T2/T3, DESIGN §6); byte-identical needs the W5 scripted walk.
  - CLOSED 2026-08-22 (P4.2/DH-G0 the O1 CAPTURE-CHANNEL RE-PIN unit
    COMPLETE, worker 4deb0081 claim 1, commits 395180b + d858728 +
    1e7392f, D80): DECISION (a) — O1's instrument = REPO-LOCAL
    SELF-BUILT DOSBox-X at upstream e522642 (the flathub pin's own
    banner commit), configured --enable-sdl2 --enable-debug=heavy
    --disable-sdlnet --disable-avcodec (host lacks SDL2_net; ffmpeg 8
    broke upstream avcodec code; neither touches the harness), built
    out-of-tree under runtime/ (src/ 474M checkout + build/, binary
    144MB sha256 24f71092..., C_DEBUG+C_HEAVY_DEBUG verified in
    config.h; host toolchain recorded as part of the pin: gcc 16.2.1,
    SDL2 2.32.70, ncursesw 6.6, autotools). Flathub runtime STAYS as
    the D29 sandbox baseline. GameLink (b) rejected: client-poll
    real-mode oriented, unproven DPMI model = a second research
    project; O2-ptrace (c) rejected as primary: abandons the DOS-side
    oracle. CHANNEL PROVEN HEADLESS (no game, dbgprobe mode):
    -break-start prompt over a host PTY (the isatty gate),
    BPINT-8/BPLM/RUNWATCH/MEMDUMPBIN/SMV all behaviorally verified —
    incl. SMV linear write + readback (real-mode linear==seg<<4) and
    BPLM arm+fire; 3-frame probe transcript shows real state deltas
    (pre-boot zeros → POST IVT/BDA COM1/COM2/LPT1 → DOS-kernel
    vectors 0070:000e). Driver tools/runtime/dbx-capgen.py (PTY +
    count-based [log]-logfile acks) wired into dosbox-harness.sh as
    `dbgprobe` (unattended-safe) + `diff capture` (FORCE_DIFF_RUN=1,
    interactive-gated, needs the staged capture-plan.json). THREE
    CHANNEL GOTCHAS pinned + baked into the driver (RUNTIME.md "D80
    CHANNEL GOTCHAS"): the [log] logfile is REWRITTEN at debugger init
    (count-match acks, never seek-tail), a permanent PTY drain is
    mandatory (ncurses redraws fill the ~64KB pty buffer → wrefresh
    deadlocks the debugger loop), 1.0s post-ack settle before each
    send (0.01s-gap input stalls tens of seconds). STILL OPEN (the
    live interactive unit): pmode flat-selector proof (INT3 at EXD
    _entry 0x5fbb0, SELINFO/LDT, present-tail BP at 0x5a6eb), cycles
    calibration, first golden S0 dumps + DH-G1 double-run determinism.
    Manifest verified. PUSHED 1e7392f.
  - CLOSED 2026-08-22 (P4.2/W4 the DOSBOX-X RUNNER unit COMPLETE via the
    ticket's split clause, worker d35c7066 claim 1, commits d9a3f77 +
    19c3bdf, D79): (a) unattended-safe slice — dosbox-harness.sh `diff
    stage|run|stitch` (EXD corpus scratch runtime/harness-corpus-exd;
    launch pinned DOS4GW.EXE BEDLAM.EXD — the 269k BEDLAM.EXE is the
    PE32 launcher, the 655k .EXD the LE game image), scenario grammar
    v1 + committed S0/S1 scenarios, DBXCAP v1 channel-agnostic capture
    transcript + zero-dep dbx-stitch bin (anti-ghost guards: registry
    membership, scenario tiers, O1 exd_addr non-empty; anchor+frames
    count contract) producing the W3 dump + JSON digest manifest
    (self-contained SHA-256, FIPS vectors); synthetic replay fixture
    decode-tests pin chain vector 1685e11311ae5b21; workspace
    fmt/clippy/tests green; MANIFEST verified around the corpus read.
    (b) live piece [BLOCKED]-on-DH-G0-channel-repin — RESOLVED by the
    D80 unit above (channel exists; live run stays interactive-gated).
    D79 audit facts (flathub: no debugger, log-only JS; [log] misc
    gate) stand recorded in RUNTIME.md; DESIGN §3/§9/§11 + watch
    skeleton amended.
  - CLOSED 2026-08-22 (P4.2/W3 the DUMP SCHEMA unit COMPLETE, worker
    6f14cea1 claim 1, commit fca6657): the DESIGN §3 dump format
    implemented in the zero-dep crate — tools/diffharness/src/dump.rs
    (schema_ver 1) + src/hash.rs + tests/dump_schema.rs (15 integration
    + 3 in-module tests; workspace build/test/fmt/clippy green).
    Grammar: "BDLD" header {schema_ver, channel 1..4 (O1 EXD/DOSBox-X,
    O2 EXW/Wine, O3 8street, E engine), build_sha256[32], scenario,
    pins} → N × "BDLD" frame {frame_no u64, injection_applied u8,
    watch_count u16, per watch {id, len u32, raw bytes}, frame_digest
    u64} → "BDLT" trailer {frame_count, chain_digest}, all LE.
    frame_digest = FNV-1a-64 over the tag-prefixed canonical frame
    bytes (BDLD domain separation vs StateHash); chain = the D28
    parity_harness construction verbatim (incremental Fnv1a64,
    write_u64 per frame digest) → dump chains are directly comparable
    fingerprints. Encoders registry-driven: canonical watch order =
    watches.toml file order (stable sort), unknown/duplicate ids
    rejected, frame_no strictly increasing (encode+decode), empty
    blobs legal (count-0 banks); identical state ⇒ identical frame
    digests on every channel (tested). decode verifies every digest +
    count + chain + truncation/trailing/magic/bool/utf8. The FNV util
    is a MIRROR of bedlam-core's (a dependency would pull thiserror
    into the zero-dep crate), pinned to the engine's public vectors by
    tests/dump_schema.rs::engine_hash_vectors. Docs: DESIGN §3/§10-W3
    LANDED + DECISIONS D78. PUSHED fca6657.
    Next P4.2 head: W4 (DOSBox-X runner diff mode: scenario → conf →
    debugger automation → D: dumps → digest manifest; S0 headless
    first) — needs the DH-G0 interactive debugger-surface pin.
  - CLOSED 2026-08-22 (P4.2/W2 the WATCH REGISTRY unit COMPLETE, worker
    873ebd5e claim 1, commit 01a6847): the DESIGN-DIFFHARNESS §4 watch
    set committed as data — tools/diffharness/watches.toml, 73 rows
    (S0 trigger 0x5a6eb/0x425a03; T0 11 rows with EXD aliases; T1 17
    rows; all 15 §5b static one-shot rows; T2/T3/T4 exd-empty per the
    W1 ticket; 6 TI injection rows), schema per row = id/tier/exw_addr/
    exd_addr/indirect/extent/layout/exd_status/anchor_doc/anchor. The
    pointer-cell rows (object bank *(0x119584), TOT/DAT/CGR/BIN/MIN
    volumes, claim bank *(0x119564)) carry indirect = true; the 6 tagged
    gaps (difficulty, SFX gate, blink-cursor, order target, no-extract
    latch, selection cursor/squad) are explicitly exd-empty, never
    guessed. New zero-dep workspace crate tools/diffharness =
    registry parser + the anti-ghost validity test (every anchor string
    must resolve EXACTLY to a ledger row heading in its named doc —
    verified to bite on a fabricated anchor) + schema-invariant checks.
    cargo test/fmt/clippy green; MANIFEST verified. PUSHED 01a6847.
    Next P4.2 head: W3 (dump schema: versioned frame-record stream +
    FNV-1a-64 BDLD chain + raw-blob encoders, tools-side only).
 - CLOSED 2026-08-22 (P4.2/W1 the EXD IMPORT + EXW->EXD ADDRESS MAP
   unit COMPLETE, worker d06341cf claim 1, commits 350b53a + 10aea57 +
   f6e067a + 8447ba7): BEDLAM.EXD imported ONCE into BedlamWatcom
   (LeLoader LE/DOS4GW, x86:LE:32:default + openwatcomcpp, analysis
   green, manifest verified both sides). 8 bounded probe passes
   (-process BEDLAM.EXD -noanalysis; dumps ghidra-project/
   exd-probe{,2..8}.txt; scripts tools/ghidra-scripts/EXDProbe*.java).
   docs/RE-EXD-MAP.md = the W1 deliverable: MissionShell = FUN_000596ed,
   S0 dump point = 0x5a6eb (flip FUN_00010670, frame counter
   [0x1195f0]++ after — EXW order exact), PresentFlip/WaitVRetrace/
   VESA/PIT/RNG-stepper twins pinned; T0/T1 + static-after-load rows
   all dual-anchored (robot bank 0xf6d34/count 0x11958c with the
   stagger formula byte-exact, tile grid 0xfe37c, type table 0x108428
   layout-exact, TRT 0x95264, beacon family 0x119628-32, money
   0x119600, mission 0x119610, zone 0x107500, mode 0x1075d8, RNG
   0x107470/74, score 0x10da28, order table 0x91ee4, volume pointer
   cells, PAD 0xf63c); 5 divergence seeds (robot-front shift, merged
   monoliths, single mission scalar, indirect banks, /KARMA); 6 tagged
   gaps with anchor methods (difficulty, SFX gate, blink-cursor, order
   target, no-extract latch, selection cursor/squad). PUSHED 8447ba7.
   Next P4.2 head: W2 (watch registry tools/diffharness/watches.toml +
   the anti-ghost anchor-resolution test).
 - CLOSED 2026-08-22 (P4.2 the DIFFERENTIAL-HARNESS DESIGN DOC unit
   COMPLETE, worker 4d7b9a5b claim 1, commit 7bc2c9d, D77, docs-only;
   no engine change): docs/DESIGN-DIFFHARNESS.md landed - the P4.2
   architecture: O1 = BEDLAM.EXD under pinned DOSBox-X = the PRIMARY
   scripted-differential instrument (observation never patches the
   binaries; EXW stays the canon of record, divergences ->
   docs/DIVERGENCES.md, arbitrated by O2 = EXW/Wine ptrace channel;
   O3 = instrumented 8street second comparator, late); per-frame dumps
   at the MissionShell epilogue/present tail aligned by
   g_frame_count@0x46ae68; tiered watch set T0-T4 with every address
   ledger-anchored (RE-EXW-SIM sec 8/7j.x); seam-only injection
   (keystore/orders/COMMAND records 0x4dd4a0/.PAD step-on - never raw
   input); canonical-record differ in 5 modes per the 0b budget with
   4 divergence classes; scenario corpus S0-S8 + hypothesis
   dispositions (pod stagger, debris 2k start-delay,
   blink-cursor-from-spawn, ring overlap = statically moot per 7j.10 +
   confirming read, mid-flight blits = T2 render-side, out of
   state-diff scope); dumps stay runtime-only (asset-derived), git
   carries fingerprints only; gates DH-G0..G3; build order W1-W12
   (W1 = EXD import + EXW->EXD address map = the next queue head).
   DECISIONS.md D77 added. Manifest verified. PUSHED 7bc2c9d.
 - CLOSED 2026-08-22 (P4 7j.28 the PROJECTILE MID-FLIGHT DRAW
   family unit COMPLETE, worker ffec42cf claim 1, commits 9a1d205
   + 27481c2, D76, docs-only; objdump-only from ghidra-project/
   exw-text-objdump.txt — an analyzeHeadless was running): the
   400×0x36 dispatch (0x404141 + secondaries 0x404d27/0x404d08)
   fully mapped — shell/artillery/mortar/damped {0xF/0x13 base
   0x20, 0x17 base 0x28 (NOT the "3-clone split" — tick-side
   only), 0x1A/0x1F base 0x18}/rocket/homing; 0x40427a CORRECTED
   to loop-next (unlisted types NOT drawn mid-flight); banks
   NAMED + corpus-verified WEAPONS 70/SHRIKE 64/REAPER 64/
   SMOKE 4/GENERAL 153 (boot string block 0x45884e..); the
   0x4e66b8 trail-ring draw consumer CLOSED (8 puffs, WEAPONS
   0x10+(tick+i)&7, mode 0x12E); the 50×0x22 walk CLOSED (table
   0x403908: 0x65/0x67/0x68 strip sprites, 0x66 undrawn, 0x69
   the per-level BEAM column); FUN_0040798e call shape + mode
   words pinned. THE FUN_00403938 RENDER TAIL IS NOW FULLY
   DECODED. Manifest verified. PUSHED 27481c2. Next: the P4.2
   differential-harness design doc.
 - CLOSED 2026-08-22 (P4 7j.27 the DROPSHIP RING PRODUCERS
   unit COMPLETE, worker e635cb76 claim 1, commit 2aa7cb7, D75,
   docs-only; dump ghidra-project/exw-text-objdump.txt = full
   .text objdump, no Ghidra run — one was already running): the
   pod-descent family writer census COMPLETE (resets
   FUN_0040cca0 0x40cd3d pods memset 0x150 + MissionShell
   0x447a7e/0x447a8d; spawners FUN_0041faf0/FUN_0041fb4b/
   FUN_0041fa51; animator FUN_0041fbb1; third writer
   FUN_00412a98 0x412b60 = per-rescue exit-dwell reset);
   +0x14 = the DROPSHIP.BIN img-group selector (7j.19 "toggle"
   superseded — 0↔1 flicker phases 1-2, ramp 2..5 oscillating
   4↔5 departing, x −= group·4); pod phase 2 = ONE tick =
   robot RELEASE (state 6, payout 100·w@+0x94+5000); latch
   0x46aed4 boot-clear GameMain 0x41c408 + gates MP respawn
   0x40e7a1; 7j.26 "7×7 grid" CORRECTED to 7×5 (0x23 = 35 =
   one group); the 0x4c71f4 pass head-decoded = projectile
   mid-flight draw dispatch (+ 0x4cc654 sibling, states
   0x65..0x69). 4 ledger rows updated + MISSIONVIEW §5e
   corrected. Manifest verified. PUSHED 2aa7cb7. Next: the
   projectile mid-flight draw family (7j.28), then the P4.2
   harness design doc.
 - CLOSED 2026-08-22 (P4 7j.26 the MISSIONVIEW §5d DRAW TAILS
   unit COMPLETE, worker 7658328a claim 1, commits 753f0a2 +
   2d124e6 + d9bb40f, D74, docs-only; dump ghidra-project/
   exw-effectstager-asm.txt): both consumer passes decoded —
   the EFFECTS LOOP (0x4cf638, 80×0x1E) draws DEBRIS.BIN imgs
   0..23 (group u16@+0x16 ×8 + frame&7, counter++ in the draw)
   via the direct blit FUN_00401e39, sy base 0x100 + the second
   shake table 0x454518, z Q13; 7j.25 field map corrected
   (d@+0x14 = rising vz 6000..12069 with the group in its high
   word, u16@+0x1A = spawn delay = producer ECX arg,
   FUN_0041ec59(n) = RandB()/(0x8000/n−1) bounded-uniform);
   mover FUN_00419f62 (ceiling/off-map kill). The PLATFORM LOOP
   (0x4eb638, 32×0x14) = the enqueue pair SMOKER.BIN (bank
   pinned) frame 0 mode 300 + column frame+1 mode 0x12d at
   sy−0x20; tick FUN_004238af 2..16/5..16. FUN_00401e39 codec
   decoded + the .BIN container corpus-verified (u16 count
   word0 + u32 dir at bank+2+4·img, 24/24 DEBRIS + 160/160
   DANTE exact — MISSIONVIEW open item 4 resolved). BONUS: the
   DROPSHIP ring passes recorded (banks 0x4e64c0 + 0x4e6610..
   0x4e66b8, 7×7 0x40-grids, DROPSHIP.BIN; producers → 7j.27)
   + the [0x4ede24/28] backlog table re-pinned as the terrain
   RESTAMP list (FUN_00440a2d = the scroll/camera restamp
   stager). 7 new + 2 rewritten ledger rows. Next: the
   DROPSHIP ring producers (7j.27).
 - CLOSED 2026-08-21 (P4 7j.25 the WEAPON-FIRE FAMILY TAIL
   unit COMPLETE, worker 399aeff4 claim 1, commits 3bfd400 +
   1016123 + b4950a8 + 6183be5, D73, docs-only; dump
   ghidra-project/exw-destroytail-asm.txt): the FUN_0041a894
   destroy tail decoded WHOLE — TERRAIN RESTORE (footprint
   W×H×D: TOT-mirror z-words ← template bank@type+0x46, seen
   + DAT volume ← bank@type+0x4A) then the FIVE-effect loop
   (selectors 1..9 → jump table 0x41a870: 1→k14+FUN_0041a225
   +5 splashes, 2/3/4/5→k18/k17/k16/k19 single gibs at fixed
   sub-tile bearings + 4-splash loops, 6/7→k10 + the
   DEADMAN1/2 thud pair (banks 0x4edfb8/0x4edfbc, shared
   with the crush dispatcher), 8→k14×25 water-level
   demolition shower, 9→k20 + 3×3 splash ring; payload words
   = tile offsets; delays ride the chain counter + entry
   index; GER gate skips the whole tail for type 0xb);
   FUN_0041a225 = the FIRST producer of the MISSIONVIEW §5d
   effects bank 0x4cf638 (80×0x1E, allocator FUN_0041a4cc,
   jittered Q13 particles, ttl 6000+); the 160-vs-0xA8 stride
   anomaly at 0x4c69e4 CLOSED (21·idx·8 = 0xA8 — a 7j.13
   census arithmetic slip; trap callers robots()@0x40bc44 +
   critter FUN_00412f34@0x413fd7); BONUS: FUN_0041a4f8 = the
   .POS loader (2000×0x10 → the 0x46cbf4 object array) +
   the .BDG loader (the 0x4dedf2 type table) — .BDG grammar
   CLOSED (no header, ≤282 variable records, 4 on-disk
   template banks 2·W·H·D B each; census 37/37 EOF-exact,
   exactly 282 recs/file, selectors ONLY 1..9); FORMATS
   §12/§16/§19 rewritten. 4 new + 2 rewritten ledger rows.
   Next: the MISSIONVIEW §5d draw tails (7j.26).
 - CLOSED 2026-08-21 (P4 7j.24 the CRITTER DEATH-HANDLER family
   unit COMPLETE, commit 3819586, worker 0f986419 claim 1,
   D72, docs-only; dumps ghidra-project/exw-dead1..5*.txt —
   exw-dead1..3 = adopted predecessor WIP from worker
   ad591680's 7j.23 session tail, exw-dead4/5 + objdump
   spot-checks this unit): the SIX per-kind death handlers
   decoded (k1/k2/k3 instant death state 7+presence 0, k4..k7
   dying anim state 6; per-kind debris kinds 1/0xD/7+6/7/7/7,
   weapon gate {0x24,0x29,0xC} → 3 extra k7 + 8/12 effect rows
   for k4/k5-6); the BOUNTY GATE (score += 30/50/500/75/150/
   1000 when killer robot type == [0x4edb90], + score-strip
   refresh DAT_0046ccf0 := 2); SECOND DEATH DISPATCHER found =
   FUN_0040dce0 (debris crush via the physics tick
   FUN_0040de9c, attacker −1, k5/6 as-if-rocket 0x24, k4 no
   weapon, k5/6 state {5,6} absorbed); FUN_00421f4c = the
   critter-death SFX trio (0x4edf88/8c/90, twin of the impact
   trio); FUN_0041a14f/FUN_0041a494 = the 0x4cec38 effect-row
   spawner + always-evict LRU allocator (w@+0 = AGE word,
   correcting the 7j.23 gloss); FUN_00418a9f = NOP stub;
   7j.17 splash expectation CORRECTED (the death handlers
   never call FUN_00424355 — splash producers are the
   controller landing/suicide paths); FUN_0040e230 SP tail
   CONFIRMED + MP respawn completed (suicide gate, MRK
   reposition, weapon/equipment re-copy); FUN_0042382c = the
   FIRST producer of the 0x4eb638 "platform loop" bank (robot
   death blast records, claim-byte gated). 8 new + 2 rewritten
   ledger rows. Next: the weapon-fire family TAIL (7j.25).
 - CLOSED 2026-08-21 (P4 7j.23 the ACTOR HIT APPLIERS unit
   COMPLETE, commit 45329e9, worker ad591680 claim 1, D71,
   docs-only; 4 × -process runs, dumps ghidra-project/
   exw-hitters{,2,3,4}*.txt + exw-hitters-scan.txt; new
   StoreScan.java operand scanner for computed refs):
   FUN_004190bc = the CRITTER hit applier (kind switch w@+0
   = the .NME section states, mode 2 = octile+z-box per-kind
   thresholds, damage FUN_00419aff per-WEAPON, attacker/
   flash/impact stores, 6 per-kind death handlers, 25%
   knockback FUN_0041a028 = 2nd 0x4cec38 spawner, k7
   in-record knock); FUN_00418fca = robot box-test applier →
   FUN_0040e230 [head: shield/hp/alarm/tier-SFX/MP-frags];
   TRAIL ALLOCATOR CLOSED (FUN_00412a4a 20 slots, writer
   FUN_0040a9ff mortar spawner, link/active/ring-zero
   protocol); third critter-applier caller found
   (FUN_00403938 weapon 0xC 5000-blast). 9 ledger rows
   touched. Next: the critter death-handler family (7j.24).
 - CLOSED 2026-08-21 (P4 7j.22 the WEAPON-ANIM MACHINE head
   unit COMPLETE, commit 29adbf1, worker 27e4f048 claim 1,
   D70, docs-only; 3 × -process runs, dumps ghidra-project/
   exw-weaponanim{,2,3}*.txt): FUN_00410823 (6102 B) = the
   WEAPON-ANIM/PROJECTILE TICK over the whole 400×0x36 bank
   0x4c71f4 — 4 calls/frame (phase 0..3; artillery phase-0
   only, actor hit-tests odd phases only); record layout
   CLOSED (target sel d@+6, class d@+0x2A = launch delay OR
   detonation cycles, arc d@+0x2E = ballistic z-vel/heading,
   trail link d@+0x32); per-type machines: bullets 2..4
   (2-substep lookahead ray), shell 5 (K3 trail), artillery
   9..0xB (scripted bursts: durations 2/4/7 frames over 7
   expanding-ring lists @0x45687c via PTR[0x456bf0], 500-
   sentinel, spotter reveal at ttl 24), the ballistic bounce
   family {0xE,0xF,0x13,0x17,0x1A,0x1F} (0xE mortar = bounce
   + 3×5000-blast per contact + the 0x4e66b8 smoke-trail ring
   bank; 0x17 = 3-clone split; 0xF/0x13 = ttl-cycle
   submunitions → the four-quadrant 0x1A detonation, 7j.13
   sites re-anchored), rocket 0x24, homing 0x29 (target lock
   + heading-search terrain avoidance). Actor hit-test front
   doors pinned: FUN_0041879d = critter lane (→
   FUN_004190bc mode 2), FUN_0041874c = MP other-robot lane
   (→ FUN_00418fca mode 2); the 7j.15 "FUN_004190bc =
   panel/preview" hypothesis CORRECTED (critter hit applier).
   RandA = FUN_00402975 re-pinned. 4 ledger rows. Next: the
   actor hit-applier internals (FUN_004190bc + FUN_00418fca,
   7j.23).
 - CLOSED 2026-08-21 (P4 7j.21 the 0x425xxx ARRIVAL-PRODUCER
   family unit COMPLETE, commit 923668e, worker b67abe61
   claim 1, D69, docs-only; 4 × -process runs, dumps
   ghidra-project/exw-arrival{1,2,3}*.txt): FUN_00425da4 =
   the ELEVATOR-RIDE STAGER (MissionShell boot @0x447b4e,
   zone/mode/mission switch, fixed-address stores, markers
   from .PAD slot words, countdown NEVER producer-written —
   records stage dormant); the runtime armer = the
   FUN_00433980 ride cases (countdown:=10, rider state 2,
   pre-position at the marker) — the 45-record 0x4dcdb8
   array is the elevator/teleport RIDE PIPELINE, closed
   boot→arm→tick(SFX+burn+teleport)→draw (sprite 0x12E,
   width clamp(11−c,0,9)). 7j.11 corrected (record layout
   marker x/y/z; walk stops at first inactive). Rect-list
   boundary: the 0x4dcae8 0x2d0 clear ends EXACTLY at
   0x4dcdb8 (no overlap; 7j.12 same-family hypothesis
   refuted; FUN_004223b8 = door open/close stepper
   re-anchored). 0x4c71c4 anchor refresh negative. Next: the
   weapon-fire head FUN_00410823 (7j.22).
 - CLOSED 2026-08-21 (P4 7j.20 the extraction BEACON +
    POD-COUNTDOWN producers unit COMPLETE, commit c37b8ef,
    worker c7269abe claim 1, D68, docs-only; 2 × -process
    BEDLAM.EXW -noanalysis runs, dumps ghidra-project/
    exw-beacon{,2}*.txt + full-objdump census of all
    0x4c6a10-displacement sites): FUN_004247b5 = the
    EXTRACTION-BEACON ARMER — sole caller FUN_00433980
    @0x433cfb (the zone pad-trigger dispatcher), REVOKING the
    old "robot-sprite click family ~0x433cbc" attribution
    (0x433cbc lies inside FUN_00433980's 3185-B body): ~25
    (zone, .PAD slot) pairs are extraction pads; armer body =
    guard 0x4eabb0 → countdown 0x197 (0 if the player-0
    alive-count == 1 = last robot), tile trio 0x4eabb4/6/8
    (z = dead store), robot state := 3 + spread-teleport +
    SFX 0x2A. FUN_004248c8 = the SPREAD-CLAIM picker (12×u16
    0x4eabba, one-shot claims; offsets center + 8 neighbors +
    (−2,0)/(0,−2)/(+2,0); ≥12 → caller stores UNINITIALIZED
    locals). w@robot+0x2C = the DROP-POD descent timer: SP
    producer EXISTS (FUN_0040cca0 spawn tail @0x40d132
    stagger 1+k·(2000−m·1000/27), m = linear mission) —
    refutes the "no SP producer known, always 0" gloss; MP
    respawn 0x28 @0x40e89d; reader FUN_0040b9f6 freezes the
    whole robot brain while ≠0 → 0-hit fires the pod anim
    (the 0x4e64c0 pod bank = deploy + respawn + extraction).
    §6.4/§6.5/§7b.6/§7c.8 corrected; +0x2C record row
    rewritten; 4 ledger rows + the 0x4c71c4 per-player
    selected-anchor census. The extraction trigger chain is
    CLOSED end-to-end (pad script → armer → rally → dropship
    → sweep). Manifest verified. PUSHED c37b8ef. Next: the
    0x425xxx arrival-producer family.
 - CLOSED 2026-08-21 (P4 7j.19 the EXIT/ESCAPE RUNTIME unit
   COMPLETE, commit c64c637, worker 90c04773 claim 1, D67
   docs-only; 3 × -process BEDLAM.EXW -noanalysis runs, dumps
   ghidra-project/exw-exitfamily{,2,3}*.txt): FUN_0041fbb1 =
   the ESCAPE-CRAFT ANIMATOR (MissionShell @0x448012): 3
   machines over one 0x1C frame {active, PHASE, x, y,
   altitude, toggle, dwell} — the 5 exit elevators @0x4e662c,
   the extraction DROPSHIP @0x4e6610 (landing = extraction
   sweep of robot states 3/4 → 5, _DAT_004dc680++, departure
   → _DAT_004dc67c = 1 complete flag read by MissionShell +
   FUN_0044425c), the per-robot ESCAPE PODS @0x4e64c0
   (landing = payout 100·w@+0x94+5000; gated by the
   [0x46aed4+idx·4] no-extract latch). The 7j.17 "+4 kind" is
   a PHASE (1 descend / 2 landed-OPEN / 3 depart) — the POI
   flee gate kind==2 = LANDED elevators only.
   FUN_00433980 = the ZONE PAD-TRIGGER SCRIPT DISPATCHER
   (caller FUN_0040b9f6 @0x40bd58 when state∈{1,4} ∧ order
   word ≠ −1): FUN_00422e5e = the PAD-TILE PROBE (DAT byte
   0xFF → 999×8B .PAD slot scan @0x4e44f8); per-zone switch
   on 0x4edd8c = elevator rides (scripted dests
   0x4dcdbc..0x4dd330), messages FUN_00424a6f, doors
   FUN_004223b8 over the 45×0x10 rects @0x4dcae8, case 0x1B =
   the SOLE exit-pad activation FUN_0041fa51 — the
   personnel-rescue loop is CLOSED end-to-end (.PAD load →
   00433980 script → 0041fa51 activator → 0041fbb1 lands →
   00412a98 POI flee → [0x4eba0c]++ → 00448b80(5000)).
   FUN_0041faf0 = dropship deployer (beacon 0x4eabb4/76);
   FUN_0041fb4b = pod spawner (countdown w@0x4c6a10).
   [0x4eba0c]/[0x4eba10] consumer censuses CLOSED; 4 ledger
   rows added/updated; open item 0a rewritten. Manifest
   verified. PUSHED c64c637. Next: the beacon/pod-countdown
   producers (FUN_004247b5 + FUN_004248c8 + 0x4c6a10 writers).
 - CLOSED 2026-08-21 (P4 7j.18 the critter/POI/exit LOADER hop
   unit COMPLETE, commits 7f1c8fb docs + f04681d tooling,
   worker a840f0af claim 1, D66): FUN_00416458 = the .NME
   loader — stages ".NME" (@0x457a57, bytes verified) and
   reads EIGHT fixed-order count+records sections (widths
   10/10/8/8/10/8/6/8; 16 FUN_0041cccb call sites
   census-verified): sections 1-7 spawn critter states
   2/1/5/4/3/6/7 (spawn multipliers by difficulty 0x46cbf8;
   hp = base+(base·d)/27, bases 0xAF/0xC8/0x96/0x5DC/0x9C4;
   species word +0x02 ∈ 1/3/6; octile dists @+0x60 via
   0x4543e4/0x454404; S2 does a DAT z=6-down floor search;
   S5 stores home; S7 z fixed 0xDF); section 8 feeds the POI
   bank (4 POIs per record, jitter ±31 sub-tiles, spawn
   state 5 ESCAPE — personnel flee from load). Corpus-exact
   on all 37 files; ZONEA/MISSION1.NME keeps a 16-B orphan
   tail no game code reads (FUN_004180b9 = empty stub).
   FORMATS-MISSION §9 REWRITTEN — the NME grammar is CLOSED
   (the old header/section model was a mis-split of the
   fixed schedule). FUN_0041fa51 = the EXIT-PAD ACTIVATOR
   (the 5×0x1C exit-slot producer: arg = a 0x4e44f8 .PAD
   slot index, dedup registry 5×d @0x46cd20, stamps {1, 1,
   pad.x·0x20+0xF, pad.y·0x20+0xF, 0x400, 0}; caller
   FUN_00433980 @0x43900e = the pad trigger handler [open]).
   7j.17 leftovers folded: FUN_00449c94 = the LOCAL
   COMMAND-RECORD BUILDER (0x4dd4a0 stride-0x80, cmd codes
   1-4 + payload words, MP broadcast loop + NETWORK ERROR
   paths — the local-input side of the command ring CLOSED),
   FUN_0040db9e = the critter ranged-attack APPLIER on
   robots (0x476fe4 0xC-stride weapon-param table, param_5
   −1 → the critter entry @0x476fd8; robot stun word 0xFFFF
   @0x4c69e4+idx·0xA8 + FUN_0040c536 timed effect scaled by
   octile dist·mult), [0x4eb8b8+slot·4] census = objective-
   done flags (MissionShell + FUN_0044425c + FUN_00448b80
   only). ENGINE/TOOLING: parse_nme replaced by the exact
   8-section schedule + a corpus exact-consumption test
   (37/37); fmt+clippy clean, workspace green, manifest
   verified before AND after. PUSHED 5deb649. Queued: the
   exit/escape runtime family (FUN_0041fbb1 + FUN_00433980).
 - CLOSED 2026-08-21 (P4 7j.17 the ROBOT TARGETING/AIM family
   ADOPT unit COMPLETE, commit eaf16c0, worker 3f4f7c10 claim
   1, D65, docs-only): adopted the three provider-outage-
   killed runs (19:15/19:34/19:40; logs agent-31790e94/
   08f6fa30/0ce3a285) — re-verified their on-disk Ghidra
   dumps (ghidra-project/exw-robottarget*.txt/-xrefs/-asm)
   and landed RE-EXW-SIM amendment 7j.17 + ledger rows +
   open items, NO new Ghidra run: FUN_00412f34 = the 0x4cff98
   CRITTER-ACTOR controller (stride 0x7E, count
   DAT_0046cc2c<-FUN_00416458@0x41646d, sole caller
   MissionShell@0x447fe1; states 1 wander/2 sine-walk
   shooter (0x65, range (2−d)·−0x40+300)/3 chase (0x67 full
   3D velocity, pathfinder FUN_0041571c, home leash 400)/
   4-5-6 mixed-AI (mode 0xB dormant, respawn-delay table
   DAT_00454edc[d]; mode 6 ballistic landing → 8× k6 debris
   + FUN_00424355 + splash FUN_0041a14f(0x18); mode 9 seek-
   steppers; mode 2 range FUN_0040db9e)/7 close-combat
   (point-blank 0x69, fire rate 32/16/8 by d, break odds
   1/8·1/16·never, leash (d+1)·0x40+600); presence byte mark
   [[0x4ea900+(y>>13)·4]+[0x46af4c]+(x>>13)]:=1, SAR 0xD
   asm-verified; Q13 x@+0x36/y@+0x3A/z@+0x3E confirmed).
   DIFFICULTY dial amended: 12 objdump sites — drives
   critter behavior, not only damage. FUN_00417e2f =
   SUICIDE-BOMB trigger (<0x30 px → k1 debris ×8).
   FUN_00412a98 = the 0x4dabdc POI/PERSONNEL controller
   (stride 0x1E, count DAT_0046cbf0@0x416f6e; flee-to-exit
   over 5×0x1C exit slots 0x4e662c via FUN_00417c64;
   escape → [0x4eba0c]++, [0x4eba10]=0x32,
   FUN_00448b80(5000); producer FUN_0041fa51 open).
   FUN_00409138 = the COMMAND-RECORD consumer (0x4dd4a0
   stride 0x80 count DAT_0046cbe0; builder FUN_00449c94 +
   MP lobby/SHOP family; 39-case weapon switch: order
   dispatchers FUN_0040b615/0xaf98/0xa56f/0xace8/0xa7a1/
   0xa9ff + projectile spawners into the 400×0x36 bank
   0x4c71f4 aimed at the ORDER TARGET 0x4dd484/88/8C;
   auto-rearm + msgs 0x1C..0x21). FUN_00448b80 = the
   MISSION-OBJECTIVE RESOLVER (6×0x20 slots 0x4eaaee,
   type 5000 rescue vs kill-stats [0x46cbf4]+type·0x14 +
   mirror wipe 0x4796d7/d8; msgs 0x26/0x27/0x34, all-done
   0x28+0x29 → DAT_0046cd00 phase state; zone-7 counter
   [0x46cce0]). FUN_0041e411 = floor probe (the
   [0x4edd60]=.CGR height-bank semantics — per-type entries
   + in-tile 0x20×0x20 byte maps). Residual 0x4dd484
   reader census CLOSED (folded into ledger). ENGINE: none
   (D65 — families stay unwired). Manifest verified.
   PUSHED eaf16c0. Queued: the critter/POI/exit LOADER
   section inside FUN_00416458 (which mission file feeds
   0x4cff98/0x4dabdc/0x4e662c — .NME/.POS candidate).
 - CLOSED 2026-08-21 (P4 7j.16 the .TRT CONSUMER hop unit
   COMPLETE, commit f7262ea, worker 16f43187 claim 1, D64,
   docs-only): RE-EXW-SIM amendment 7j.16 pins the three
   0x4cccf8 scanners — FUN_00417264 (MissionShell tick
   0x44807b) = the TRT ANIMATION/FIRE machine (rec frame
   active@0x4cccf8: {active@+0, state@+4, anim_frame@+8,
   fire_ctr@+0xC, hp@+0x10, x@+0x14, y@+0x18, z@+0x1C}; states
   idle→alert→aim S/N/W/E→fire/death; the "+0x08 scratch"
   producer CLOSED = this machine); FUN_00417698 = FIRE
   (0x28px lane, ≤2 levels → projectile type 0x66, damage
   (d+1)·300, free-slot FUN_0041286f) — TURRETS RESTORED,
   structures animate+shoot, never move; FORMATS §14
   re-anchored. FUN_00419943 = the map-click pick (ret
   (idx+1)|0x2000 = structure), FUN_00410644 = the click
   ORDER dispatcher (order target 0x4dd484/88/8C),
   FUN_0041ec81/FUN_0041ee20 = the SCANNER widget overlay,
   FUN_00417c00 = nearest-robot octile probe, FUN_0041ebf8 =
   octile distance (51 sites). The two 3D banks = the map
   FILE VOLUMES (FUN_0041dc5a: .TOT→[0x4ede20] with u16
   W,H header + 8 word planes, corpus-verified; .DAT→
   [0x4edd58] u8 planes ≥0x80 sanitize; + .CGR/.BIN/.MIN/
   .LNG-.LNK/.PAD 999 slots 0x4e44f8 stamping 0xFF);
   FUN_00440a2d = the TOT-volume→mirror MATERIALIZER (the
   TRT word-1→sprite bridge); FUN_0044661b = the EDITOR\ZONE
   restore reload. The uncommitted 22c1c14b erratum draft
   landed CORRECTED (W/H/D stay @+2/+4/+6; its 5×8B entries/
   count/banks/0x4E closure confirmed). ENGINE: none (D64 —
   corpus verdict unchanged, turret fire stays unwired).
   Manifest verified. PUSHED f7262ea. Queued: the robot
   targeting/aim family (FUN_00412f34/FUN_00417e2f/
   FUN_00412a98 + the 0x4dd484 order consumer FUN_00409138).
 - CLOSED 2026-08-21 (P4 7j.15 weapon-fire family THIRD HOP
   unit COMPLETE, commit 52b1ebd + state c8ded44/b50f449,
   worker efff097c claim 1, D63, docs-only): RE-EXW-SIM
   amendment 7j.15 pins FUN_00419aff = the WEAPON/PROJECTILE
   DAMAGE TABLE — a pure id→damage switch, NO table walk
   (2/3/4→20/30/40, 5→75, 0xc→5000, 0xd→312, 0x1a→75, 0x24→400,
   0x29→250; projectiles 0x65→(d+1)·50, 0x66→(d+1)·300,
   0x67/0x68→(d+1)·75 with d=2 flat overrides 200/1200/300; else
   1). ERRATUM 7j.13: no field arg (EDX passes through; the
   fire sites' push 1 only arms the score flag). DAT_0046cbf8 =
   the DIFFICULTY dword 0..2 (cycled (d+1)%3 at NameEntryScreen,
   save-persisted, 500·d money delta, zone-7 temporarily forces
   2). Caller census 28 = FUN_00410823×16 + FUN_004190bc×6 +
   FUN_00412010×4 + FUN_004197d4 + FUN_00418fca. The 0x4cccf8
   PRODUCER = FUN_004170a6 = the ".TRT" mission-section loader
   (sole caller FUN_00416458): 250-rec capacity, rec {+0=1,
   +4 active, +8 scratch 0, +0xC hp=250+(250·mission)/27,
   +0x10 x, +0x14 y, +0x18 z} at stager base 0x4cccfc (7j.14
   resolver frame is +4); stamps tile 0x66 + word 1 into two
   NEW 3D banks ([0x4edd58]/[0x4ede20], consumers open).
   FORMATS-MISSION §14 anchored: TRT third u32 = z LEVEL;
   "turrets?" retired. ENGINE: none (D63 — corpus verdict
   unchanged). Pins untouched; manifest verified. PUSHED
   27f5def..b50f449 — the 7j.13/7j.14 push debt is CLEARED
   (secret service recovered after a machine restart). Queued:
   the family FOURTH HOP (the .TRT consumer trio +
   FUN_004190bc).
 - CLOSED 2026-08-21 (P4 7j.14 weapon-fire family SECOND HOP
   unit COMPLETE, commit 7b9ce05 + state, worker d37fb3a2
   claim 1, D62, docs-only): RE-EXW-SIM amendment 7j.14 pins the
   sibling resolver — FUN_0041bc1c(x/y Q13, damage ebx) = the
   TERRAIN-STRUCTURE damage resolver over the NEW array
   0x4cccf8 stride 0x20 count [0x46ccd4] {active@+0, hp@+0x10,
   x@+0x14, y@+0x18, z@+0x1C}, externally 1-based
   (dword[0x4cccd8+id·0x20], id-0 guard at 0x4cccd8); survivors
   take hp−=damage only; destroy → zone floor word
   [0x454a04+4·zone] into the TOT mirror 0x4796bc+30·tile+2z +
   seen @0x4796cc + DAT volume 0 + debris K0xF + splash — NO
   robot-armor branch (7j.13's terrain/robot question closes
   TERRAIN-only; 10 call sites census'd with arg windows).
   FUN_0041eaa1 = the per-pixel terrain-height probe (DAT volume
   byte → the 32×32 height banks behind [0x4edd60], entry
   (h−1)·4+2 +6 header; hit iff z ≤ (z>>5)·0x20 + byte).
   FUN_004124a4 = the weapon-anim debris disburser (rec
   0x4c71f4+0x36·i, kind word@+0 → K2/K3/K6/K9/K0xC map, z−10);
   FUN_004126dc = the projectile disburser (rec 0x4cc654+0x22·i,
   +0 = TYPE word 0=free: 1→K2, 0x65→K0x14, 0x66→K8, 0x67/0x68→
   K4; FUN_004197d4 = the robot-hit expiry walker |dx|<0x10 Q8,
   |dz|<0x20; projectile type ids = weapon-stat ids). Splash
   gates + max-age eviction pinned (claim byte 0x46af58 third
   reader). ENGINE: none (D62 — corpus verdict unchanged, all
   fire/impact sites stay unwired). Pins untouched; manifest
   verified. Push retried twice, STILL blocked (secret service
   dead — commits 4448a77, 2064e18, 7b9ce05 safe locally,
   retry by next run/operator). Queued: the family THIRD HOP
   (FUN_00419aff stat table + the 0x4cccf8 producer census).
 - CLOSED 2026-08-21 (P4 7j.13 FUN_0041a894 weapon-impact ray
   head FIRST HOP unit COMPLETE, commit 4448a77 + state, worker
   b7f866b6 claim 1, D61, docs-only): RE-EXW-SIM amendment 7j.13
   pins the resolver — FUN_0041a894(x Q13, y Q13, chain ctr ecx,
   damage ebx, [stack] score flag) is the PER-TILE WEAPON-IMPACT
   OBJECT RESOLVER, NOT a walk: grid-word dispatch (0/0x7d2/
   0x7d3 pass-through ret 0; 0x7d4 → FUN_00422693 platform
   damage; n>0 → rec n−1 hp −= damage; ret 1 only on destroy).
   The RAY lives in the callers (17-site census): the projectile
   tick FUN_00412010 (50 rec @0x4cc654 stride 0x22, ballistic
   x/y/z += v, terrain probe FUN_0041eaa1, damage =
   FUN_00419aff(0x65/0x66)), the robot fire controller
   FUN_00410823 (8 sites: weapons 5, 0x1a ×4 quadrant blast,
   0x24, 0x29; damage FUN_00419aff(id,1)), the tile-0x62 trap
   pair FUN_0040fe93/FUN_0040ff92 (damage 100 → 5× k12 debris),
   the script blast FUN_004244a1 (damage 5000, score armed), and
   4 chain-detonation self-calls (perimeter walks, damage 1000,
   id-table chain word@+0xC gate). The 7j.12 "object-stamp loop
   0x41a84f" is FUN_0041a7f0 (footprint stamper, word = rec
   idx+1 over W×H) invoked from the mission-load restamp pass
   FUN_0041a4f8@0x447b76, which parses the OBJECT TYPE TABLE
   (0x4dedf2, 0x4E stride, 282 recs from the mission file: W/H/D
   @+2/+4/+6, hp@+8, chain@+0xC, type@+0xE — 0xb scores 10,
   jitter words@+0x16..+0x1C, 4 scratch banks@+0x30..+0x3C).
   ENGINE: none (D61 — weapons never fire in the gates; resolver/
   tick/controller/table stay unwired). Pins untouched; manifest
   verified. Push attempted; origin push blocked by a dead
   secret service at close-out (commits safe locally, retry by
   next run/operator). Queued: the weapon-fire family SECOND
   HOP (FUN_0041bc1c).
 - CLOSED 2026-08-21 (P4 7j.12 FUN_00422693 platform/destructible
   family decode unit COMPLETE, commits f759b3a + state, worker
   5aa2d164 claim 1, D60, docs-only): RE-EXW-SIM amendment 7j.12
   pins the gate banks — 0x460dfa = the tile OBJECT-WORD GRID
   (0/0x7d2/0x7d3/0x7d4/object-id n → rec n−1 @0x46cbf4
   {x,y,z,id,flags,hp}), 0x465daa = the PLATFORM STRENGTH word
   (the §7c "TOT mirror" gloss superseded). FUN_00422693 = the
   damage entry (weaken/scorch+4/conditional ring spread, or
   destroy: water z-word cleared via FUN_0042394a@0x422750 +
   both banks + 5 kind-7 debris@0x4227b9); FUN_00422832/8ce =
   the spread ring (0x7d4+strength+water z-word create
   @0x422a54); FUN_00422a9c = the 1/32 creep tick (strength 199,
   site latch 0x4dc5c8/cc). PRODUCERS CLOSED: 0x7d2/0x7d3
   (FUN_00422f18, load 0x447b8f, per-zone ranges 0x454a20/
   0x454a3c — §7g.5), type-DB +0x19/+0x1a (FUN_00422fd1, load
   0x447ba3, 45×0x10 rect list @0x4dcae8 — MISSIONVIEW §8.1),
   scorch increment (FUN_0042223c, +v clamp 7). FUN_00422cc2 =
   the 32-timer delayed-trigger tick → floor-word write via
   FUN_0041bd54 (fast z-writer; second 0x454a90 context — 7h.3
   pickup producer still open). ENGINE: none (D60 — all callers
   off the corpus path; banks/timers stay unwired). Pins
   untouched (no code change); manifest verified. Pushed.
   Queued: the weapon-fire family first hop.
 - CLOSED 2026-08-21 (P4 7j.10 FUN_00424051 decode unit COMPLETE,
   commits 782a25b + 54c4109 + d08b51f, worker 89d34b53 claim 1,
   D58): RE-EXW-SIM amendment 7j.10 IDENTIFIES the 7j.9 item-5
   producer — FUN_00424051 is the per-frame mission-epilogue tick
   (0x447ff0, right after the debris tick): (1) the GLOBAL +0x18
   FADE — every nonzero armor-pad/scorch byte decays 1/frame
   unconditionally, so the D57 ring is TRANSIENT (a value-4 center
   arms pads for exactly four phase-1 passes) and permanent map
   pads CANNOT exist (MISSIONVIEW 8.1 +0x18 question FULLY
   closed); (2) the WATER-SPLASH EVENT TICK — 250 records @0x4e9778
   {x,y,z,delay,age}: weapon impacts (11 stager callers, the
   FUN_0041a894 family, one co-staging debris) stamp the zone
   water sprite at the first free z (FUN_0041bd78), fall through
   empty levels on odd frames (g_frame_count&1), absorb into
   water below, re-stamp base+0x16 @age 40, dry up @age≥47,
   scorching the tile every tick. FUN_0042394a = the z-structure
   writer (TOT z-word + seen + DAT volume — the map-edit
   primitive); FUN_0041eb28 = the DAT volume read (NOT
   visibility). ENGINE: the fade landed at the advance_frame tail
   (corpus-safe: no armor_pads corpus producer, set_armor_pads
   test-only); the two permanent-pad tests now stage value 7; +1
   unit test (decay + single-charge value-1 + full ring fade);
   the splash system stays UNWIRED (no corpus producer —
   documented, re-open with the weapon family). Gates: pins
   UNMOVED, 41 suites green, fmt/clippy clean, smoke two-run
   byte-identical AT the baselines (scene 696adb1cd110e062,
   parity cce30c983b97b16d, audio 110400/158092), MANIFEST
   verified. Pushed. Queued: the FUN_00420608 remaining-kind
   census.
 - CLOSED 2026-08-21 (P4 7j.8 scorch re-verify unit COMPLETE, commits
   d436a58 + 982e0fa, worker 11384359 claim 1, D57): RE-EXW-SIM
   amendment 7j.9 resolves the 7j.8 caveat byte-precisely — the
   robots() phase-1 armor reader (0x40bc57..0x40bc9f) tests the RAW
   type-DB +0x18 byte != 0, NO mask; FUN_00422287 (whole re-verified)
   writes that SAME byte (0x4796d4+tile*0x1E, sar>>5 world->tile, map
   bounds, zero-extended value >= 8 -> 7) — scorch and armor pads
   SHARE the byte. The kind-5 ring CORRECTED from "six" to NINE 3x3
   tile writes (corners 1 / edges 2 / center 4, exact order
   0x421476..0x421291 incl. the shared tail; a death = 45 writes,
   overlaps last-write-wins). Full caller census: SEVEN in-family
   ring producers (kinds 3/4/5/6+12/9/11/20, identical rings; jump
   table 0x4205b8 re-verified) + ONE external FUN_00424051 (five
   same-tile re-rolls, values 3..6 then 1..4, census-only/unwired).
   ENGINE: MissionSim::scorch_write (FUN_00422287 model over the
   armor_pads mirror, zero-padded growth, public host seam) + the
   apply_damage death-tail nine ring writes per debris + pub
   armor_pad_byte + DEBRIS_SCORCH_RING + 2 unit tests (the ring-fold
   pattern/offsets/overlap + the survivor-charges-on-scorch raw
   reader semantics; the writer bounds/clamp rules). Gates: EVERY
   pin UNMOVED — corpus + scene gates green, smoke two-run
   byte-identical AT the recorded baselines (scene 696adb1cd110e062,
   parity cce30c983b97b16d, audio 110400/158092), fmt/clippy clean,
   MANIFEST verified before and after. Pushed. Queued: the
   FUN_00424051 scorch-family decode.
 - CLOSED 2026-08-21 (P4 dead/hit dither unit COMPLETE, commits
   4f702e1 + 31a4691, worker efc8b1e0 claim 1, D55): RE-EXW-SIM
   amendment 7i decodes the FUN_00401ae6 static blit whole (mode 0
   rep-movsb replace vs mode 1 nonzero-only overlay; dest = fb +
   y*pitch + x; per-row RESEED RandB&0x1ff when src+96 >= 0x800;
   seed FUN_0041ec59(0x7f6,0x30) = (RandB()&0x7fff)/15 clamp
   0x7f5) and REFUTES the "512-B mask bank" gloss: 0x4e6ed8 is a
   2048-B .bss NOISE RING (cursor 0x4ddb30), binary {0,0xFF} at
   25% white - boot fill 2048 RandB draws in the MissionShell
   staging (0x447b13) + a 15-byte/frame churn in the frame
   epilogue (0x448147, unconditional incl. overlay frames); the
   portrait pass confirmed: in-squad dead/hp<1 -> mode 0, alive +
   hit_flash != 0 -> portrait then mode 1, beyond-squad slots ->
   mode 0 EVERY frame. ENGINE: the Dither ring + blit wired in
   draw_sidebar_portraits over the real sim hit_flash (the pass
   never decays it - 7g.8 stays the sim tick), edge_rng renamed
   rand_b as the ONE shared RandB stand-in consumed in the EXW
   order (terrain edges -> dither -> churn), the sidebar block
   moved after the terrain pass in present() (disjoint plane
   halves, pixels identical). Gates: frame pins RE-PINNED ONCE
   (spawn 7fdada56b10f1cad, walk 58ea10373e8d4284, overlay
   1d70e0bd059f5ae0, armed 6050d20755b2d852 - ZONEA spawns a
   1-robot squad so slots 1/2 carry static; reason recorded in
   the gate header), sim pins byte-identical, the overlay gate's
   stale-sidebar reference re-anchored to the last-presented
   frame (per-blit seed draws make normal sidebars differ per
   frame, exactly like the EXW), 41 suites/470 tests green (+1
   dither unit test), fmt/clippy clean, smoke two-run
   byte-identical AND at the recorded baselines (scene
   696adb1cd110e062, parity cce30c983b97b16d - the smoke hashes
   are end-of-journey cutscene state), MANIFEST verified.
   Pushed. Queued: the 0x4dc5d0 effect-row producer family +
   FUN_00420608 debris stager.
 - CLOSED 2026-08-21 (P4 pickup consumer unit COMPLETE, commits
   e10fdb5 + d8e03a7 + 5a3a419 + 81fd558, worker 66831068 claim 1,
   D54): RE-EXW-SIM amendment 7h decodes the FUN_0040eba0 pickup
   family - the tile-word dispatch (DGROUP range tables
   0x454a58/0x454a74 per the _DAT_004edd8c terrain set; A values
   CORRECTED to [0x4e,0x75,0x75,0x358,0x75,0xa3,0xa3] by a
   byte-precise re-dump after the first read was off one dword;
   closed 4-word groups -> A cases 1/3/2/4, B cases 9/7/8; the
   9-entry jump table), the case bodies 1/2/3/7 (drop +0x80=1000,
   shield +0x88=1000, hp +0x78 +=0x9C4 clamp 0x1388, shield_boost
   +0xA0=200; SFX 0x43a48e head + the 0x4dc5d0 16-B effect-row
   tail with ids 1/6/7/0xE), the robots() caller consume block
   (probe-latch mirror-word read, DAT z-plane zero, the 0x454a90
   floor-word swap), and the _DAT_004edd8c producers (GameMain
   boot 1; the mission-number->set family 0x43edb0+). ENGINE:
   pickup_case(word, set) pure decode + PICKUP_RANGE_A/B consts
   (bedlam-core), MissionSim::apply_pickup(idx, case) writing
   the hash-covered D53 fields, PickupOutcome exposing the
   effect id, the thin MissionScene::pickup host seam (game);
   case 4 kept as the D52 pickup_score_money producer. The
   tile-word producer stays host-seamed (the 0x4796bc mirror is
   not modeled - queued). Gates: workspace tests green (+4),
   fmt/clippy clean, smoke two-run byte-identical AND equal to
   the recorded baselines (scene 696adb1cd110e062, parity
   cce30c983b97b16d - pins UNMOVED, the seam is off the corpus
   path), MANIFEST verified. Pushed. Queued: the dead/hit dither
   overlay unit (FUN_00401ae6 + the 0x4e6ed8 mask bank).
- CLOSED 2026-08-21 (P4 damage unit COMPLETE, commit d9032d9,
  worker 416ca029 claim 1, D53 — unit finished across an
  interrupted predecessor run that committed the 7g pre-decode
  5e10768 + the implementation WIP; this run validated the WIP
  line-by-line against the exw-missionrender decompile and landed
  it): RE-EXW-SIM amendment 7g + ENGINE: the Robot damage fields
  (hp +0x78, armor +0x30, hit_flash +0x2E, alarm +0x34, alarm_ctr
  +0xA4, shield +0x88, shield_charges +0x8C, shield_boost +0xA0,
  battery +0x94, armor_pool +0x98, kind +0x2A, death_flag +0x9C)
  are hash-covered sim state; spawn hp = the dropship-landing
  5000+100*battery (set_battery seam); MissionSim::apply_damage =
  the FUN_0040e230 SP core (state-2/alive gates, the ordered
  state-3 -> shield 0x20 conversion, the auto-shield idle, the
  alarm trip at ctr > 100 on the player type, shield absorb vs
  hit_flash-then-hp subtract, the SP death subset with five debris
  staged from the SHARED stream — 10 RandA draws, DamageOutcome
  carries the presentation half); the phase-0 pre-walk
  (alarm/ctr decay, shield -2 clamp, the booster 10000/150
  family); the phase-1 armor pass (pad byte -> FUN_004100b7 +20
  behind the +0x98 pool else -10 bleed, clamp 3000/0;
  set_armor_pads seam — the producer is MISSIONVIEW §8.1-open,
  all-zero on the shipped corpus); the portrait-pass hit_flash
  clamp-5 decay. Game side: the D52 Sidebar vitals staging DROPPED
  (bars/portraits read the sim fields; set_weapon_loadout lands
  battery through sim.set_battery; the death hosts the
  DAT_0046ccec = 3 redraw countdown). Not modeled: +0x32 decay,
  the 0x7d2/0x7d3 tile words, the 7 order-word death clears, MP
  respawn, SFX — and the damage PRODUCERS stay host-seamed.
  Gates: sim pins RE-PINNED ONCE for this reason (post-spawn
  1cc7b8e125165988, post-arm 5b9c2fd5d85f9adc, arrival
  d8eeb3e608af0be4, click 0bf4fb534d6b3bd5, overlay
  78a16ba63607d197 — spawn hp 5000 is the only nonzero new hash
  input); frame pins byte-identical (9ecd7691d388bbfa /
  333d128dc812d547 / 1504c600819e724c / 86a788ff93bd78a5); 41
  suites / 465 tests green (8 new), fmt/clippy clean, smoke
  two-run byte-identical AND equal to the recorded baselines
  (scene 696adb1cd110e062, parity cce30c983b97b16d), MANIFEST
  verified. Pushed. Queued: the pickup consumer unit (7f.6
  cases 1-3 + 7 as sim seams behind the FUN_0040eba0 dispatch
  decode).
- CLOSED 2026-08-21 (P4 sidebar bars + score strip COMPLETE,
  commits a11e468 + 2035395 + 3f7fad7, worker 36c9e956 claim 1,
  D52): RE-EXW-SIM amendment 7f decodes the vitals family —
  FUN_0040807f (HP bar 0x18..0x46 @ (0x1E8+0x32k, 0x3C), armor bar
  word@+0x30 0x60..0x8E @ (slot_x, 0x49), exact clamps/idiv/cap),
  FUN_004085ce (NUMBERS.BIN strip: icon 0xA + 9 unsigned score
  digits / icon 0xB + 6 signed money digits, exact x tables),
  the CORRECTED FUN_00403938 tail order (bars -> strip countdown ->
  rows countdown), FUN_004072bf exact gates (+ the +0x2E HIT-FLASH
  correction — armor is word +0x30), FUN_0040e230 damage
  application (shield absorb +0x88, death path w/ debris RNG),
  FUN_0040eba0 cases (health/shield/drop/ammo/score-money), the
  armor producers (FUN_004100b7 +20 on type-DB +0x18 pad tiles vs
  -10/frame bleed, clamp 3000), the dropship-landing hp init
  (5000+battery*100), the score/money + NUMBERS.BIN census (23rd
  chain asset, sole consumer the strip). ENGINE (2035395):
  MissionScene draws the bars + strip from HOST-STAGED Vitals
  {hp,armor} (D52: hp = 5000+100*battery via the BATTERY PACK
  loadout group; armor 0 — the empty 0x8E bar draws every frame
  exactly like the original) + campaign session state (0/4000
  fresh) with the case-4 pickup seam (PICKUP_AWARDS, two rand_a
  draws from the shared sim stream, countdown 2); portrait hp>=1
  gate; the corrected tail order. Gates: 41 suites green (2 new
  unit tests), fmt/clippy -D warnings clean, smoke two-run
  byte-identical, MANIFEST verified; frame pins regenerated ONCE
  (spawn 9ecd7691d388bbfa, walk 333d128dc812d547, overlay
  1504c600819e724c stale-sidebar, armed 86a788ff93bd78a5), sim
  pins UNCHANGED (36ddc86345c8351c / f35db41f0efb858d /
  64ef1ddbc65cba47 — the damage path did not land). Pushed. P4
  sidebar follow-up queued: the damage unit (promote hp/armor to
  real sim fields + apply_damage + deliberate re-pin).
- CLOSED 2026-08-21 (P4 map-overlay family COMPLETE, commits 78b2506
  + 9cb8fbe + 59af1b3, worker 6d689cfd claim 1, unit finished across
  an interrupted predecessor run): RE-EXW-SIM amendment 7e decodes
  FUN_004089b1@0x4089b1 END-TO-END (clear 0x4b000 backbuffer ->
  TABLE.BIN image 0 the 480x480 RLE backdrop -> per-tile territory
  stamps: the TOT type-DB mirror words destructively advanced through
  the LNK image at 0x45cdda (the "0x45cdd8 table" of 7d.1 IS the LNK
  file), mask = MIN bank [0x4edd9c] (load_mission's .MIN load),
  color = MAPTRAN[variant[tile]] via FUN_00402ab8's 4x4 XLAT stamp at
  row'=0x80+r+c-2z / col'=0xf0-2r+2c -> GENERAL 0x55/0x56 robot
  markers at 2(tx-ty)+0xe4 / tx+ty+0x62-(z>>4) -> the PAD/order
  0x57..0x59 loop 0x408c94..0x408dc4 -> the NON-RETURNING JMP
  0x4072b8: overlay frames skip the whole sidebar tail). The
  territory variants = FUN_00408dcc's 11x11 Chebyshev ring max-stamp
  (dwords 0x454cf8, 7 center -> 1 corners) per moving robot.
  MAPTRAN/PALTRAN loaders pinned (FUN_00422171/FUN_0042209b - the
  MISSIONVIEW sec 8 u32[0x4dd444] producer question CLOSED: the
  PALTRAN ramp pointers, slot 0 NULLed after load). The toggle
  family: strip writes 0x4eb8dc=5 + toggles 0x4edba0; MissionShell
  decrements per frame (0x44871d); entry zeroes the bit (0x44786b);
  FUN_00401107 map mode presents the backbuffer 480x480 stride 640;
  overlay-on game-area clicks swallowed at 0x40b868; button chrome
  0x8f/0x5f/0x5e at (0x213,0x1b5), 0x5f dead code. ENGINE
  (9cb8fbe + 59af1b3): bedlam-render MapOverlay (TERRITORY_RINGS,
  stamp_territory, the lattice draw) + the mission chain tail
  (TABLE.BIN, MAPTRAN0..7.TRN, zone-level .MIN - 22 staged assets);
  MissionScene: the strip + lockout + overlay bit, the overlay frame
  (clear viewport half only - the sidebar keeps stale pixels,
  faithful to the screen), markers, chrome 0x5E per non-overlay
  frame, ring stamps for moving robots; PAD/order markers 0x57..0x59
  deliberately unwired (unmodeled order staging, never-invent).
  Gates: 455 tests green, fmt/clippy clean, headless smoke two-run
  byte-identical with hashes EQUAL to the prior commit; sim pins
  UNCHANGED (36ddc86345c8351c / f35db41f0efb858d), frame pins moved
  once (chrome: spawn b19a8034ee001253 / walk 1df4dfcb1e8b3eba /
  armed 0a22733e37c88a3c) + new overlay pins (frame
  f47217a154bf93c9, sim 64ef1ddbc65cba47); MANIFEST verified.
  Pushed. P4 sidebar remaining: HP/armor bars + score strip (queued).
- CLOSED 2026-08-21 (P4 weapon table COMPLETE, commits 5af9a70 +
  1c7b387, worker 4b75846d claim 1, D51): RE-EXW-SIM amendment 7d
  REFUTES the queued TABLE.BIN hypothesis (XRefList whole-program
  evidence): TABLE.BIN is the strategic-map OVERLAY backdrop bank
  (draw_IMG-family, image 0 drawn into the 0x4b000 map buffer by
  the sole reader FUN_004089b1@0x4089d5; per-tile map colors via
  the 0x45cdd8+2*type word table, PALTRAN/MAPTRAN .TRN kin;
  robot markers GENERAL 0x55/0x56, PAD/order markers 0x57/0x58) —
  NOT the weapon table source. The 0x4de664 0x62-stride table is
  .bss SESSION STATE: written only by the shop FUN_00440e45
  (buy/sell/auto-buy write 7-word groups name/ammo/price/cat/
  item/0/owned at type*0x62+group*0xE), the save-load restore, and
  the MP lobby exchange (0x4dd4a0 0x80-stride staging); player TYPE
  word@0x4edb90 = 0 all single-player (GameMain 0x41c34c boot
  write; MP lobby otherwise); fresh campaign = money 4000 SP /
  0x5DC mode-2 / 4000-500*difficulty, EMPTY loadout, shop before
  EVERY mission (GameMain loop: map room 0x43e7d4 -> briefing
  0x43d00b -> SHOP 0x40e45 -> MissionShell). FUN_00420260 name
  switch pinned exactly (39 strings 0x4589DD..0x458C11 + ERROR
  default, PE bytes). ENGINE (1c7b387): MissionScene models the
  loadout as host-staged 7x(name_idx, ammo) groups —
  GameHost::mission_mut + set_weapon_loadout re-running the exact
  6c.6 spawn-copy armer (1<<first group with word0!=0, 0 when
  empty) — with the faithful EMPTY fresh-campaign default
  (set_order_availability + the all-7 design default REMOVED);
  order-row click gate corrected to the AMMO word (sec 6c.3 — the
  +0x38+8k gate); row TEXT wired: weapon_name (the pinned switch
  embedded) + "%04i" counts through the new ui_bank draw_glyph
  (FUN_00402884 solid-color mask fill) at (0x1ED/0x25C, 0x5B+14i)
  color 0x24, FUN_00408913 advance rules (space 6 / glyph w+1).
  CRITICAL CODEC FIX en route: ui_bank draw_sprite RLE corrected
  to the FUN_00401ca2 asm — a literal control word with bit14 ends
  the line (EVERY shipped sidebar sprite row is one 0x4000|w
  word; the old decode painted each sprite as a single long row)
  and RLE transparency copies literal bytes VERBATIM (transp==0
  skip runs write zeros). Corpus gate: frame pins regenerated
  ONCE (default spawn 9f20732f29a5baf2 / walk 27494d6ab505bcf3,
  the empty default leaves the rows band black) + the new armed
  pin 51ebd515bc638e81 (staged NEEDLER#1+HADES#1: rows chrome +
  >20 name-text px at 0x24 + count pixels); sim pins
  36ddc86345c8351c / f35db41f0efb858d UNCHANGED (loadout never
  reaches the hash — pinned). 441 workspace tests / 0 failed,
  fmt + clippy -D warnings clean; headless smoke two-run
  byte-identical AT THE RECORDED BASELINE (scene 696adb1cd110e062,
  parity cce30c983b97b16d, audio 110400/158092); parity harness
  byte-identical on all four D28 anchors; MANIFEST verified before
  and after the corpus runs. Next per queue: the map-overlay
  family (7d.1 pinned its inputs).
- CLOSED 2026-08-21 (P4 mission sidebar ART COMPLETE, commits
  5860fe6 + abcbb37 + 805ed10, worker 49294e3c claim 1, D50):
  RE-EXW-SIM sec 6c.8 decodes the sidebar redraw pass
  FUN_00408403 in full (asm 0x408403..0x4085c6) + the whole art
  family: the 7 order rows over the selected robot's record (gate
  = group word0/name idx +0x36+8i, count = word1 clamped 9999,
  ARMED rows GENERAL.BIN sprites 0x47+0x4A / unarmed 0x49+0x4C at
  (0x1EB,0x59+14i)/(0x25A,0x59+14i) - 108x11 + 27x11 real
  geometry, name + "%04i" count text via FUN_00420260/
  BmpNameBuild + SMLFONT FUN_00408913 color 0x24); SEMANTIC
  CORRECTION - the "orders" are WEAPONS (the compiled-in name
  table 0x4589DD..0x458C0F: needler/plasma/hades/proximity/
  pressure/frag/bouncy/sticky/rocket/reaper/auto-shielding/
  battery/thermal/scanner; +0x6E = armed bits, word1 = ammo,
  FUN_0040eba0 case 8 = the ammo-refill producer, case 4 =
  score/money pickups); the banks pinned by asm ESI anchors +
  shipped bytes (GENERAL 0x4edd7c, SMLFONT 0x4ede7c, NUMBERS
  DAT_0046af3c for the FUN_004085ce score/money strip, SCANNER
  0x4edd80 for the deploy-panel sprite 0x12@(0x1EE,0xC3)); the
  sibling every-frame passes FUN_004072bf (portraits 0x12..0x17
  48x48 + HP dither + armor tick + blink cursor 0x51+ (0x4dc5d0
  producer open)) and FUN_0040807f (HP bar sprite 0x46-hp*46/5000,
  armor 0x8E-armor*46/2500) + the MissionShell initial trigger
  0x447c74 (both countdowns = 2). ENGINE (abcbb37): bedlam-render
  ui_bank codec (FUN_00401ca2 semantics, 5 tests incl. corpus
  GENERAL.BIN geometry pin); GENERAL.BIN + SMLFONT.BIN join the
  12-file mission chain; activate arms redraw 2; present draws
  the portraits every frame + the row chrome on the countdown
  (name/count text, bars, score strip, deploy panel + cursor
  deliberately unwired - unmodeled data, D50 never-invent rule).
  Corpus gate: sidebar-black pin -> sidebar-carries-art pin
  (4844 nonzero px); frame pins regenerated ONCE (spawn
  018eba568d9b3bae, mid-walk 4a3abd2de43f31df), sim pins
  byte-identical (D17 holds). Workspace tests + fmt + clippy -D
  warnings clean; headless smoke two-run byte-identical
  (GENERAL.BIN 128826 B + SMLFONT.BIN 4038 B fetched, scene
  696adb1cd110e062, parity cce30c983b97b16d, audio
  110400/158092); MANIFEST verified before and after. P4 sidebar
  thread: the strip is no longer black; remaining sidebar art
  (text/bars/score) is blocked on sim state, queued behind the
  TABLE.BIN slice.
- CLOSED 2026-08-21 (P4 mission sidebar producer COMPLETE, commits
  cfee256 + 490d856, worker 6ebe5cff claim 1): RE-EXW-SIM sec 6c
  decodes sidebar_control@0040d197 in full (decompile + objdump
  0x40d197..0x40d712 + a new tools/ghidra-scripts/XRefList.java for
  xref provenance): map-toggle strip x[0x213,0x24D] y[0x1B5,0x1CF]
  writes _DAT_004eb8dc=5 + toggles the overlay bit _DAT_004edba0
  (CORRECTS sec 6.2's old gloss that claimed it wrote DAT_0046cbdc);
  robot-select strips [0x1E7,0x217]/[0x219,0x249]/[0x24B,0x27B] x
  y[5,0x35] gated by squad size + the ALIVE dword -> DAT_0046cbdc +
  redraw DAT_0046ccec=2; order keys 1..7 + the 7-row strip
  x[0x1E9,0x275] y[0x57,0xB8] (row=(y-0x57)/14 clamp 6) toggle bit k
  of the ORDER-BITS word +0x6E gated by word +0x38+8k;
  DAT_0046ccec is a per-frame COUNTDOWN (the FUN_00403938 draw tail
  decrements it and calls the sidebar redraw pass FUN_00408403);
  FUN_00424a6e is an empty stub. The 0x62-stride type table at
  0x4de664 is structurally the 7x0x0E per-type ORDER table (spawn
  copies group word0->+0x36+8i, word1->+0x38+8i twice; order bits
  default 1<<first-available; player TYPE from word@0x4edb90);
  file source open ([hypothesis] TABLE.BIN). Field-table offset fix
  double-anchored (0x40d269 + 0x424810): alive=+0x7C@0x4c6a60,
  countdown=+0x80@0x4c6a64. ENGINE (490d856): MissionScene grows the
  sidebar presentation half (D17 - unit + corpus pinned that sidebar
  clicks never arm orders and never move the sim hash): click
  dispatch x>=0x1E0 -> sidebar_control, select strips with the
  squad/alive gates, 7 order rows with per-robot availability
  (default all-7 [design], set_order_availability host seam,
  spawn-default bits 1<<first), redraw countdown set 2 / decremented
  per present. Map-toggle + keyboard latches documented out of
  scope. 4 new unit tests + a real-ZONEA corpus gate pin block; all
  existing hash pins unchanged. 435 workspace tests green, fmt +
  clippy -D warnings clean, headless smoke two-run byte-identical
  AND identical to the recorded baseline (scene 696adb1cd110e062,
  parity cce30c983b97b16d, audio 110400/158092), MANIFEST verified
  before and after. P4 slice remaining: the sidebar ART producer
  (FUN_00408403 + its bank - the strip is still black), queued next.
- CLOSED 2026-08-21 (P4 modern audio output rates COMPLETE, commit

  4ed1e26, worker 2cd16045 claim 1): the device edge speaks modern
  rates. DECISIONS D47 + DESIGN-AUDIO Q1 ANSWERED: cpal output
  negotiation prefers 48000 Hz, then 44100 Hz, then mixer-native
  11025, then the device default - a pure choose_output_config over
  a neutral OutputConfigSpec (cpal 0.18's range is not
  constructible; fallback matrix unit-pinned without a device),
  ranked within a rate stereo > mono > other channels then S16 >
  F32 > other formats, rate dominating (48000 mono beats 44100
  stereo); wide supported ranges pin via try_with_sample_rate. The
  D40 Q16 frame stepper gained LINEAR INTERPOLATION (round to
  nearest, ties toward +inf, i64 internally since |delta|*frac
  overflows i32; a lone buffered frame edge-holds, an empty ring is
  exact [0,0] silence, the native rate keeps frac 0 = exact 1:1
  passthrough - D40's passthrough pin unchanged). The mixer bus and
  the parity stream stay 11025 Hz stereo u8 byte-faithful; only the
  callback converts. Tests: negotiation matrix, 44.1k quarter-ramp
  0/250/500/750 + 48k ramp literals 0/941/1882/2822/3763/4704,
  downsample blend, i16/f32/u8 silence + both full scales, u8
  128/255 end-to-end through the D31 bus into the ring. 428
  workspace tests / 0 failed; fmt + clippy -D warnings clean;
  headless smoke two-run byte-identical AND byte-identical to the
  pre-change binary (scene 696adb1cd110e062, frame parity
  cce30c983b97b16d, audio 110400/158092 unchanged); parity harness
  identical on all four anchors (chain 0xcae25cd08d7cbc08, sim
  0x72979d5d9dedc832, frame 0x87263f149564ad25, audio
  0xc862e45d2e95ad29); MANIFEST verified before and after; the
  opt-in live probe opens 48000 Hz 2ch i16 on this machine (was
  11025) and drains cleanly. P4 slice remaining: the Escape-exit
  window fix (queued next).
- CLOSED 2026-08-21 (P4 GAMEPAL mission present tail COMPLETE, commits
  663ddba + 7c25bfd, worker 1776dc60 claim 1): the mission viewport
  presents in color. DESIGN-GAME sec 11 amended (design commit
  663ddba) then implemented (7c25bfd): GAMEGFX\GAMEPAL.PAL (770 B,
  the parse_vga770 LOADPAL family; RE-EXW-MISSIONVIEW sec 6 GAMEPAL
  -> 0x4edbf8, RE-EXW-SIM sec 7c.3 the 0x302-B mission-load copy)
  joined the Mission fetch set in the GAMEGFX tail - SINTABLE,
  DANTE, GAMEPAL, then MRK (10 files) - folds with the exact
  loading_palette rule (>>2 lossless on 6-bit file values) and OWNS
  the mission plane: MissionScene carries the folded [Vga6; 256],
  plane() returns its own palette, render_now no longer passes the
  host stand-in, the frame palette IS GAMEPAL with palette_dirty
  every frame (MovieFrame seam; the indexed->RGBA window upload
  stays platform-side). Signatures: MissionScene::stage +
  GameHost::load_mission grew gamepal; the chain passes bytes[8]
  GAMEPAL, bytes[9] MRK. Corpus gate re-pinned ONCE (documented in
  the gate header): spawn frame a79fcada30ec5e50, mid-walk
  1b75b68ce66019e1; sim pins 36ddc86345c8351c / f35db41f0efb858d and
  the render-gate pins UNCHANGED; new structural pins frame.palette
  == folded GAMEPAL + palette_dirty + 254/256 non-black (entry 1 =
  6-bit 0x3E,0x3A,0x39). Headless smoke 25 fetches (GAMEPAL.PAL
  770 B) two-run byte-identical exit 0; parity harness
  byte-identical to the D28 anchors (chain 0xcae25cd08d7cbc08, sim
  0x72979d5d9dedc832, frame 0x87263f149564ad25, audio
  0xc862e45d2e95ad29); all workspace tests green; fmt + clippy -D
  warnings clean; release ok; MANIFEST verified after the corpus
  reads; D46 records the choices. P4 slice remaining: audio output
  rates, the Escape-exit window fix (queued next).
- CLOSED 2026-08-21 (P4 mission SCENE step COMPLETE, commits 26a11ef
  + e6de264, worker 74fa370e claim 1): the playable-slice composition
  landed. bedlam-game/src/mission.rs MissionScene per DESIGN-GAME
  sec 11 (design committed by predecessor a835cefc as a6317c5, whose
  WIP - the shared dat_plane_bytes loader + the public
  project_robot seam - was adopted and landed first as 26a11ef):
  staging = Terrain::from_mission_bytes + AngleTable(SINTABLE 2..66)
  + MissionSim seed 0x1E240 + robots_per_player(zone) MRK spawns +
  staged markers (the 0x46cbe0 network seam) + MissionView over the
  swept PRE-PAD planes with DANTE staged; lifecycle = movie pattern
  (inert until Mission, activate fixes the camera at robot 0 Q5,
  drop after leaving); per-frame = pointer integrate -> left-EDGE
  click seam (viewport x < 0x1E0, enqueue-projection hit box 0x20,
  nearest octagonal wins, arm AT the robot) -> advance_frame;
  present = enqueue_robots -> draw_terrain -> present_window ->
  480x480 at canonical (0,0) + black sidebar, one render per pump.
  Host: load_mission/mission_slot/mission_asset_names (episode
  arithmetic), the tick-loop drive, sync_mission, mission plane
  first in render_now. Shell chain: the Mission 9-file fetch set +
  stage_scene wiring + the GameGfxSource EDITOR tier for '/' names;
  headless smoke = 24 fetches, 20 mission pumps, two runs
  byte-identical. Corpus gate tests/mission_scene_gate.rs: scene
  frames pinned spawn 51ef4fe93eaaed77 / mid-walk 7bae11a5c7f34ab6
  + sim hashes 36ddc86345c8351c / f35db41f0efb858d, scripted
  click->arm at the projection (tile (21,73), snap to origin,
  state 3), walker state 4 live anim, sidebar black, two-run
  identity; the render-gate pins stay untouched. Parity harness
  output BYTE-IDENTICAL to the D28 anchors (the mission is inert on
  unstaged paths). D45 records the [design] choices. 422 workspace
  tests green, fmt+clippy clean, release ok, MANIFEST verified,
  pushed. P4 slice remaining: the GAMEPAL/window present tail +
  sidebar (queued next), audio rates, Escape-exit fix.
- CLOSED 2026-08-21 (P4 mission RENDER half 2 - ENTITIES, commits
  007237e + 186050b, worker e08e64c2 claim 1): the robot entity
  overlay decoded and wired onto the pinned frame. RE notes
  RE-EXW-MISSIONVIEW sec 5b-5d: per-frame bucket-grid clear (ECX
  0xa200 @0x46cdbc) + arena reset; FUN_0040798e node/bucket/
  insertion semantics (48-B nodes, bucket (wx>>5 - camTx +9)*4 +
  (wy>>5 - camTy +9)*0x90 + layer*0x1440, sort = wx+wy ascending,
  stable after equals, head-insert); the terrain-loop flush site
  (per cache cell per layer, gate 0..0x24, next @+0x20); FUN_0040179b
  asm-authoritative (directory entry 2+4*id with the fmt word SKIPPED,
  forced u16-RLE, literal runs RAW-copied with NO zero-skip - mode
  0x130 paints 0xFF, 0x12c/300 plain, 0x12d/0x12e TXPAL1 64-KiB
  composition / 0x12f DARKPAL XLAT only with the water flag on);
  the robot loop field map (sx/sy iso projection + 0x23f clip,
  shield sy-0x48 mode 0x12e, body DANTE[anim], variant/overlay/
  +0x20 sprites; spawn defaults => DANTE[anim] + DANTE[0x20]);
  SIM sec 3 correction: the deploy countdown is u16@+0x16, +0x14 is
  the frame-base word. Engine: mission_view.rs SpriteList +
  RobotView + enqueue_robots + flush_node + the draw_terrain flush;
  corpus gate: ZONEA/MISSION1 spawned robot + order-walking second
  robot from MissionSim on real bytes drawn with real DANTE.BIN
  (160 sprites) - spawn frame pinned 8d2c559df035b75b, mid-walk frame
  8804f9deec6b1fee, terrain pin 90a9e929eea24ced kept as the
  no-entities regression pin. 5 hermetic entity tests; 413 workspace
  tests green, fmt+clippy clean, MANIFEST verified, pushed.
- CLOSED 2026-08-21 (P4 mission RENDER half, commits 02363f6 + 889d6b0,
  worker b9aaaa38 claim 1): the isometric viewport draw chain decoded
  and rendering ZONEA/MISSION1 to a hash-pinned frame. New
  docs/RE-EXW-MISSIONVIEW.md (ghidra dumps exw-missionrender{,2,3}.txt,
  scripts ExwMissionRender{,2,3}.java, -process -noanalysis x3):
  init_tiles@00407e11 = the 36x36 2:1 iso viewport cache at
  DAT_004ede24 (grid origin (0x130,-0x100), +32/+16 steps, sticky
  anchor 17, 467 cells) + the TOT 8-plane word mirror into the
  0x1e-stride type-DB records at 0x4796bc (8 words + 8 seen bytes at
  +0x10 + zero-filled tail); LNK = the PER-FRAME tile ANIMATION link
  (word -> LNK[word] walked and memoized back every drawn frame);
  BIN = MISSION{A..G}.BIN the terrain sprite bank (u16 count + u32
  offsets relative to entry 2+4*id); FUN_00401471 blit codec (fmt 0
  raw 64x64 skip-0 / fmt 1-3 u16 RLE bit15-ctrl bit14-eol low12 /
  fmt>=4 u8 RLE bit7-ctrl bit6-eol low6+1; stride 640; XLAT remap);
  FUN_00403938 terrain loop (camera tiles, 8-layer bottom-up walk,
  0x5000/level, seen-chase columns, 0x59b00 draw cap, off-map edges
  via FUN_00408030 per zone); sprite-list enqueue FUN_0040798e +
  flush FUN_0040179b (entity overlay seam, decoded not yet wired);
  present FUN_00401107 = the 480x480 window at buf+0xa040 + fine-cam
  offset (camera 0 -> (96,64)). Engine: bedlam-render mission_view.rs
  (MissionView + DrawParams + present_window, hermetic, per-write
  bounds) + corpus gate mission_view_gate.rs: cache geometry/anchor
  pins, deck mirror + seen semantics, codec pixels on sprite 0,
  one-LNK-step-per-frame walk (visible tiles only - off-camera words
  frozen, layer-0 cap respected - faithful), frame hash pinned
  90a9e929eea24ced (camera (0,0), frame 0), two-run byte identity,
  zone-0 fixed edges vs zone-1 stream sensitivity. Corrected en route:
  the cache anchor is 17 (first in-bounds cell (12,4)), not 21; TOT
  plane stride is the standard w*h*2 (decompiler artifact fixed from
  asm). 407 workspace tests green, fmt+clippy clean, release build
  ok, MANIFEST verified, pushed.
- CLOSED 2026-08-21 (P4 slice tail, commits 5381bea + c4f615a +
  055879e, worker d8c46c88 claim 1): the mission file-load +
  table-build pass decoded and wired. docs/RE-EXW-SIM.md amendment 7c:
  load_mission@0041dc5a (EDITOR\ZONE{x}\MISSION{n} / zone-level path
  prefixes from build_mission_paths@0044670c; TOT/DAT/CGR/BIN/MIN/LNK
  arena loads; map w/h from the TOT header; y-line table 0x4ea900 =
  y*w for y in 0..=h, z-base 0x4eaacc = z*w*h for z in 0..=7; >=0x80
  sweep planes 0..6; PAD records staged 8-byte and written
  DAT[kind*w*h+y*w+x]=0xFF with NO bounds check; CGR height byte at
  2+4(type-1)+dir[type-1]+6+(sy<<5)+sx - RAW 1024-byte 32x32 height
  maps, NO codec, correcting FORMATS-MISSION 18; MRK word 3 = spawn
  z LEVEL feeding z=word3*0x20-1, robot i takes record i verbatim;
  the order armer FUN_004247b5 has a single caller, the robot-sprite
  click family 0x433cbc - the verified move producer stays the
  order/walk path, and no shipped mission spawns two markers within
  the 6-tile order radius, so a second walker on a real map is a
  staged marker, exactly what the network override 0x46cbe0 does).
  FORMATS-MISSION rows updated (DAT/MRK/PAD/CGR semantics confirmed).
  Engine: Terrain::from_mission_bytes (hermetic loader rules) +
  corpus gate engine/bedlam-core/tests/mission_corpus_gate.rs - ZONEA
  25x75 loader pin (deck floor z 31, type-37 wall column reads z 1 =
  climb 30 = the real-map wall, PAD mark materialises), MRK[0]
  (21,73,1) spawn settle z 31, staged second robot order->walk 4
  tiles east on the real bytes (arrival snap from the west lands one
  tile short of the target origin - faithful 0x1400-radius + grid-snap
  semantics), state hash pinned at spawn/arm/arrival with the 7-frame
  EXW cadence + two-run identity, and ZONEB/MISSION1 MRK[0] (27,71,3)
  settling at z 95 on the 3-deep deck stack. All workspace tests
  green, fmt+clippy clean, release build ok, MANIFEST verified,
  pushed. P4 slice remaining: the isometric viewport RENDER half -
  queued as the next Now item (init_tiles@00407e11 + the draw chain).
- CLOSED 2026-08-20 (P2d sim-tail slice, commits c33f615 + 6280857,
  worker 778d091a claim 1): the mission-sim seam for the P4 vertical
  slice. docs/RE-EXW-SIM.md amendment 7b re-verified the contested
  facts from the binary (move_is_possible per-probe climb refs =
  probe_z[i] sar-signed with 0xFFFF = -1, no writes on any probe
  fail; dist_octagonal abs's BOTH args - always >= 0; armer snap =
  tile ORIGIN tx<<13 with no +0xF00; spread table slots 0..8; spawn
  settle best-effort - seeds L*0x20-1 can never settle a tall floor).
  engine/bedlam-core mission.rs adopted from the interrupted e1eb0092
  WIP and driven 6/9 -> 9/9: Terrain (DAT planes + CGR height
  sprites, get_z_pos search/latch/0x1F rule), Robot record subset,
  Order + spread claims, MissionSim 6-phase frames + order-window
  tick, robot_move diagonal/slide/move_x_y_who, move_possible
  per-probe, state hash over the sec-7 coverage list. 38 workspace
  test binaries green, fmt/clippy -D warnings clean, release build
  ok, MANIFEST verified x2, pushed. P4 slice inputs now complete
  except the mission file-load/table-build pass (RE-EXW-SIM sec 9
  item 1) - queued as the next Now item with the ZONEA/MISSION1
  render + one squad move.
- CLOSED 2026-08-20 (P4-menu engine step, D42, commits 57413b0 +
  0a10a54 + 7ff713e, worker 26ccbd31 claim 1): the D41 title-menu
  findings implemented. bedlam-game menu.rs = TitleMenu (builder
  semantics for menus 1/2/3/5 with count word + 7 slots, strip hit
  test, hover/click SFX debounced 4 ticks, bottom-anchored draw with
  the dual glyph bases - font.rs from_bank_at/draw_at, name entry
  with the 0x8e cursor blink + explicit typing API + GOD default,
  attract replay at idle >= 0x300 via MoviePlayer restart/finish,
  menu-1 actions incl. start (seed 4000-diff*500, cached on the
  host) and quit-confirm). GameHost: load_title_menu staging
  (LANGUAGE + FULLFONT + FULLPAL + MENU1/MENU2 RAW as instruments
  0xE0/E1), the menu OWNS the Title input path (fsm fed neutral
  frames - hash-isolated, unit + corpus pinned), staged-inert
  lifecycle, menu plane after the title movie. Shell chain: Title
  fetch set = TITLE + LANGUAGE + FULLFONT + FULLPAL + MENU1/2,
  GameGfxSource SOUND/SFX tier. Corpus gate tests/menu_gate.rs
  (MENU_ITEMS table, difficulty cycle, strip geometry, green 233..=
  244 vs blue 244..=255 ramp pin end-to-end, start handoff, SFX
  audibility, TITLE.SMK restart, scripted two-run byte-identity).
  393 workspace tests / 0 failed, fmt + clippy -D warnings clean,
  headless smoke two runs byte-identical, parity IDENTICAL to the
  D40 baseline 143e60d, MANIFEST OK x2. Open: backdrop content
  (RE-EXW-TITLEMENU sec 8), HOF/credits/save/coop stubs, CONFIG.BDL
  writer, OPTIONS music. Remaining for the P4 slice exit: ZONEA/
  MISSION1 render + one squad move (needs P2d sim tail).
- CLOSED 2026-08-20 (P4 native shell step 2, D40, commits 58eb8a6 +
  c48cd91 + 143e60d, worker e76159bb claim 1): platform audio output.
  cpal 0.18.2 (bedlam-shell only; mixer stays hermetic, un-hashed):
  bounded stereo-frame ring (4096 frames; full = drop OLDEST, underrun
  = exact [0,0]) behind a poison-tolerant mutex - the ONE thread
  crossing; window loop the ONLY producer (watermark fill 736 frames
  after each pump batch), cpal callback the only consumer. Device
  config pinned at the native 11025 Hz when any supported range
  contains it (stereo > mono > other; this machine's Pulse/ALSA
  default accepted 11025/2ch live - #[ignore]d probe), else device
  default through a Q16 nearest-neighbor frame stepper (4x = exact
  repeats; 48k/8k step values + sample-hold positions unit-pinned);
  mono floor-average (l+r)>>1; formats via dasp conversions; no
  device = stderr note + silent run, never fatal. Headless smoke now
  drains 184 frames/pump off the host bus (110400 = 600x184, 158092
  non-silent samples) - scene/frame hashes IDENTICAL to the pre-
  change binary, two runs byte-identical, MANIFEST OK x2, workspace
  366 tests / 0 failed, fmt + clippy -D warnings clean. Next per
  queue: menu/ZONEA/MISSION1 playable vertical slice (P4 exit).
- CLOSED 2026-08-20 (P4 native shell step 1, D38/D39, commit 493fbd5,
  landed by the watchdog repair agent after a step-cap death spiral):
  bedlam-shell crate = window + surface + fixed-step present loop.
  FixedStepClock (pure u128 integer banking, anti-spiral clamp 4,
  surplus dropped not fast-forwarded); input seam map_physical_key
  pinned (winit KeyEvent has a pub(crate) field - NOT constructible
  outside winit; predecessor test rewritten); D31-D37 chain fetch
  layer (scene_assets + stage_boot/stage_scene); env-gated (--window
  / BEDLAM_SHELL=1) winit 0.30.13 + wgpu surface host (Arc<Window> ->
  Surface<'static>, Fifo vsync, D20 PARITY present); headless smoke
  (600 fixed pumps, scripted campaign walk, two runs byte-identical,
  fetch set exactly the 10 D31-D37 corpus files); two-tier
  GameGfxSource (GAMEGFX/<name> then <root>/<name> - LANGUAGE.ENG at
  install root). bedlam-platform +ParityGpu::new_for_surface. The
  WIP survived FOUR GLM workers killed at the opencode2 step cap
  (orchestrator default agent, steps:60, edit denied) - cumulative
  work by 486a18e1/8d2f7acc/3a5e5f9e/f24c9332, fixed (impossible
  KeyEvent test, saturate-bank assertion, usage string) + verified +
  landed by repair agent 410671: 356 workspace tests green / 0
  failed, fmt, clippy -D warnings, MANIFEST OK before AND after.
  CONTROLLER FIX (same repair): nudge workers now launch with
  --agent build (no step cap, edit allowed); step-cap truncations
  classify as 'step-cap' and no longer feed the taskfails/cooldown
  spiral; the llm-watchdog check prompt flags the signature. Next per
  queue: native shell step 2 (cpal audio output).
- CLOSED 2026-08-20 (P5 BRF_DROP briefing intro pair, D37, commits
  3a2981d + bba01fe + 40b3700): the BRF_DROP play site located and
  wired - the EXW briefing screen (FUN_0043d00b; RE corrected the
  prior gameplay-advance gloss) opens BRF_DROP.SMK FIRST at every
  movie-enabled briefing (asm 0043d447..0043d490), one full-screen
  pass, then the constructed BRF_{zone}{level}.SMK backdrop ring
  until UI exit (letter = zone + 0x40, zones 2..=6 = B..=F; D33
  open note resolved; open failures fatal; GO arms after handoff).
  Engine: bedlam-game brief.rs BriefIntro Staged->Drop->Backdrop
  (drop hard-capped frames-1, starvation-proof; backdrop ring
  unbounded; entry audio at start + handoff); GameHost
  load_briefing on the D31 lifecycle (inert-until-Brief, drop +
  stream clear on exit, hash isolation unit-pinned); latent D31
  MoviePlayer ring-Last bug FIXED (rings froze at their first
  cycle end; now wrap 512->1 and continue; SHOP.SMK inherits the
  fix). Corpus gate tests/brief_gate.rs: drop max frame 28 =
  29/30 rendered, handoff at closed-form pump 58, zero PCM, 2+
  ring cycles, two runs byte-identical. Code by predecessor
  3d88a359 (died after bba01fe leaving the DECISIONS/RE-EXW docs
  uncommitted; adopted + 342->343 test recount corrected by this
  run), verified + queue-closed by run 5a637669 (claim 1): 343
  workspace tests green / 0 failed, fmt + clippy -D warnings
  clean, MANIFEST.sha256 OK before AND after. All P5 D31-D37
  movie/play sites now wired. Next per queue: native executable
  shell step 1 (window + surface + fixed-step present loop, P4).
- CLOSED 2026-08-20 (P5 boot attract sequence, D36, commit 8738a03):
  the region-variant publisher pair plays on the Boot scene. RE
  prerequisite landed by predecessor as 4e9ccbb (RE-EXW-GAMETHREAD
  "Boot attract arm RE": FUN_0044567c runner - one-pass bound
  frames-1, dst 480-2*arg2 geometry incl. the TITLE replay arg2=0x50
  letterbox that verifies D31 centering, per-frame 256-entry palette,
  screen cleared twice per call, skip gate 004edbc4 => boot pair
  unskippable). Engine: bedlam-game boot.rs BootAttract
  Staged->Playing->Done (EXW order GTLOG then LOGO, movies::boot_pair,
  time-exact switch at (frames-1)*period on the x240-us grid, entry
  audio per movie, Done holds the last raster);
  MoviePlayer::advance_limited hard decode cap (EXW loop bound,
  starvation-proof); GameHost load_boot_attract on the D31 lifecycle
  (inert-until-Boot, dropped + stream cleared on exit, scene-hash
  untouched - unit-pinned). Corpus gate tests/boot_attract_gate.rs:
  both region pairs to Done at 60 Hz, max decoded frame = frames-2
  (68/69 of 70/71 - ring never wraps), switch/Done pump counts by
  closed formula, continuous in-order DPCM >100 kB per pair, two
  runs byte-identical. Rust WIP of interrupted predecessor 19dc859e
  (died on transport error after the docs commit) adopted, validated
  + completed by run 7d041b7e (claim 1; clippy tail only). 335
  workspace tests green / 0 failed, fmt + clippy -D warnings clean,
  manifest OK x2. All D31-D36 movie play sites now wired. Next per
  P5: BRF_DROP.SMK play-site RE (queue item 1).
- CLOSED 2026-08-20 (P5 FULLFONT loading-text glyph pass, D35, this
  commit): the four LAB_0041c69e text draws + the FULLPAL font-ramp
  copy run in GameHost. bedlam-game font.rs = FUN_0043c87c (measure/
  draw passes, x0 = 0x140 - total/2, space +9 / glyph w+2, RLE16
  transparent blit, hotspot dy->row dx->col baseline anchoring,
  FUN_00410493 accent remap with the shipped e-/o-diaeresis dash
  quirks, overlay glyphs at entry 0x82+0x6b+id = 238..=241);
  bedlam-assets language.rs = the LANGUAGE.* [MENU_ITEMS] table
  (strings = entries 0x45/0x46/zone+0x51/0x58; the DAT_0046bc4c/7c/
  bfdc globals are table base + idx*0x30); pal.rs parse_font_ramp =
  the 98B FULLPAL ramp (lead e0 20) that replaces fade-target
  entries 224..=255 after the draws (EXW order: 0x3f transient ->
  draws -> ramp -> FadeSetup). D34 row/y swap CORRECTED: 0x82 is the
  glyph entry base; 150/180/210/260 are draw ROWS. Host
  load_loading_font stages inert; corpus gate tests/font_gate.rs
  (FULLFONT 390 entries / 333 glyphs, ASCII pixel set {0} U
  {233..=244}, dy {0,5,10,15}; FULLPAL + 6 LANGUAGE files pinned;
  independent width re-measures). 15 new units; 326 workspace tests
  green, fmt + clippy -D warnings clean, manifest OK x2. WIP of
  interrupted predecessors adopted + completed by run 315d2af1
  (claim 1). Next per P5: boot attract LOGO/GTLOG sequence (queue
  item 1).
- CLOSED 2026-08-20 (P5 post-cutscene loading flow, D34, d834f08): the
  EXW LAB_0041c69e zone-transition tail runs in GameHost as a
  presentation-only flow (bedlam-game loading.rs, LoadingFlow
  Staged->Between->Loading): BETWEEN.BIN entry 0 owns the Cutscene
  plane after the cutscene movie ends (standing host palette); the
  region-variant loading screen (LOAD_UK/US.BIN + LOADPAL/LOADPALU,
  path-selection only) owns the Select plane with the 10-step 20 ms
  50 Hz fade on the x240-us accumulator grid; DAC tail entries
  224..=255 forced 0x3f (buf bytes 0x2a2..0x301); text row pinned
  (y=0x82, x=150/180/210, zone-6 +260, stage-1 pre-increment
  reconciliation) as TextRow state for the queued FULLFONT glyph
  pass; endgame arm (MAX_STAGE) drops the flow; skip-advance still
  runs the loading screen; scene-hash untouched (D17-b). 14 new
  units; 311 workspace tests green, fmt + clippy clean, manifest OK
  x2. WIP of interrupted predecessor 3977d55d adopted, doc fix + D34
  DECISIONS entry + bookkeeping by run f807449c (claim 1). Next per
  P5: FULLFONT.BIN glyph pass over the pinned text row (queue item
  1).
- CLOSED 2026-08-20 (P5 loading-screen asset path, this commit): the
  LAB_0041c69e zone-transition tail assets are decoded + PINNED
  (bedlam-assets tests/loading_gate.rs, 3 tests + ignored regen):
  BETWEEN.BIN / LOAD_UK.BIN / LOAD_US.BIN are single-image 640x480
  rle16 banks (flags=3, hot=(0,0)) through the existing
  sprites::parse_bin_images - no decoder changes owed; 1:1 blit into
  the 640x480x8 render Frame (no letterbox/scale). LOADPAL/LOADPALU:
  770B VGA palettes, 244 distinct, entry0 black/entry1 white.
  CORPUS FACT: LOAD_UK == LOAD_US and LOADPAL == LOADPALU
  byte-for-byte - the EXW region split selects paths, not content;
  doc note added at Region::loading_pal (bedlam-game movies.rs).
  Content pinned via file sha-heads + decoded-plane sha256s. Next per
  P5: the post-cutscene loading-screen FLOW in GameHost (queue item 1).
- CLOSED 2026-08-20 (P5 shop + briefing backdrops, D33, 1b3ef85): Shop
  and Brief scenes play their SMK backdrops through the D31 movie
  lifecycle - GameHost::load_shop (SHOP.SMK 61-frame 40 fps ring behind
  the shop UI), GameHost::briefing_name + load_briefing
  (BRF_{B..F}{sub}.SMK from the hashed episode slot;
  movies::briefing_name_for_slot: stages 2..=6 -> letters B..=F = the
  25-file corpus domain, sub = lowest-unset mask bit + 1 = the
  Episode::complete arithmetic, boot camp + endgame stages -> None - no
  BRF_A/BRF_G exists in the corpus). 6 new units (3 selection incl. the
  corpus-domain cross-check, 3 host lifecycle through the FULL_MASK
  campaign). Commit landed by worker a1ad7346 which died after push,
  before the queue rewrite; run ed15e708 (claim 1) adopted +
  independently re-validated: workspace 294 tests green / 0 failed with
  all 6 D33 units passing, fmt + clippy -D warnings clean,
  MANIFEST.sha256 OK before AND after the corpus runs. Next per P5:
  loading-screen asset path (BIN image-bank decode), then the
  Cutscene->Select flow.
- CLOSED 2026-08-20 (P5 cutscene movies + corpus inventory, D32): every
  game-data SMK inventoried and PINNED (bedlam-assets smk_corpus_gate:
  34 files, formats/rates/ring/y-scale/audio shapes; listing must match
  the table both ways). Reject-or-map verdict: ALL MAP onto the D31
  playback path, none rejected - y-scale None corpus-wide (no scaling
  logic owed), all periods exact on the x240-us grid, the single audio
  shape (DPCM mono 8/11025) is already stream-bus-native. Movie
  selection module (bedlam-game movies.rs): cutscene_name over the
  hashed stage (ZONEDONE.SMK; END.SMK at the endgame = stage >=
  MAX_STAGE, EXW pre-increment vs FSM post-increment reconciled and
  unit-pinned through the FULL_MASK cadence), Region (DAT_0046ae64)
  backing LOAD_UK/US.BIN + LOADPAL(U).PAL + LOGO/GTLOG variants,
  briefing_name over BRF_{B..F}{1..5}. Host wiring:
  GameHost::cutscene_name + load_cutscene = the D31 lifecycle on
  Scene::Cutscene (inert-until-scene, dropped on exit, hash-free).
  Workspace 257 tests green, fmt + clippy -D warnings clean,
  MANIFEST.sha256 verified before AND after the corpus runs. Next:
  Shop/Brief backdrop wiring, then the post-cutscene loading screen.
# STATE - project snapshot (update when phase changes)

 - CLOSED 2026-08-21 (P4 7j.11 FUN_00420608 kind census unit
   COMPLETE, commit 199fe32, worker 804e8c9d claim 1, D59,
   docs-only): RE-EXW-SIM amendment 7j.11 answers the 7j.10
   tail note — the 0x4203a5 FUN_0042394a call is NOT in a
   debris kind body but inside FUN_0042034c, the DELAYED-ARRIVAL
   SCHEDULER (MissionShell epilogue 0x448076; 45 records
   @0x4dcdb8 stride 0x24 {active, two xy pairs, spawn xyz,
   countdown, robot slot}: countdown 0xa SFX, the 0x465daa word
   gate (both banks cleared at the tile), the FIRST water-level
   z-structure CLEAR via FUN_0042394a (arg order pinned: eax=x,
   edx=y, ebx=z, ecx=word, stack=byte), the robot teleport
   x<<13/y<<13/z<<5-1 + FUN_0041e231 re-settle + the 8-word z
   fill at robot+0x1a). The stager body itself: ZERO type-DB
   references, ZERO z-writer calls — no debris kind edits
   terrain beyond the FUN_00422287 rings. The 20-kind table
   fully pinned (11 seq tables 0x454424..0x454510 = BLOWUP
   sprite walks 0..104, +0x20 physics classes 0/1/2/3/6, inits
   0x40/0x20, FUN_00421e60 3-way + FUN_00421dec 4-way arrival
   SFX, k11's FUN_00402975 LCG gate). CORRECTION to 7j.9 item
   4: kinds 1/13/14/15 DO write the nine ring (shared body
   jmp 0x4209e9 into the k20 tail); kinds 2/8 write ONE center
   tile (values 3/4); only 7/10/16..19 are ring-free. Complete
   47-site caller census: every kind except k5 (the death
   tail, engine-landed D53/D57) lives in the weapon-fire/
   impact families, the FUN_00422693 platform/destructible
   family, the selection chaser, or FUN_004244a1 — all off the
   current corpus path. NO engine change (D59 — the census
   feeds the later widening); manifest verified before and
   after; pushed. Queued: the FUN_00422693 platform/
   destructible family decode.

 - CLOSED 2026-08-21 (P4 effect-row seam unit COMPLETE, commits
   4f858d9 + e706a33 + 9bbf1ac, worker 6ab53863 claim 1, D56):
   RE-EXW-SIM amendment 7j decodes the whole 0x4dc5d0 producer
   family the 7f.4 sidebar switch consumed with "producer open":
   the 10 effect rows are 16-B records {x,y,z,id} at 0x4dc5d4
   (FUN_00422038 = the id-word allocator, first-free else row 9;
   FUN_0042205c = the z += 6 rise-tick to the 0x190 cap then
   free, MissionShell epilogue 0x448080 before the draw; the
   FUN_00403938 tail draw enqueues FLAGS.BIN sprite id-1 layer
   0x12c with its own +0x118/+0x124 projection; the effect-id
   table completed to {1,6,7,1,0xE,0xC,0xD} per pickup case
   {1,2,3,4,7,8,9}); the scalar _DAT_004dc5d0 is a SEPARATE
   variable = the blink-cursor selector (the selected robot's
   slot + 1; producers the robots() select-ack blocks
   0x40c1ae..0x40c25e + the MissionShell entry zero; consumer
   the 0x407420 switch drawing GENERAL 0x51+(frame&3) at
   (0x1F0+0x32k, 0xD)). FUN_00420608 = the 128-slot 0x30-stride
   debris stager (z clamp 0x20..0xFF, first-free-else-min-seq
   LRU, 20-kind jump table; kind 5 = the death debris with SIX
   FUN_00422287 ring writes per debris = the per-tile type-DB
   +0x18 byte writer, CLOSING the MISSIONVIEW §8.1 producer
   question with an armor-pad reader caveat; the 0x454424
   kind-5 i16 seq table {5..0x10, -1} walked by the FUN_00420549
   tick; the draw pass reads BLOWUP(B/G).BIN, 0x12c for kinds
   3/7/0xA else 0x12e). ENGINE: bedlam-render NodeBank::
   {Flags,Blowup} + enqueue_effects (verbatim projections/
   bounds/modes); bedlam-game EffectRows + DebrisFx presentation
   state staged by the damage/pickup seams over the D53/D54
   outcomes, ticked in the epilogue order (overlay frames too),
   the blink cursor on the select-ack; FLAGS.BIN + BLOWUP.BIN
   join the 25-file mission chain. Gates: ALL pins UNMOVED (the
   effects draw nothing on the default corpus path, the cursor
   is 0 until a select click) — the scene gates pass
   byte-identical, the smoke two-run byte-identical AT the
   recorded baselines (scene 696adb1cd110e062, parity
   cce30c983b97b16d, fetch list 25); new: 3 render units + 6
   game units + the corpus effects gate (control-host diff at
   the same pump index — the LNK walk animates every frame, so
   consecutive-frame identity is not a valid invariant — plus
   two-run determinism). 41 suites green, fmt/clippy clean,
   MANIFEST verified before and after the corpus reads. Pushed.
   Queued: the 7j.8 scorch/armored-pad reader re-verify (+
   scorch wiring if clean).
- CLOSED 2026-08-19 (P5 title-movie playback, D31): TITLE.SMK plays
  through GameHost end-to-end - MoviePlayer fixed-step x240-us clock,
  compose-level MovieFrame (scene pipeline replaced while a movie
  plays, centered letterbox, palette fold PALMAP>>2 lossless), mixer
  PCM stream bus (native u8 mono 11025 Hz FIFO under voices, loud
  16 MiB cap), inert-until-scene host lifecycle with scene-hash
  isolation pinned. Full-playback gate green (pacing exact vs the
  accumulator math, composite byte-identical to an independent
  SmkStream walk, two playbacks identical). Workspace 280 green,
  fmt/clippy clean, manifests OK x2. Next per PLAN sec 6 P5: extend to
  cutscene movies + per-zone parity gates.

- CLOSED 2026-08-19 (P4 SMK decode gate, smk-stream unit): headless TITLE.SMK
  decode gate green via the codec-neutral SmkStream seam (D30) over vendored
  smk 0.1.0 - 640x320, 1227 frames, 66660us/frame, DPCM mono 8-bit 11025 Hz
  track 0; two full decode passes byte-identical (video/audio SHA-256 chains
  in NEXT.md run notes); vendored backend DPCM panic patch documented in
  bedlam-smk/NOTICE.md. fmt/clippy/tests green, manifests OK. Next phase per
  PLAN sec 6: P5 playback integration (TITLE.SMK into GameHost/presentation).

- CLOSED 2026-08-18 (P2 cosmetic tail, 119ba2d+b6620c0+007fbe5+4ace8a6):
  B2 census sec-7 residuals ALL CLOSED (census sec 7.7a-e). Campaign
  tables byte-pinned (order[8] = {3,0,1,5,9,13,17,21}; full 27-step
  idx list; 25 distinct indices = union over stages 1..7). 25-vs-27
  RESOLVED by static arithmetic - no playthrough needed: linear counts
  completions (27), formula indices are distinct table slots (25); the
  gap = two endgame completions at stage-slot 8 via the OOB order[8] =
  zone[0] sentinel hop (0x81dba + 8*4 = 0x81dda exactly). 4f02 =
  BANKED 0x101 (BX verbatim caller passthrough at 0x12439, zero 0x4101
  constructions in the 671-fn sweep, g_lfb_ptr + g_vesa_mode_req
  write-only dead). Display start 0x200 = SCANLINE units (page-B bank 5
  = 0x50000 = 0x200 x 640-byte pitch; 4f07 DX-scanline form). B2 fade
  chain named + documented (B2FadeStep@0x126c8 8.8-fixed 768ch serviced
  at 50 Hz in the ISR &1 sub-block - RATE CORRECTED on close-out verify,
  identical to EXW 200 ms fade, no divergence; setup/cancel/dacread/
  dacupload/fadewait + 3 labels persisted;
  B2LblFix repaired 2 mislabels, primaries restored). Persistence
  re-verified 14/14 (B2ResidVerify). No import (1x -process
  -noanalysis); manifests OK x2. P2 cosmetic queue EMPTY; P4 runtime
  half remains, interactive-gated.

- CLOSED 2026-08-18 (P2 cosmetic, 8f5f18f+94a65da): EXW DD surface
  creation-order CONFIRMED (RE-EXW-TICK new section): 004ee9bc =
  flip-chain head/primary; 004ee9c0 = implicit backbuffer (fullscreen
  GetAttachedSurface) / offscreen staging (windowed) - g_dd_surf_staging
  correct in both modes; FUN_0044a9ac = DDStagingProbe (sentinel
  survive-a-flip readback -> g_staging_persistent 004ee9e4); 004ee9b4
  dual-use corrected (lo = master vol, hi = palette re-attach flag;
  RE-EXW-MUSIC addendum). Trampoline CrtThreadTrampoline@00451fbc +
  usage roles were already persisted by the tick-sat run; this pass
  added the creation-order proof + names. No import; manifests OK x2.
  P2 cosmetic queue now: only the B2 census sec-7 residuals item (in
  flight). P3 charter complete; P4 runtime half still interactive-gated.

- CLOSED 2026-08-18 (P4 kickoff code half, c61d7f7): headless parity
  harness v0 example landed (engine/bedlam-game/examples/parity_harness.rs,
  D28): GameHost driven end-to-end over a recorded input script, JSON
  report with per-tick scene-hash chain + frame parity + sim hash + audio
  stream hash; .MRW banks loaded per track (audible baseline); verified
  byte-identical across runs; fmt + clippy -D warnings clean; workspace
  204 green unchanged; manifests OK x2. P4 runtime half (wine/DOSBox
  comparisons vs this CPU baseline) = next, needs interactive desktop.

- CLOSED 2026-08-18 (game unit, 4ab051c+7e3e472): P3 CHARTER SET COMPLETE.
  bedlam-game = the LAST charter crate (assets/core/render/platform/audio/
  game all landed as skeletons). Scene FSM (10 scenes, B2 episode shape
  {stage,mask,linear} + FULL_MASK@0x81d9a, D26 hashed per-tick edge
  latches), host pump in FUN_0043d00b order, MusicPump bridge (D27
  melody-chunk + attach-anchored mixer dispatch), typed OPTIONS.BDL.
  Workspace 204 tests green, fmt + clippy -D warnings clean, manifests
  OK x2. Next phase per PLAN sec 6: P4 (harness/playable) - first item
  = dependency/version spike + runtime smoke, needs interactive desktop
  for wine-exw (do NOT run unattended).

- CLOSED 2026-08-18 (P4 runtime unit, unattended subparts, 79227e5+11c8d9c+b951e7c):
  D28 anchors REPRODUCED byte-identically x2 runs (scene
  0xcae25cd08d7cbc08, sim 0x72979d5d9dedc832, frame 0x87263f149564ad25,
  audio 0xc862e45d2e95ad29; reports cmp-identical). DOSBox-X harness
  LANDED: flatpak static-home finish arg DISCOVERED (per-dir :ro grants
  illusory) -> sandbox hardened (home revoked, runtime-only, verified via
  flatpak info), corpus via rsync scratch copy, pinned conf (svga_s3/
  core=normal/cputype=pentium/cycles=fixed 60000/vmemsize=2/scaler=none/
  sample-accurate sb16), driver prepare/smoke/shell/game, watch skeleton
  (census-verified watch set; PresentFlip frame trigger; 3 ghost addresses
  dropped), HEADLESS SMOKE GATE PASS first-hand (SMOKETST.TXT lists both
  EXEs). D29. Interactive half still gated: wine EXW launch + DOSBox-X
  golden-run calibration/checklist (RUNTIME.md follow-ups).
  Post-restart re-verification 17:56-18:0x (worker 1787068533):
  smoke gate re-run FIRST-HAND - PASS (rc=0, both EXEs at pinned
  sizes), sandbox posture verified via override file + flatpak
  override --show --user (!home + runtime only; note: without
  --user the CLI prints empty under env-based XDG_DATA_HOME),
  manifests OK x2 bracketing - harness stack stable across the
  4th restart of this lane.

- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
- Phase: P1 essentially complete; P2 well underway. P3 UNDERWAY (bedlam-core skeleton DONE 2026-08-18): decoders
  promoted to workspace crate engine/bedlam-assets (pure, inspect CLI output
  byte-identical, D14); MUSIC FORMATS DECODED IN RUST 2026-08-17: music.rs
  module (MRS container + full event-stream walk + RATIO_TABLE verbatim from
  EXW, MRW bank with wave ranges, byte-exact rebuilds) + decode-song CLI +
  inspect mrs dumper + corpus invariants (see RE-EXW-MUSIC.md 3b). EXW outer architecture +
  100Hz tick + game worker thread FULLY mapped (GameThread@0044dea0 = 59-byte
  trampoline -> GameMain@0041c050 = real game shell/loop; 7x5 zone/level
  structure; RNG seeds 123456/234567). RATES (D15): 100Hz service tick /
  50Hz palette fade while fading / 12.5Hz palette cycle; 004ede10 = fade
  countdown (NOT a frame gate - D13 50Hz parity claim withdrawn); sim/render
  rate UNKNOWN pending FUN_0043d00b/FUN_00440e45 bodies. Tick satellites
  fully mapped: fade engine (FadeStep/FadeSetup/SetPaletteRGB), CursorToGame
  (window->640x480), DDRAW init/shutdown chain + object slots, thread spawn
  via Watcom CRT ThreadSpawnImpl@0045204b -> real CreateThread. Names applied in BedlamWatcom project (WinMain..
  AppActivate, TickWorker.., GameThread/GoFlagSet/GameMain - see
  docs/RE-EXW-MAINLOOP.md, docs/RE-EXW-TICK.md, docs/RE-EXW-GAMETHREAD.md).
  EXD import still pending.
- CLOSED 2026-08-18 (b2-import run): B2 DOS IMPORT DONE - ghidra-lx-loader
  built from source vs our exact 12.1.2 install (zero version risk),
  installed to userSettings/Extensions; import command + 3 gotchas in
  RESEARCH-BEDLAM2-CENSUS.md sec 5 (-loader LeLoader forced; MzLoader
  otherwise claims LE first). BedlamWatcom:/BEDLAM.EXE analyzed: 671 fns,
  blocks 0x10000/0x80000-0x1304ee, entry 0x66a60, 24041 applied fixups.
  First cross-build parity fact: RNG seeds 123456/234567 identical in B2
  (FUN_0002f731 game-init) and EXW (004ede48/4c). B2 pipeline = -process
  BEDLAM.EXE -noanalysis from here on (NEVER re-import).
- CLOSED 2026-08-18 (b2 entry/tick run, 2df7664+c3b1552+9b4d119): B2
  entry chain named + TICK SOURCE FOUND + zone/mission stride located
  (census sec 6, D22). _entry@0x66a60 -> CrtInitChain@0x6b1bc (argc/argv
  g_argc@0x1280d4/g_argv@0x1280d8) -> GameInit@0x2f731 = boot + episode
  loop shell (seeds RNG 123456/234567 as code constants at 0x11ef1c/18).
  Tick = 100.01 Hz PIT INT-8 ISR (divisor 0x2e9b, DOS INT21 AH=25h vector,
  immediate EOI, drop-not-queue reentrancy): 7 counters, 12.5 Hz palette
  banks 0x90..0x97 (same as EXW), 50 Hz mouse poll+clamp vs 320x240 coords,
  play-clock divider; present = vblank double-poll 0x3da (WaitVRetrace).
  Same two-clock architecture as EXW -> D16 parity budget carries to DOS.
  Zone/mission = lookup tables (order[8]@0x81dba, zone letters@0x81dda,
  mission[27]@0x81e46; +5 when mode==2 -> MISSION{6,7} corpus files; 6
  zones x {4 regular + 2 alt}, 27 linear missions). 15 fns + 16 labels
  persisted in BedlamWatcom:/BEDLAM.EXE.
- CLOSED 2026-08-18 (miri+hash-CI run): PLAN sec 7 DETERMINISM CI GATE DONE
  (1501ab9 + 014597b). (a) Miri CLEAN on this host: rustup component add
  --toolchain nightly-x86_64-unknown-linux-gnu miri (miri 0.1.0
  771916f902 2026-08-08, on the existing nightly; rustc 1.99.0-nightly
  b07e5a086 2026-08-07), then cargo +nightly miri test -p bedlam-core =>
  41 unit + 12 determinism tests green, ZERO UB findings (111.5s + 40.9s;
  re-run with the new fixture green too). (b) Committed per-tick hash
  fixture: engine/bedlam-core/tests/hash_fixture.rs - 600-tick fixed
  integer script (seed 123456, fade window armed ticks 101..200) pins 13
  milestone StateHash values + FNV-1a chain over all 601 hashes
  (EXPECTED_CHAIN 0x760d221bec3b3b99); runs in the ordinary cargo test
  matrix => cross-OS/toolchain hash drift fails loud per tick; ignored
  print_fixture is the ONLY documented regeneration path (intentional
  hashed-state changes + FORMAT_VERSION bump). (c) ci.yml miri job:
  ubuntu-latest, dtolnay/rust-toolchain@nightly + miri component,
  cargo +nightly miri test -p bedlam-core per push/PR. Workspace now 154
  tests green (fixture +1), fmt + clippy -D warnings clean, manifests OK
  x2. Next P3: bedlam-audio mix-graph skeleton (design note first), then
  bedlam-game scene-FSM skeleton.
- Repo: github.com/roguefort-dev/bedlam-re (main). Local: ~/Documents/bedlam-re
- Autonomy: tools/nudge.sh + systemd user timer bedlam-nudge.timer (60s) + crontab
  fallback. Heartbeat: .state/heartbeat (stale > 7 min => spawn continuation run).
  Stop conditions: .state/PLAN-COMPLETE (forever) or .state/PAUSE (temporary).
  NOTE: touch heartbeat around every long shell command (Ghidra ~2min) or a
  second agent gets spawned mid-run (happened 2026-08-17, see NEXT.md run notes).
- Backups: game-data copy at ~/Backups/bedlam-re/game-data (1069 files). Original
  bin/cue on Desktop. Offsite: NOT YET (user to arrange).
- CLOSED 2026-08-17 (input-map run): EXW input/control map - scan-code
  keystore @004edc44 (arrows +0x80 remap), 12 edge latches, mouse flags
  @004dc6e4 (dbl-click dead), Up/Down=volume P=pause, Left/Right arrows
  DEAD (3-way proof), camera=cursor+drag only - docs/RE-EXW-INPUT.md.
- Known open: GameMain second hop - FUN_0043d00b (per-frame sim/render; its
  004ede10 read = fade-status, real rate mechanism unknown - D15) +
  FUN_00440e45 (zone/level manager) + divider consumers (FUN_00448ef1,
  FUN_00402b48); music chain FULLY closed incl. sub-voice start path (SubVoiceStart = SetFrequency ratio*11025 / SetVolume / SetPan / Play; table C + 0xFE loop flag + pending-restart all DEAD); .BLD/.CTG (editor-only), PAL variant
  renderers, EXD import (needs LE loader ext), goldens pipeline (P4).
  Parity budget: NO committed logic rate (D15 withdrew D13 50Hz).
  CLOSED 2026-08-17 (tick2 run): GoFlagSet caller = FUN_0041e19d; fade
  engine, cursor mapping, DDRAW init chain, thread-spawn slot all mapped
  (docs/RE-EXW-TICK.md tick2 section).
- CLOSED 2026-08-17: music format chain fully RE-d + byte-validated
  (.MRW layout, .MRS container + complete event grammar, MusicPump=song 3,
  ratio table @00454174; CONFIG.BDL = installer SB-setup record, EXW never
  reads it; RNG seeds consumed by RandA@00402975 / RandB@004029b6) - see
  docs/RE-EXW-MUSIC.md.

- CLOSED 2026-08-18 (bedlam-core run): P3 sim core skeleton DONE in
  engine/bedlam-core (f15eb60+7396491+889cbef) - D17 hybrid timing: hashed
  60Hz Sim (300Hz microstep satellites per DESIGN-RENDER sec 6) + non-hashed
  per-frame FrameState + 240Hz sub-tick SimDriver accumulator; PCG32, Q16.16
  fx, in-crate FNV-1a state hash, versioned b"BDLR" replay + b"BDLS"
  snapshot; 132 tests green, clippy clean, manifest OK. Next P3: render
  crate skeleton (design note a3ad066), Miri + cross-OS hash CI.

- CLOSED 2026-08-18 (episode-loop run, 928748d+7bfac4b+aff1ae8 + adopted dead-run
  B2EpisDump): B2 EPISODE LOOP + COUNTER VERDICTS DONE (census sec 7, D23).
  All 7 INT8 counters classified - NONE gate sim/render (2 audio bases
  0x801a6/0x80010, 2 DEAD 0x11f158/0x11f0b4, ISR phases 0x11f0c8, 100Hz
  timeout base 0x11f0c4 w/ WaitTicks100Hz, 50ms delay 0x11f0b0). Mission loop
  = present-paced VESA page flip + vblank (D16 architecture CONFIRMED on
  DOS, D23). Episode progression: linear 0..26 + per-stage-slot completed
  mask (full-mask table 0x81d9a) + stage-slot advance w/ zone-complete
  cutscene; sub = PLAYER-selected in MapRoomSelect (mission-select UI, BRF_*
  backdrops); saves = 5 x 61B records {mask,slot,linear,money,stats}.
  B2 audio = IRQ0-shared 11025 Hz PCM driver (PIT reprogram on arm; stub
  ms-clock x10 vs hi-res tick+PIT-phase) - same native rate as EXW.
  Video = VESA 0x101 640x480x8, dual pages bank {0,5} display-start {0,
  0x200}, 640-byte stride + 320x240 logical space = 2x scale. Zone letters
  dword[0]=25 = sentinel (unreachable index). 30 fns + 33 labels persisted;
  orphan stub/driver callbacks created as functions. Open residuals queued:
  27-vs-25 step accounting, LFB-vs-banked 4f02 variant, 0x200 units,
  FUN_000126c8 satellite.
- CLOSED 2026-08-18 (render+platform unit): P3 PRESENTATION SKELETONS DONE
  (ff8fb17 + d2b7fb8, D24). engine/bedlam-render = pure state->canonical
  640x480x8 Frame + 6-bit palette + FNV parity hash, fixed pass order
  (world->sprites->rows->overlays->entities), camera clamp, palette_dirty
  derivation, 12 tests. engine/bedlam-platform = pure scale/uv geometry
  (Integer default/Fit/Fill) + wgpu 27.0.1 parity pipeline (index tex per
  frame + packed palette tex on dirty + fullscreen-triangle WGSL
  palette-expand/scale, Original v<<2 default), offscreen GPU round-trip
  test that skips without an adapter, 9 tests. Workspace 153 green, clippy
  -D warnings + fmt clean. Provenance: code landed by the 03:00 worker
  whose client died transport rc=1 at 03:05 while its server session
  finished both commits (03:07, 03:17) then died before the queue rewrite;
  the 03:32 respawn verified the work (153 green incl. real GPU test,
  fmt/clippy clean, manifests OK x2) instead of redoing it and closed the
  unit. Next P3: Miri over bedlam-core + per-tick hash CI job.
- CLOSED 2026-08-18 (audio unit, triple-agent night): P3 AUDIO MIX-GRAPH
  SKELETON DONE in engine/bedlam-audio (846ebab + b684bee + 00c2260 +
  b950b44 + a8f26f8). DESIGN-AUDIO.md pinned first (mix topology voices ->
  master bus -> device; 11025 Hz native both builds; Q16 tick grid 441/4
  samples = exact; D25 linear-Q8 volume over the EXW (master*vol)/48
  product, dB curve documented not reproduced; note-off-releases-BASE
  quirk kept; audio NOT hashed per D17 b - byte-identity of the mix stream
  is the gate). Crate: hermetic integer Mixer (forbid unsafe, no floats,
  no I/O/clock), flat 20-voice pool (B2 walker) tagged (instrument, sub
  0..3) (EXW mrw 4 sub-voices), 16.16 phase step = RATIO_TABLE verbatim,
  Q8 volume x pan gains snapshotted at spawn (EXW reads master per
  SubVoiceStart only), i32 bus + symmetric clamp, S16 stereo interleaved
  out; MusicScript = absolute-tick NoteOn/NoteOff list with
  no-bedlam-assets coupling (mapping lands in bedlam-game); render
  dispatches events at exact Q16 positions chunking-invariantly.
  9 unit + 14 determinism tests (same script => byte-identical buffer
  across 1/7/64/512-frame chunkings, base-only note-off, drop-when-full,
  one-shot recycling, saturation clamp, tick-grid exactness at frame 441),
  workspace 177 green (+23), fmt + clippy -D warnings clean, miri CLEAN
  (9+12 tests, zero UB; integration suite ~292s under miri - ci.yml miri
  job extended to -p bedlam-audio, acceptable CI cost). DECISIONS D25.
  Deliverable survived a duplicate-spawn storm (three agents on item 1:
  0162 silent death, 0711 = this run, 1260 transport death mid-verify; a
  watch run contaminated then cleaned the lane and deleted the uncommitted
  test file - regenerated from the /tmp/opencode generator; boundary bugs
  it would have caught were caught by the restored suite: immediate
  one-shot free + event-on-exact-boundary ordering). Next P3: bedlam-game
  scene-FSM skeleton (LAST charter crate).

