# O3 — the 8street instrumented comparator: W10 feasibility + landing note

Status: **FEASIBILITY CONFIRMED (docs-only unit, 2026-08-24)** — this note is the
§10-W10 landing study required before any rebuild work. No engine/Rust change;
nothing from the 8street repos enters this repo (PLAN §0/§1 test-only policy).
Provenance: every clone fact below is tagged with the clone path it was read
from (navigation reference only, per AGENTS); every EXW/EXD fact is anchored to
this repo's spec docs/registry. The pinned-clone digests of §1 are the
integrity anchor for anything quoted here.

The decision recording this unit is **D142** (DECISIONS.md); the drift
ledger below is the load-bearing RE artifact.

---

## 1. The pinned rebuild target

| What | Value |
|---|---|
| Channel | O3 (DESIGN-DIFFHARNESS §1; dump.rs `Channel::O3Street` = code 3) |
| Role | SECOND comparator only — three-way disagreement localizes inherited-8street error; never a porting source, never canon (D77 §2) |
| Clone | `https://github.com/8street/Bedlam` @ local `~/Documents/bedlam-refs/Bedlam` (clean tree, verified 2026-08-24) |
| Pinned commit | `a8622e663d35c00c331a88880c20abfefccdc0eb` (2025-06-12, "Merge pull request #11 from 8street/update_readme") |
| Tree hash | `f9df7045c435b061e9e25bb5b78fe9e9c7b41793` |
| `ASM_sources/bedlam.asm` sha256 | `da77a5e44382cfb1003a511cff0d8b75f325a89189710c1bb5f1e48c643697b0` |
| `ASM_sources/bedlam_data.inc` sha256 | `e26105c0084595b706c0a73c52b8c8b30ed9c862b1568a98978433467c93ffc1` |
| What it is | ~100k-line IDA disassembly of the **Win95/Watcom BEDLAM.EXW** (`bedlam.asm` + `bedlam_data.inc`) linked against a C++/SDL2 shell (`CPP_sources/`) + vendored `libsmacker-1.2.0`, `SDL2`, `SDL2_mixer` (RESEARCH-8STREET header; [verified] on the pinned tree) |
| Sibling clones (NOT the O3 target) | `Bedlam2` = the *sequel* (Bedlam 2: Absolute Bedlam) reconstruction; `ReversedBedlam` = early C++ port study; `BedlamTools` = format tools. None enter O3. |

License note (operator-relevant): the 8street repo **carries no top-level
license** — only the vendored SDL2/SDL2_mixer sub-licenses ([verified]:
`git ls-files | grep -i licen` hits only `SDL2_mixer/lib/x86/LICENSE.*`).
The disassembly is published original-derived code with no grant. That is
consistent with our existing policy: the rebuild is a local, test-only
instrument; no 8street code, build output, or derived source is ever committed
here; git carries fingerprints + this site map only.

## 2. Build toolchain (both supported paths)

**Linux (the harness path):** `linux/compile.sh` on the pinned commit —
1. `clang -m32 -c libsmacker-1.2.0/*.c` (vendored, in-tree — no fetch)
2. `clang++ -m32 -c -std=c++17 CPP_sources/*.cpp` + `pkg-config --cflags SDL2_mixer sdl2`
3. **`jwasm -elf -c -zcw ASM_sources/bedlam.asm`** — JWasm (MASM-syntax
   assembler) built from source (`github.com/JWasm/JWasm`, cmake)
4. `clang++ -m32 o/*.o -o bedlam -lstdc++fs` + SDL2/SDL_mixer libs.

Requires: i686 multilib (`libc6-dev-i386`, `gcc/g++-multilib`) and **i686
SDL2 ≥ 2.0.12 + SDL2_mixer built `-m32`** (prepare.sh builds both from
upstream source). Output: a 32-bit `bedlam` ELF binary dropped into a game
folder.

**Windows:** `vs2019/Bedlam.sln` under VS2022, x86 platform, linking the
vendored SDL2/SDL2_mixer import libs; ships `SDL2.dll`/`SDL2_mixer.dll`.

