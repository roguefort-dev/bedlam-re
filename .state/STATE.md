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

