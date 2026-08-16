# Bedlam (1996) — Groundwork Findings (verified 2026-08-17)

All facts below were verified directly against the game files in `game-data/`
(md5-checked copies of `~/Desktop/Bedlam_GT_Interactive_Software_Mirage_1996/extracted/`).

## Ident
- Bedlam, GT Interactive Software / Mirage, 1996. Isometric squad/robot action game.
- Hybrid release: DOS version (VESA + HMI sound + DOS4GW) AND Win95 version (DirectX 3).
- Original CD is mixed-mode: data track (ISO, 148M, md5 9af13a2f...) + 7 CDDA audio tracks (bedlam02-08.wav, ~266M total, 44.1kHz 16-bit stereo).

## Executables (all Watcom C/C++ 10.x — "WATCOM C/C++32 Run-Time system (c) 1988-1995" strings; BEGTEXT/DGROUP PE sections)
| File | Format | Size | Role (verified) |
|---|---|---|---|
| BEDLAM.EXW | PE32 i386, 6 imports: ADVAPI32, WINMM, KERNEL32, GDI32, USER32, DSOUND, DDRAW, DPLAY, smackw32.dll | 409K (~336K code) | **The real Win95 game** (DirectDraw + DirectSound + DirectPlay + Smacker) |
| BEDLAM.EXD | DOS LE (DOS/4GW) | 655K | **The DOS game** (VESA, HMI sound) |
| BEDLAM.EXE | PE32 i386, GDI/KERNEL/USER/ADVAPI only, ~27K code | 269K | Launcher; strings reference `C:\MIRAGE\BEDLAM\DOS4GW.EXE` (chain-loads DOS version) |
| DIRECTX/BEDLAM0.EXE / 1 / 2 | PE32 i386, same imports as BEDLAM.EXE | 268,800 / 268,288 / 268,288 | Launcher build variants (diff-able — good tooling warm-up) |
| SETUP.EXE | DOS LE (DOS/4GW) | 196K | Sound card setup (HMI); leaks `c:\watcom\H\*.h` include paths |
| DOS4GW.EXE | DOS extender (stock Watcom) | 265K | Not a RE target |
| SMACKW32.DLL | RAD Smacker 32-bit player | 61K | Known format; use as spec cross-ref |

Note: Watcom default calling convention = register-based (args in EAX/EDX/ECX/EBX, not on stack).
This materially affects decompiler setup — see PLAN.md §Tools.

## Supporting runtime files
- HMIDRV.386 / HMIDET.386 / HMIMDRV.386 — HMI (Human Machine Interfaces) DOS sound system drivers.
- LANGUAGE.{ENG,FRE,GER,ITL,SPA,DCH} — all game text, INI-style `[SECTION]` format, 842 sections in ENG.
- CONFIG.BDL (ASCII config: sound card + settings), SAVED.BDL (binary player save, "PLAYER" magic), OPTIONS.BDL.
- LEFT.RAW / MIDDLE.RAW / RIGHT.RAW — SETUP.EXE speaker-test sounds.

## Data layout (game-data/BEDLAM)
- GAMEGFX/ — 124 .BIN sprite/tile banks, 60 .PAL palettes, 16 .TRN 256-byte palette translation LUTs, 35 .SMK Smacker videos (TITLE.SMK 18M, END.SMK 7.4M, BRF_* briefings, SHOP...).
- EDITOR/ZONE{A..G}/ — shipped level-editor source data, per-mission file sets:
  ZONEA=1 mission, ZONEB..F=7 each, ZONEG=1 ("MISSIONG") → **37 missions total**.
  Per-mission extensions: .BDG .BLD(building blocks?) .CGR(tile gfx) .COL(collision) .CTG .DAT .LNK .MAP(25×75 header + 30000 bytes = 25*75*16) .MRK(markers) .NME(enemy config, u16 arrays) .PAD .PAL .POS(positions) .PTH(paths) .TOT(30004 bytes) .TRT .TXT(briefing text, ASCII).
- SOUND/SFX/*.RAW — unsigned 8-bit PCM (0x80-centered), 58 files, no header (rate TBD, likely HMI 11/22kHz — confirm in RE).
- SOUND/SPEECH/*.RAW — same format, 91 files.
- SOUND/MIDI/*.MRW + *.MRS — custom music format (not standard MIDI; .MRW starts with u16 count + u32 offset directory, .MRS = parameter table). 5 songs × 2 files.
- CDDA .WAV × 7 — in-game music tracks (copied to game-data/BEDLAM/).

## Format first-pass (hexdump-verified, hypotheses to confirm in RE)
- .PAL (770B): 2-byte header + 768 bytes RGB (VGA 6-bit levels).
- TXPAL*.PAL (66K): palette-adjacent bulk tables (gradient LUTs?) — TBD.
- .TRN (256B): identity-permutation palette remap table.
- GAMEGFX .BIN (sprites): u16 count + u32 offset directory then data.
- SINTABLE.BIN (512B): 256 × u16 sine table (0, 804, 1607... amplitude ≈ 16384).
- SMK: SMK2 magic, e.g. TITLE.SMK = 640×320.
- .MAP: u16 width(25) u16 height(75) + 25*75*16 bytes tile data.
- LANGUAGE: INI-like, CRLF sections.

## Environment (host)
- Linux (CachyOS), fish shell; Rust 1.97.1 + cargo; python 3.14; gcc/clang; binutils (objdump).
- Not installed yet: Ghidra / IDA / rizin / retdec / DOSBox — see PLAN.md §Tools.

## Known quirks from README.TXT (input for the bug-fix list)
- DOS version needs UNIVBE (SciTech) VESA driver workarounds; `/NOSYNC` cmdline flag exists for cards that break vsync polling.
- SETUP warns of crashes with wrong sound settings; game speed/audio tied to fragile hardware assumptions.