**Their CI** (`.github/workflows/linuxbuild.yml`) = prepare.sh + compile.sh on
ubuntu-latest, artifact `bedlam.linux.i386` — a ready-made recipe proving the
Linux build is reproducible on a stock runner.

### Operator gates (the answer: YES, the FIRST build is operator-gated)

- `prepare.sh` uses **sudo apt** and clones/builds JWasm + SDL2 + SDL2_mixer
  from the network → unattended agents may not run it (AGENTS: no interactive/
  sudo; network fetches outside the approved refs are not budgeted).
- Once the toolchain exists on the host, `compile.sh` itself is unattended-safe
  (no sudo, no network; the vendored libsmacker + local sources only).
- No game-data contact at build time. The BUILT binary is run against a
  **staged corpus copy** (see §7) — never `game-data/` itself: the
  reconstruction WRITES `SAVES/` and `BEDLAM.LOG` into its game folder
  (RESEARCH-8STREET §1.1 .BDG row + §5 "Save files"; [CPP] `save.cpp:10-11`,
  the log writer at [ASM] ~53080), and AGENTS bars any write there.

## 3. Memory-layout reality — how a registry row becomes an 8street read

The single most important decoded fact of this unit:

**8street references every game cell by SYMBOL NAME, not by address.** The
process image is whatever `ld` gives the ELF; the IDA names
(`dword_46AEC8`, `mouse_buttons_state`, `rnd_seed1`, …) are the only stable
handles. Consequences for O3, verified against the pinned tree:

1. **`bedlam_data.inc` is a sequential byte mirror of EXW's sections** —
   `.data` from `org 454000h` (commented org; `.data` directive at inc line
   18) and `.bss` from `org 45B000h` (`;org 45B000h` + `.data?` at line
   14720; emission end 0x4EFB60 by simulation). All watch-registry data rows
   (0x45C000..0x4EFA00 range) live in these two sections.
2. **BUT the emission is NOT uniformly address-faithful.** A drift simulation
   over the whole `.bss` (db/dw/dd/align only — a directive census proved no
   other forms exist) checked 1208 IDA-auto names and found **8 drift
   transitions**: exact (Δ0) from 0x45B018 up to the 0x4DC6CC region; then
   Δ+16 at `dword_4DC6E0` (inc line 499363 — the emission carries SEVEN
   anonymous `dd ?` between `dword_4DC6CC` and `dword_4DC6E0` where the IDA
   names imply four; a genuine .inc defect, [verified] by eye on the pinned
   file), growing to Δ+48 by 0x4DE660, then NEGATIVE jumps (−208 at
   0x4EDD5C, −1188 at 0x4EEE08, −1184 at 0x4EEE61) where the emission has
   FEWER bytes than the original layout. The game works regardless because
   nothing references the anonymous filler. **Never map an EXW address into
   the 8street image by arithmetic.**
3. **Row resolution is therefore three cases:**
   - **(a) Named `.inc` symbol** — the registry row's EXW cell has a named
     8street symbol (semantic or IDA-auto). The hook code references the
     symbol directly (the `.inc` is included in `bedlam.asm`, same module).
   - **(b) Anonymous filler** — the cell exists only as `db ?` padding in the
     emission (e.g. the command-ring cells at 0x4DD4A0+, the command count at
     0x46CBE0). The FORK adds a zero-size label at the emission position
     computed by the drift-aware simulation (the §3 item-2 ledger), cross-checked
     by the writer-xref route: our RE-EXW-SIM writer census pins which EXW
     function writes the cell; the same function exists by name in
     `bedlam.asm`; the operand it references names the 8street cell.
   - **(c) C++-shell cells** — input state lives OUTSIDE the disassembly, in
     `CPP_sources` as `extern "C"` globals (`PRESSED_KEY_ARR[257]`
     `keyboard.h:7`, `CURSOR_POS_X/Y` `mouse.cpp`, `GAME_UPDATE_TIMER`
     `sdl_timer.cpp:10`). The hook module references them directly (same
     process, C linkage).
   - **Dead-cell equivalence seam:** the EXW frame-counter cell 0x46ae68 has
     NO reference anywhere in `bedlam.asm`/`.inc` ([verified] grep) — the EXW
     Present-tail increment was replaced by the `redraw_` C++ path. The O3
     emitter numbers frames with its OWN hook counter and emits that in the
     `frame-counter` row — semantically identical (the cell only ever counted
     frames); recorded as the channel seam, never a finding (the `_e_staging`
     seam pattern).

4. **Cross-validation (the re-anchoring proof).** Simulating the emission
   positions of the semantic symbols and reconciling with this repo's
   independently-pinned EXW cells confirms the identity BEYOND the drift —
   every delta matches the drift ledger exactly:

   | 8street symbol | emission addr | EXW canon (this repo) | delta | ledger check |
   |---|---|---|---|---|
   | `current_money` (inc 79790) | 0x46AE70 | `money` row 0x46ae70 | **0** | exact region |
   | `difficulty` (84017) | 0x46CBF8 | `difficulty` row 0x46cbf8 | **0** | exact region |
   | `robots_available` (84004) | 0x46CBD8 | the W8/D89 per-player count cell 0x46cbd8 | **0** | exact region |
   | `mouse_buttons_state` (499364) | 0x4DC6F4 | `inj-mouse-buttons` 0x4dc6e4 | +0x10 | Δ+16 region ✓ |
   | `game_mode` (565545) | 0x4EDBB8 | `mode` row 0x4edb88 | +0x30 | mid-correction region |
   | `zone`/`zone_level` (565690/88) | 0x4EDCBC/B8 | `zone`/`mission` rows 0x4edd8c/88 | −0xD0 | **Δ−208 exactly** ✓ |
   | `mission_square` (565749) | 0x4EDD24 | `static-tot-volume` 0x4ede20 (= N tiles — same semantic) | −0xFC | Δ−208 region ≈ |
   | `rnd_seed1`/`rnd_seed2` (565790/91) | 0x4EDD78/7C | `rng-state-a`/`-b` 0x4ede48/4c (seed 123456/234567 — the same seeds, RESEARCH §8) | −0xD0 | **Δ−208 exactly** ✓ |
   | `sound_enable` (565799) | 0x4EDD88 | `sfx-master-gate` 0x4ede58 | −0xD0 | **Δ−208 exactly** ✓ |

   The `zone`/`zone_level`, `rnd_seed1`, and `sound_enable` rows landing at
   precisely the ledger's −208 is the strong check: the 8street symbols ARE
   our registry cells, displaced only by the .inc's filler defects. In the Δ0
   region (0x45B018 up to the 0x4DC6D0 gap) symbol+delta arithmetic is SAFE
   for case (b) labels (e.g. the 0x46CBE0 marker-override cell =
   `robots_available`+8); inside drift regions only the full ledger applies.

## 4. The hook family (frame-tail dump points → 8street code sites)

All sites below are in `bedlam.asm` of the pinned commit (line numbers =
pinned file). The re-anchoring policy per tier: every site is anchored BOTH by
clone navigation ref (below) AND by our spec docs' EXW/EXD counterpart —
mismatches become DIVERGENCES seeds, never silent.

| Hook | Site (8street) | EXW/EXD counterpart (canon) | Tiers served |
|---|---|---|---|
| **H1 frame-tail dump** | `game_level` tail wait: `loc_448730` line 99697 (`cmp GAME_UPDATE_TIMER,5 / jl / jmp loc_447E6A`) — after `redraw_` (present) and the last state writer, before the loop-back | EXW 0x425a03 PresentEnd→`g_frame_count++` / EXD 0x5a6eb (watches.toml `s0-trigger`) | T0..T3 per-frame rows |
| **H2 anchor / mission start** | loop head `loc_447E6A` line 98943 FIRST entry (after the full load sequence — RESEARCH-8STREET §7 steps 1–11 — and the post-load palette/KEY-latch init at 98810–98940) | the O1 anchor stop / E `resolve_at=anchor` (D84) — loader statics settled | TS statics + frame 0 |
| **H3 inject seams** | keystroke/cursor → case (c) C++ cells; mouse → `mouse_buttons_state`; order-target → `dword_4DD484/488/48C`; command ring → case (b) labels; pad reads → `pad_ptr` bank | D77 §3 seam table (keystore 0x4EDC44, cursor 0x4EDDC4/8, mouse 0x4DC6E4, ORDER 0x4DD484/88/8C+0x46CC30/60, COMMAND 0x4DD4A0, .PAD step-ons) | TI |
| **H4 transcript emitter** | the hook module writes **DBXCAP v1** text directly (grammar = runner.rs module docs; needs only hex + FNV-1a-64 for its own diagnostics — the chain itself is computed by `dbx-stitch`, so the C++ side stays trivial) | consumed unchanged by `dbx-stitch` → W3 dump → chain → `dbx-diff` | all |

Design consequence (H4): O3 does NOT need a C++ W3 binary encoder. It reuses
the whole downstream pipeline via the DBXCAP seam — exactly the D139/D140
pattern (O2's feed/transcript split), minus even the driver because the hook
is in-process.

Inject application ordering: the hook applies scenario writes at H1 BEFORE
the watch reads of that frame (write-then-dump, the W5/O1 ordering; DBXCAP
`frame N 1` injected flags). The scenario/plan source is the same
`dbx-plan` output already committed for O1/O2 (channel-agnostic data; the
fork parses the JSON or a pre-compiled step list — implementation freedom,
recorded in the fork, not here).

## 5. Differ intake status (what exists / what is missing)

| Piece | Status today | W10 gap |
|---|---|---|
| W3 schema channel 3 | **LANDED** — dump.rs `Channel::O3Street` encode/decode + chain (D78; channel round-trip test) | none |
| stitch (`runner::stitch`) | channel-agnostic core; anti-ghost address rule defined for O1 (`exd_addr`) and O2 (`exw_addr`) only — "Engine and O3 dumps carry no address rule (…; O3 is W10)" (runner.rs header) | add the **O3 rule = validate ids against `exw_addr`** (8street reconstructs EXW, so the O2 mirror applies — EXD-only rows reject loud) + `dbx-stitch --channel o3` (CLI currently accepts `o1|o2` only) |
| differ normalizer | `Channel::O3Street => Err(UnsupportedChannel)` (differ.rs ~1018) | the **O3 field map** = the O2 map modulo the §6 seam set (same EXW cells, same layouts); plus the seam ledger so known-divergent rows are classified `o3-seam`, not findings |
| scenarios/plans | committed for O1/O2 (`capture-plans/*.json`) | none — reuse as-is |

Both in-repo pieces are bounded single units (no engine change, diffharness
crate only) and unattended-safe.

## 6. What O3 can and cannot arbitrate (deviation consequences)

From the RESEARCH-8STREET §5 deviations ledger (all [verified] on the pinned
tree), classified for differential use:

**Comparable (frame-indexed diffs valid):**
- The 9 ms timer deviation is a **wall-clock speed** deviation only (~111 Hz
  vs 100 Hz): per-frame logic consumes TICK counts (5 ticks/frame either
  way), so per-frame dumps stay comparable; only real-time-anchored behavior
  (wall-clock input, SMK pacing) shifts.
- Core sim state (RNG — deterministic reseed `rnd_seed1=0x1E240` per level,
  §8; robots; enemies; turrets; map/mirror state; money/score) all reach the
  hook through §3's named symbols.
- SP ZONEA robot-count parity holds on O3 exactly as pinned for O1/E (D89 —
  the mode cell `game_mode` ≡ EXW 0x4EDB88 and the per-zone count
  `robots_available` ≡ 0x46CBD8 are both written in the disassembly:
  [ASM] 18053–18065 sets 1/2/3 per zone in `load_markers_mrk_file`, the
  `game_mode` writes at 81710/83624/83644 ride the title-menu flow).

**Never comparable (expected-divergence classes → `o3-seam`, never findings):**
- Config-sourced cells whose WRITER differs: 8street reads
  `SAVES/OPTIONS.BDL` + auto-detects language/cinematics/misc-flag from FILE
  EXISTENCE ([CPP] `options.cpp:125–246`) where EXW/EXD read the registry
  (HKCU\Software\Mirage\Bedlam\1.00; RE-EXW-TITLEMENU §7j.56/D128) — every
  registry-mirrored TS/config row (ACTIONPAN 0x4EDBD8 family, sound/speech/
  cinematics/language/default-name cells) diverges BY CONSTRUCTION on O3.
- `sfx-master-gate` (0x4EDE58): the cell exists (`sound_enable`) but is fed
  by OPTIONS.BDL, not the registry — an o3-seam row (E dumps constant 1 per
  D136; O3 dumps the OPTIONS.BDL value).
- Volume-key scancodes 0xC8/0xD0 → 0x48/0x50 swap ([ASM] 98948, 98990) —
  only those two input rows drift.
- Speech-always-on, CDDA-disabled (music silent), the §3 item-2 anonymous-filler
  drift regions (no semantic effect), and the reconstruction's crash fixes
  (unitemized upstream — any residual behavioral delta shows up as exactly
  the three-way disagreement O3 exists to localize).

**Role guardrail (D77):** O3 is the tiebreak COMPARATOR — when E, O1/O2, and
O3 disagree, O3's vote localizes whether an error is inherited from the
8street disassembly or introduced by our engine. EXW stays the canon of
record; 8street is never a porting source (PLAN §0/§1, AGENTS).

## 7. Artifact placement + hygiene

- **The instrumented fork lives OUTSIDE this repo** — e.g.
  `~/Documents/bedlam-refs/bedlam-o3` (fresh clone at the pinned commit + the
  hook patch). Per PLAN/§10-W10: "no code enters this repo". What enters THIS
  repo: this site map, the drift ledger, fingerprints (chains in tests /
  DECISIONS), and — at implementation time — the patch's sha256 (the patch
  file itself may live under `runtime/` (git-ignored) or the fork tree; if we
  ever want it in git it goes to the operator, since it is 8street-derived
  code).
- **Build outputs + dumps stay runtime-only**: `runtime/harness-out/o3/`
  (git-ignored), chain digests in git (D77 §4 hygiene).
- **Corpus**: O3 runs against a staged corpus COPY under `runtime/` (the
  reconstruction writes `SAVES/` + `BEDLAM.LOG` — §2). `game-data/` untouched;
  MANIFEST check brackets every staging + capture run.
- Clone reads are manifest-bracketed too (this unit ran
  `sha256sum -c MANIFEST.sha256 --quiet` before AND after; clean both sides —
  clones are outside game-data/, the bracket is belt-and-braces).

## 8. The work split + verdict

**FEASIBLE, with the rebuild itself operator-gated.** Decomposition:

1. **In-repo, unattended-safe, small:** (a) `dbx-stitch --channel o3` + the
   O3 anti-ghost rule (`exw_addr`-mirror); (b) the differ O3 field map +
   `o3-seam` classification. Two bounded diffharness units; each lands with
   its own gate test (fabricated O3 transcript → stitch → self-cross, the
   D140 smoke pattern).
2. **Outside the repo, operator-gated:** the toolchain install (sudo +
   network) + the fork + the hook patch + the first build. The pinned sites
   (§4), the row-resolution cases (§3), and the seam ledger (§6) are the
   complete spec the fork needs.
3. **Operator or unattended after (2):** rebuilds (compile.sh only) and O3
   capture runs against the staged corpus.

Sequencing note: per D77 the W10 channel is deliberately LATE — after the
operator S0/S1 live sessions arbitrate the first real findings. The in-repo
pieces (1) may land any time (they touch no live machinery); the rebuild (2)
should wait until a three-way tiebreak is actually wanted, so the pinned
commit does not rot unrebuilt. This note is the landing contract for both.
