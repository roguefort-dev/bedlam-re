# RESEARCH — 8street Bedlam reconstruction: formats, main loop, input, audio, deviations, state, RNG

Provenance key (all paths relative to the pinned clones):

- **[ASM]** = `/home/kato/Documents/bedlam-refs/Bedlam/ASM_sources/bedlam.asm` (commit a8622e6) — line numbers refer to this file.
- **[INC]** = `/home/kato/Documents/bedlam-refs/Bedlam/ASM_sources/bedlam_data.inc` (data/BSS symbol comments carry IDA array-size hints).
- **[CPP]** = `/home/kato/Documents/bedlam-refs/Bedlam/CPP_sources/<file>.cpp|h`.
- **[RB]** = `/home/kato/Documents/bedlam-refs/ReversedBedlam/src/`.
- **[BT]** = `/home/kato/Documents/bedlam-refs/BedlamTools/src/`.
- **[DATA]** = facts verified by direct byte inspection of `/home/kato/Documents/bedlam-re/game-data/` (read-only; scripts ran from /tmp).

Everything below is FACT unless marked **HYPOTHESIS**. Game-data was never modified.

The 8street "Bedlam" repo is a playable reconstruction of the **Windows 95 (Watcom) Bedlam executable**: ~100k lines of IDA-disassembled assembly (`bedlam.asm`, the whole game) linked against a C++/SDL2 shell that replaces DirectDraw/DirectSound/Win32 (`CPP_sources/`), plus vendored SDL2, SDL2_mixer and libsmacker-1.2.0. Note the shipped game folder contains *both* DOS (`BEDLAM.EXE` + `DOS4GW.EXE`) and Win95 (`BEDLAM.EXW`/`BEDLAM.EXD` + `SMACKW32.DLL`, `DIRECTX/`) builds; the reconstruction targets the Win95 code base (WinMain/WindowProc xrefs, `__EnterWVIDEO_` Watcom runtime stubs at [ASM] 103592).

---

## 1. Format semantics table (core deliverable)

### 1.0 How file paths are built

- `concat(base, ext)` appends `ext` to `base` into a 0x40-byte buffer ([ASM] `concat`, 40739–40806).
- `get_current_mission_path()` ([ASM] 97030–~97260) builds two globals every level:
  - `var_filename` = `EDITOR\ZONE<char>\MISSION<level>` where `<char>` = `'A' + zone` and `<level>` = `itoa(zone_level)`, **except** `game_mode==2` (deathmatch) where `<level>` = `itoa(zone_level + 5)` ([ASM] 97139–97154).
  - `var_buffer` = `EDITOR\ZONE<char>\MISSION<char>` (zone-level base name, e.g. `MISSIONA`) ([ASM] 97179+).
- All level files are opened via `open_file` ([ASM] 39879–39974) which prefixes the cwd path, tries `r+b`, retries with the configured install drive letter, then errors out (`ERROR:Could not find file '%s'` + exit) if missing.
- **Campaign layout** ([RB] `levels.cpp:51–61` `init_zone_mision_arr`, mirrored at [ASM] 98191–98229): mission list = ZONEA1, then zones B..F × levels 1..5 = **26 campaign missions** (`zone_arr[27]`, 12-byte entries {zone, level, ended}; [INC] 505761). `level_var = (zone-2)*5 + zone_level - 1`, clamped to 0..0x1A (26) ([ASM] 39170–39184). Zone 7 (ZONEG) is a special 1-mission zone with difficulty forced to 2 ([ASM] 39190–39201).
- **Deathmatch**: `game_mode==2` ⇒ mission 6/7 of a zone (MISSION6.*, MISSION7.*). These have 16-byte NME files (empty spawn lists) and their own .BIN ([ASM] 97139–97144; [DATA] NME sizes).

### 1.1 Extensions actually loaded by the runtime

| Ext | What it is / binary layout (all little-endian) | Loaded by | Contents / formulas |
|---|---|---|---|
| **.TOT** | Mission tile-type map. Header: 2 × int16 (`x_size`, `y_size`, signed). Then `N = x_size*y_size` tile records of **16 bytes = 8 × int16**, one word per z-level (8 levels). File size = `4 + 16*N` ([ASM] `load_tot_dat_cgr_bin_min_pad` 40884–40904; header parse duplicated in `load_mission_files_in_briefing` 96974–96989; [RB] `levels.cpp:34–42`). | `load_tot_dat_cgr_bin_min_pad` (game) / `load_mission_files_in_briefing` (briefing) → `mission_tot_ptr` | Per-z tile/object **type id** per map tile; `mission_x_line_offset[y] = y*x_size`; `mission_square = N`. Type DB in RAM is `tot_buffer` / `tot_buffer_pl2`, **0x1E bytes per tile type**, runtime-mutated (e.g. `tot_buffer[type*0x1E + z*2]`, [ASM] 16901). ZONEB: 100×100 → 160,004 B ([DATA] header `64 00 64 00`). |
| **.DAT** | Per-z-level terrain/flags grid, **1 byte per (z,y,x)**. Header: 4 bytes skipped. File = `4 + 8*N` (8 z-planes; `pos_z_ofst_in_dat[z] = z*N` computed at load, [ASM] 40918–40929, table aliases `mission_z_in_xy_offsets+4`, [INC] 553433–553435). Addressing: `dat_ptr + pos_z_ofst_in_dat[z] + x_line_offset[y] + x` ([ASM] `get_from_dat_file` 42327–42343). | same loader → `mission_dat_ptr` | Values are per-z flags/types: `0xFF` = pad/marker (written by PAD/TRT loaders); `0x66` = turret; `0x2A`/`3` checked by `get_z_pos` ([ASM] 41286–41301). At load, **any byte ≥ 0x80 is cleared to 0** for every (x,y,z) ([ASM] 40930–40978). ZONEB: 80,004 B = 4+8·10000 ([DATA]). |
| **.MAP** | Same header + same `4 + 16*N` size as .TOT in every mission ([DATA]: byte-identical header words). **Not referenced anywhere in the reconstruction** (no ".MAP" literal in [ASM]/[INC]). | — (never loaded) | HYPOTHESIS: editor-side mirror of TOT (map editor's own copy). Purpose for runtime unknown — needs EXE RE. |
| **.COL** | Same header, `4 + 16*N` bytes ([DATA] ZONEB first words `01 00 01 00 …` ≠ TOT). **Not referenced anywhere in the reconstruction.** | — (never loaded) | HYPOTHESIS: per-tile collision/damage-class table consumed by the original exe (the editor's "COLlision"); unused by 8street. Needs RE. |
| **.BLD** | "Buildings" graphics/height data; **variable size, not referenced by the reconstruction** (no ".BLD" literal). 43 files (37 mission + 6 zone-level; ZONED has no MISSIOND.BLD) ([DATA]). | — (never loaded) | HYPOTHESIS: pre-rendered building/structure data for the editor or DOS renderer. Needs RE. |
| **.CGR** | Tile graphics bank, same directory format as .BIN: int16 count (128 for all missions, [DATA]), then count × int32 **offsets relative to each directory slot** (`cgr + 2 + 4*(type-1)`), tile image at `+offset+6` (6-byte sub-header, then 32-px-row pixel data) ([ASM] 41358–41373; count check [DATA]: word0=128, file=132,354). | `load_tot_dat_cgr_bin_min_pad` → `mission_cgr_ptr` (0x20788 alloc, [ASM] 40556–40560) | Per-tile-type 32×32 (isometric) tile imagery for the current zone. |
| **.BIN** | **Image bank** (the game's sprite format). Layout (authoritative decoder [BT] `Bedlam_func.cpp:7–188` `draw_IMG_in_buffer` (rev of 0x00401E39) and [CPP] `exported_func.cpp`): `int16 image_count`; then `image_count` × int32 offset (relative to directory slot `bin+2+4*i`); image header at `bin+2+4*i+offset`: `int16 flags` (bit1 ⇒ two int16 x/y hot-spot offsets follow), `int16 width`, `int16 height`; then pixels. bit0 of flags = RLE: control int16 runs — bit15 set ⇒ skip (transparent run, count = word & 0x0FFF), bit14 set ⇒ end-of-line; else literal run of (word & 0x0FFF) bytes. Uncompressed variant = raw rows w/ optional 0-skip. A second per-tile codec (byte-RLE: bit7 = skip `(b&0x3F)+1`, bit6 = EOL; plus palette-xlat variant) in [BT] `draw_tile` 191–489. File count check ([DATA]): GENERAL.BIN word0=153, WEAPONS.BIN=70, MISSIONA.BIN=1450. | dozens of fixed-name loads | Everything graphical: UI, fonts, robots, weapons, enemies, explosion frames. **MISSION\<n\>.BIN** (level sprites, loaded only when the mission has its own, e.g. ZONEB M6, ZONED M5, ZONEE M6) and **MISSION\<zone-letter\>.BIN** (zone default, `mission_bin_ptr`, count ⇒ `mission_bin_count` [ASM] 40869–40875). **WEAPONS.BIN** loaded into `weapons_bin_ptr` (0x5208 alloc) at [ASM] 41079–41081; weapon sidebar graphics + (per alloc size) weapon data. **SINTABLE.BIN** = 512 B = 256 × int16 sine LUT → `sintable_bin_ptr` ([ASM] 40387–40389; reader `get_value_from_sintable` 42366–42375: `int16 sintable[byte & 0xFF]`; `sinus()` 42393–42436 does a 64-entry lookup with the table’s words). Other fixed loads: DROPSHIP, SPIDER, TERRA, CACO, HUMANS, SENTRY(G), BIOMEX1/3(G), GRILLA(G), DANTE, SCANNER, BLOWUP(G), SHRIKE, REAPER, SMOKE, TELEPORT, NUMBERS, FLAGS, VICERA, DEBRIS, SHIELD, ROBNUMS, TABLE, DIGITS, SMOKER, FULLFONT, MONOFONT, TINYFONT, SMLFONT, IDIOTGFX, GENERAL, BRIEF, BETWEEN, LOAD_US/LOAD_UK, DB_MAIN, DEATHM, NETMONT, NORMAL, SELMONT, SELECTOR, WEAPICON, CONLITE, SHOPFONT, SHOPLITE ([ASM] 40385–40396, 40449, 40483–40543, 41061–41139, 81380+, 87353+, 90848+, 93974+). |
| **.PAL** | Four distinct variants ([DATA] census: 52×770, 2×98, 3×256, 3×65536): **(a) 770 B** = 2-byte header + 768 B RGB (VGA 6-bit 0..63 components; RGB read at +2,+3,+4 — [ASM] 41166–41201 builds `R_pal/G_pal/B_pal` brightness-adjusted copies and 0x3F-clamped `*_pal63`). GAMEGFX\GAMEPAL.PAL is the master game palette ([ASM] 41118–41120). The 44 `EDITOR\ZONE*\MISSION*.PAL` (770 B) are **never loaded** by the reconstruction (no path builds them). **(b) 98 B** (FULLPAL.PAL, CONSPAL.PAL): small palette, 2-byte header + 32×3 RGB **HYPOTHESIS** (font palette). **(c) 256 B** (DARKPAL, DARKPALS, SELDARK): 256-entry palette-index translation table (dark variant), loaded to `var_darkpal_buffer` ([ASM] 41124–41126). **(d) 65536 B** (TXPAL1/2/3.PAL): **256 × 256-byte palette-translation tables** (fire/text effects: `al = txpal[pixel]`, [ASM] `draw_fire` 1818–1839, `sub_402A28` 4372–4396). TXPAL1 → in-game (`txpalX_bin_ptr`, [ASM] 41112–41114), TXPAL2 → briefing, TXPAL3 → mission-select. | fixed-name `load_file` calls | Palettes / palette remaps. (The user prompt’s “66332 B” figure is `ls` rounding; actual size is 65,536 B.) |
| **.TRN** | Two families of 8 files each: `GAMEGFX\PALTRAN0..7.TRN` → `paltran_trn_ptr[8]` and `GAMEGFX\MAPTRAN0..7.TRN` → second 8-pointer array; names built as `"GAMEGFX\PALTRAN"+itoa(i)+".TRN"` ([ASM] `load_paltran` 47255–47364, `load_maptran` 47370–47474). | load_paltran/load_maptran (game_level+428/+42D) | Per-zone (index 0..7 ⇒ one per zone letter A..H?) **palette transform** and **map/tile transform** tables applied per zone at level start. Internal record layout not decoded here — HYPOTHESIS: arrays of palette index remaps (zone tinting). |
| **.RAW** | **Headerless 8-bit unsigned PCM mono, 11,025 Hz** (see §9 for proof). Loaded whole-file; size = duration×11025. | `load_raw_` ([ASM] 259–285: `ebx=0x2B11` (=11025) Hz, 8 bit, 1 channel → C++ `load_raw_to_soundbufer`) for SFX; `load_speechs` → `load_speech` thunk → same `load_raw_` ([ASM] 639–642) for speech | 149 files: SOUND\SFX\*.RAW (weapons, explosions, ricochets, beeps, elevators, grunts…) and SOUND\SPEECH\SPCHnn[AB].RAW (mission briefings/debriefings). SFX set loaded per level by `load_raws` ([ASM] 81103–81186) and per room (menus, shop, briefing, map room). |
| **.MRW** | Music **waveform bank** (**VERIFIED against EXW+DATA 2026-08-17**: `FUN_0044c2cc`=mrw_load, `FUN_0044c64c`=DS CreateSoundBuffer 11025/8/mono; u16 n_inst; n_inst × 8-byte `{u32 rel_off(+2-based), u32 size}` at +2; deduplicated waveforms; max(off+size)==file size exact for all 5 files): `int16 count`; then `count` × 8-byte entries `{int32 offset, int32 size}` at +2; sample data at `file+offset`. Each chunk is submitted as 11,025 Hz / 8-bit / mono (`ebx=0x2B11`, `ecx=8`, channels=1) ([ASM] `load_mrw_to_buffers` 102374–102456, esp. 102434–102444). | `load_midi(basename, song)` → `load_mrw` ([ASM] 5842–5880) via `load_mrw_to_buffers` | The "instrument" samples for one music piece (selected by `word@0` per song index). Buffers tracked in `midis_buffers_arr`. |
| **.MRS** | Music **sequencer/score data** for the same basename: header word pair → `midi_arr[song]`, `midi_arr_pl1[song]` (chunk counts); contains per-channel chunk tables (`dword_45CD88/98/A8` per-song section pointers, `dword_45C7E0` chunk pointers, 0x28-byte and 0x50-byte per-channel state) parsed by `load_midi` ([ASM] 5564–5716) and played by the software sequencer `midi_callback`/`init_midi_vars`/`sub_4032A5`/`dsound_midi_play` ([ASM] 4593–5563, 102464+). | `load_mrs` ([ASM] 5748–5835) called from `load_midi` | 5 pairs in SOUND\MIDI\: **BRIEF, DEBRIEF, OPTIONS, SELECT, SHOP** ([ASM] 81427, 85613, 87449, 90890, 93893 — base strings `SOUND\MIDI\<NAME>` + `.MRS`/`.MRW`). This is the HMI-derived (HMIDET.386/HMIDRV.386/HMIMDRV.386 ship with the game) custom tracker format; the reconstruction replays it by triggering MRW sample chunks through DirectSound→SDL_mixer (`play_midi_chunk`/`play_sound2`). **MRS is the per-song score; MRW is that song's sample set — i.e. .MRS is per-song parameters+events, .MRW per-song waveforms.** **FULLY DECODED 2026-08-17** [EXW+DATA, byte-validated all 5 files]: [EXW] load_midi=FUN_00403642, load_mrs=FUN_00403827, load_mrw=FUN_004038c6, LoadFile=FUN_0041cc7f, MusicPump=FUN_00402bac (song slot 3 only), MrsChunkStart=FUN_004032a5, MrsNextEvent=FUN_00402e74, MrsTriggerNote=FUN_00402e46, VoiceAlloc=FUN_00402db9. Event = u16 delta (10 ms ticks) + event byte: <0x7F note (variant 0: byte=instrument; variant 1: instrument=variant+7, ratio=16.16 table @00454174[byte], note tag=byte-0x54) + volume byte (0xFF=note-off); 0x7F song-end (unused by data); 0x80-0xFD rest (unused); 0xFE/0xFF pattern restart on channel byte (chunk 1 = loop timer, its tick delay = exact song length: 331/400/1476/1600/3388 ticks). All 28 melody streams consume to the exact byte. Complete grammar + validation: docs/RE-EXW-MUSIC.md sections 2/2b. |
| **.TRT** | Turret list: `int16 num_turrets`, then `num_turrets` × 12-byte record `{int32 x, int32 y, int32 z}` (tile coords). Loader then pokes `DAT[z*N + y*x_size + x] = 0x66` and `TOT[..]=1`, sets HP `= 0xFA + level_var*0xFA/27` ([ASM] `load_turrets` 31514–31618; [DATA] ZONEA M1: `0300` + 3×12 = 38 B ✓). | `load_turrets` (called first inside `load_enemy_nme_file`) → `turrets_arr[250]` (0x20-byte stride, [INC] 435616) | Static defense turret placements; `num_turrets` global. |
| **.NME** | Enemy/human spawn script: a **sequence of sections**, each = `int16 count` followed by `count` × fixed records (10-byte or 8-byte, per section). Loader walks ≥5 sections spawning different archetypes ([ASM] `load_enemy_nme_file` 30600–31508: section loops at 30645, 30773, 30940 (8-B records), 31046 (8-B), 31164 (10-B), final humans section 31417 (8-B records, `humans_arr`, 4 humans per record)). Record fields used as words at +2/+4/+6 (+8/+A) = tile x, y, z plus per-type params; spawns = `max(1, word@+2 + difficulty)` etc.; positions jittered with `rnd3`/`rnd2`; HP scales with `level_var` (e.g. `0xAF + level_var*0xAF/27`, `0xC8 + …/27`, `0x5DC + level_var*0x5DC/27`). | `load_enemy_nme_file` → `enemy_arr[350]` (0x7E-byte stride, [INC] 448529), `num_enemys`, `humans_arr[128]` (0x1E stride, [INC] 492549), `num_humans` | Empty file = 16 zero bytes (deathmatch maps, [DATA] ZONEB M6/M7). Exact per-section archetype mapping is evident in code (types 1..7 + humans) but each section’s record field semantics are only partially named here — HYPOTHESIS on individual field meanings. |
| **.POS** | Fixed **2000 records × 16 bytes** (= 32,000 B exactly for every file, [DATA]). Record = 4 × int32: `[0]`, `[4]` (validated ≠ −1), `[8]`, `[0Ch]` = −1 ⇒ end. `num_pos_non_minus1` = index of first record with `[0Ch]==−1` (+1) ([ASM] `load_pos_bdg_files` 36692–36735). | `load_pos_bdg_files` → `mission_pos_ptr` (0x9C40 alloc) | "Positions": map-room/objective/badge anchor coordinates (used with BDG icons; records with `[0Ch] != -1` link to BDG building ids at `[0Ch]` → `dword_4DEDFA[...]`, [ASM] 36885–36894). |
| **.PTH** | 2 bytes, zero, in **every** mission ([DATA]). Never referenced by name in [ASM]. | — | Placeholder for "paths" (editor patrol paths?) — always empty in shipped data. HYPOTHESIS: unused stub. |
| **.PAD** | Dropship/teleport pad list: **999 records × 6 bytes** = `{int16 x, int16 y, int16 z}` (all files exactly 5,994 B, [DATA]); parser stops early at `x == 0xFFFF` (max 999). Record i read into an 8-byte RAM slot (`pad_ptr[i*8]`), active flag word written at +0 ([ASM] `load_tot_dat_cgr_bin_min_pad` 40981–41037). Marks `DAT[z*N + y*x_size + x] = 0xFF`. | same loader → `pad_ptr` (0x1F38 buffer) | Teleport/dropship landing pads; `set_map_variablers` reads `pad_ptr[0]>>16` etc. as teleport targets ([ASM] 53241–53267). |
| **.MRK** | **12 records × 16 bytes** = 192 B (all files, [DATA]). On-disk record: 4 bytes ignored + three int16→dword values; in memory kept as 12-byte marker `{x, y, z}` at `mrk_buffer[i*12]`; loader maps robot start `pos_x = (v<<13)+0xF00`, `pos_y = ((v@+4)<<13)+0xF00`, `pos_z = (v@+8)<<5 - 1` ([ASM] `load_markers_mrk_file` 17984–18305, mapping at 18279–18296). | `load_markers_mrk_file` → `mrk_buffer` (0x90), robot arrays | The 12 possible **player robot spawn markers** (robot structs are 0xA8 B; `robot_move_to_x_tile[12]`, [INC] 84039). Also sets `robots_available` per zone (1–3, [ASM] 18046–18065). |
| **.BDG** | "Badge"/building overlay bank: up to **0x11A (282) records**; each record begins with an int16 flag — if flag ≠ 1 only those 2 bytes are consumed (record inactive); if == 1: words w,h,d + dword + word + dword + 5×4 words (0x16..0x35) then **three image blobs of `2*w*h*d` bytes each** into a big scratch buffer (offsets kept in `dword_4DEE30/34/38/3C` arrays); RAM record stride 0x4E ([ASM] 36737–36868). | `load_pos_bdg_files` → `mission_bdg_ptr` array | Per-building multi-layer graphics used by the map room / damageable structures (`damage_map_structure_maybe`). |
| **.DAT (game root)** | — | — | (covered above; only EDITOR\ZONE*\MISSIONn.DAT exists) |
| **.CTG** | 16,384 B (all 44 files). Never referenced by name in [ASM]/[INC]/[RB]. [DATA] MISSION1.CTG begins `01 00 00 00 | 1F 00 00 00 | 45 00 00 00 | 03 …` — sparse int32 entries; zone-level MISSIONA.CTG mostly zero. | — (never loaded) | HYPOTHESIS: per-tile-type **cost/score** table (cf. MISSIONn.TXT "Full Score Codes for all buildings"), consumed by original exe/editor. Needs RE. |
| **.LNK / .LNG** | 16,384 B = 8,192 × int16 per-image **link/remap table**; begins with identity ramp 0,1,2,3… ([DATA]). Loaded per level: `language_option==1` (German) ⇒ `MISSION<zone-letter>.LNG`, else `.LNK` ([ASM] 40855–40868) → `mission_lnk_ptr`. | `load_tot_dat_cgr_bin_min_pad` | Language-dependent tile-graphics remap applied at draw time (`tot_buffer[tile] = mission_lnk[tile]`, [ASM] 9469–9471 inside `draw_all`). i.e. .LNG files exist so German releases can swap tile art; the pair are the same format. |
| **.MIN** | Mission **minimap colors**: exactly `16 × image_count` bytes where `image_count` = word0 of the zone's MISSION\<letter\>.BIN (verified all 7 zones: A 23200/16=1450=BIN count, B 1872, C 1743, D 1450, E 1455, F 989, G 1872 — [DATA] + [ASM] `draw_map_tiles` 4487–4518 which blits 4×4 pixels per entry through an `xlat` table). | `load_tot_dat_cgr_bin_min_pad` → `mission_min_ptr` | 4×4-pixel minimap block per tile image (map-room rendering). |
| **.TXT** | 37 `MISSIONn.TXT`, 409 B (or 1,649 B) ([DATA]). Not referenced by the reconstruction. Content = designer notes: "Full Score Codes for all buildings … CODE-NUMBER - SCORE-VALUE …". | — (never loaded) | Editor documentation per mission. **CRITIC.TXT** (0 bytes at root) likewise unused. |
| **.RST** | Single file `GAMEGFX\STATE.RST`, 491 B, plain text starting `[edit-]\r\nscreen=132 60\r\ntoggles=1 0 0 1 0 0\r\nsrch=\r\nsrc=\r\nrpl=\r\nfile=c:\dev\mayhem\pc\gamegfx\state.rst 1 1 1 7\r\n[brief]…` ([DATA]). Not referenced by the reconstruction. | — | **Editor state file** — and a leak of the original project path: `c:\dev\mayhem\pc\…` (internal codename "mayhem"). |
| **.LNG (root LANGUAGE.\*)** | — see .LNK above for the EDITOR ones; the root `LANGUAGE.ENG/GER/FRE/ITL/SPA/DCH` are ~69–79 KB **CRLF text databases**: `[SECTION_ID]` blocks whose value is a `[` … `]` delimited multi-line string; parser `get_string_from_ID` scans for `[`+ID+match ([ASM] 51166–51242; [DATA]: LANGUAGE.ENG = 69,485 B, 842 `[`-sections). Selected via `language_option` 0..5 = ENG/GER/SPA/FRE/ITL/DCH ([ASM] 38909–38955; [CPP] `options.cpp:150–202`). | `game_core1` → `LANGUAGE_file_ptr` (0x13C68 alloc) | ALL UI/briefing/debrief/shop strings (e.g. `MENU_ITEMS`, `WARNINGS`, `OVERVIEW_<zone><level>` with atoi-parsed icon coordinates, [ASM] 86889–86968). |
| **.BDL** | Three kinds: **(1) `SAVES\SAVED.BDL` = 900 B = 5 slots × 180 B** ([CPP] `save.cpp:7–21`; slot layout from [ASM] `save_game` 97561–97735): `+0x00` 8-B name; `+0x08` dword completed-missions bitmask; `+0x0C` word zone; `+0x0E` dword hiscores; `+0x12` dword money; `+0x16` word difficulty; `+0x18` 7×7 words weapon inventory (from `dword_4DE662`/`robot_weap_ammo`/`robot_weap_price`, stride 0x62/player); `+0x7A` 2×7 words misc weapons (stride 0x1C/player); `+0x96` 3×5 words shop stock (bouncy/flame/rocket). **(2) `SAVES\HISCORE.BDL` = 120 B = 10 × {dword score, 8-B name}** ([ASM] `load_hiscores` 97768–97826, `set_hiscores` 98304–98403). **(3) `SAVES\OPTIONS.BDL` = 41 B packed struct** `OPTIONS_BDL` ([CPP] `options.h:8–21`): `int32 backbuffer, actionpan, language, cd_audio; char playername[8]; int32 volume, code_no_title, midi, sound; char installdrive;` — read by `read_options_bdl_file` ([ASM] 52402–52465) through C++ getters. The repo also writes **BEDLAM.LOG** (debug log writer at [ASM] ~53080–53113). The root `CONFIG.BDL` (61 B, shipped) and root `OPTIONS.BDL` (41 B) are **not written/read by the reconstruction** (it only uses `SAVES\*`); root OPTIONS.BDL matches the 41-B struct ([DATA]). | `load_saves`/`save_game`/`load_hiscores`/`set_hiscores`/`Options` ctor | Save-game / options persistence. Root CONFIG.BDL = sound-card setup record, never read by EXW (answered, see docs/RE-EXW-MUSIC.md §3). |
| **.SMK** | Standard Rad Game Tools Smacker videos (libsmacker-1.2.0 vendored). Played by `play_smack_`/`Smack` class ([CPP] `smk.cpp`; frame pacing from `smk_info_all` us-per-frame; audio track decoded and pushed through `SOUND_SYSTEM.add_raw` at the file's own rate/bitrate/channels, [CPP] `smk.cpp:150–166`). Name building: briefing plays `GAMEGFX\BRF_<zone-letter><level>.SMK` + `.BIN` subtitle images ([ASM] 85493–85544); logos `GTLOG_US/UK`, `LOGO_US/UK` ([ASM] 39038–39059); `END.SMK`, `ZONEDONE.SMK`, `GAMEOVER.SMK`, `TITLE.SMK`, `SHOP.SMK`, `BRF_DROP.SMK` ([ASM] 39310–39380, 98409+). | `play_smack_`, `play_level_smack` | 35 videos ([DATA]). `cinematics_is_enable()` in the reconstruction keys off existence of `GAMEGFX/BRF_DROP.SMK` (Win95-vs-DOS data detection, [CPP] `options.cpp:217–224`). |
| **.WAV** | 7 root files `BEDLAM02..08.WAV` = **44,100 Hz, 16-bit, stereo PCM, ~206–225 s each** ([DATA], parsed with python `wave`). The **in-game music (CDDA image)**. Original: CD audio tracks played via `cd_audio_*` (all such calls are commented out in the reconstruction: [ASM] 38893, 39213, ~98902; `ingame_music.cpp` `play_wav_music`/`stop_wav_music` are empty stubs, [CPP] `ingame_music.cpp`). | (original: cd_audio_init / play_cd_audio — disabled here) | Soundtrack during levels. Track 1 = data track (hence numbering from 02). |
| support files | `DOS4GW.EXE` (DOS/4GW extender), `HMIDET.386`, `HMIDRV.386`, `HMIMDRV.386` (HMI sound drivers), `SMACKW32.DLL`, `BEDLAM.EXE`/`BEDLAM.EXW`/`BEDLAM.EXD`, `DIRECTX\BEDLAM0..2.EXE`, `AUTORUN.INF`, `*.ICO`, `LAUNCH*.ICO` | — | Runtime/OS plumbing, not data. |

**Extensions present in game-data but with no loader anywhere in the three repos:** `.MAP .COL .BLD .CTG .PTH .TXT .RST` (mission), `MISSIONn.PAL` (the 44 editor 770-B palettes), `CONFIG.BDL`, `CRITIC.TXT` — verified by grepping every `".EXT"` literal in [ASM]/[INC] (full list of extension literals found: `.MRS .MRW .MRK .NME .TRT .POS .BDG .TOT .DAT .CGR .BIN .MIN .LNG .LNK .PAD .TRN .SMK`, [INC] 13812–14453). These are the prime EXW/EXD reverse-engineering targets (see OPEN QUESTIONS).

---

## 2. Main loop and timing

Startup → loop ([CPP] `main.cpp:11–48`):

1. Verify game files exist (`GAMEGFX/GAMEPAL.PAL`, `EDITOR/ZONEA/MISSIONA.BIN`) — else error 404.
2. `GAME_WINDOW.init()` (SDL2 window, logical original resolution **640×480**, `ORIGINAL_GAME_WIDTH/HEIGHT` [CPP] `bedlam_draw.h:6–7`; sidebar 160 px, map 480 px, `bedlam_draw.h:9,15`).
3. `WINDOW_CURSOR.init()`, `SOUND_SYSTEM.init()`.
4. **`GAME_TIMER.init(9)`** — comment: *"timer changed to 9 for more smoothly performance"* ([CPP] `main.cpp:35–36`).
5. Call ASM `game_core1()`.

`game_core1` ([ASM] 38813–39516) is the original top-level flow, run on the game thread:

- init RNG (`rnd_seed1=123456 (0x1E240)`, `rnd_seed2=234567`, [ASM] 38851–38859), read options, black palette, `allocs_and_load_files` (memory + GENERAL/SINTABLE/fonts, [ASM] 40286–40414), load LANGUAGE file by `language_option` switch ([ASM] 38909–38955), optionally load speech (`load_speechs`), play GTLOG/LOGO smacks, then a **room state machine** loop: `init_zone_mision_arr` → `main_menu` → `briefing_room` → `shop_room` → `game_level` → `show_mission_statistic` / `map_room`, dispatched on `room_exit_code1/2` ([ASM] 39062–39243 and following).

**Timer model.** The original DOS/Win95 game ran a **100 Hz periodic timer** (`timer_update` → `increment_timers`, [ASM] 4535–4551): every tick increments `timer1`, `pallete_timer`, `WAITING_TIMER`, `timer3`, `GAME_UPDATE_TIMER`, calls `level_clock`, and every other tick animates the palette. `level_clock` ([ASM] 4557–4585) increments `level_time_ms` and wraps at **100 → 1 second**, i.e. it assumes 100 ticks/s. In the reconstruction the whole ISR is replaced by `SDL_AddTimer(9 ms, sdl_timer_callback)` ([CPP] `sdl_timer.cpp:30–97]): the callback does `WAITING_TIMER++; GAME_UPDATE_TIMER++; midi_callback(); level_clock(); mouse_update();` + palette animation + pushes a `SDL_USEREVENT` (palette apply) and every 8th tick a cursor-animation event. Note 9 ms ⇒ ~111 Hz, so the level clock and all tick-based timers run ~11% fast vs. the original 100 Hz — a deviation (see §5).

**Frame pacing.** The in-game main loop ([ASM] `game_level`, frame body between `loc_447E6A` [98943] and the wait at `loc_448730` [99697–99699]) runs the full simulation+render (enemys, humans, explosions, fire, robots, shoots, elevators, `draw_all`, `draw_map_and_game_screen_`, scanner, …) then busy-waits:

```
loc_448730:  cmp GAME_UPDATE_TIMER, 5 ; jl loc_448730 ; jmp next frame
```

⇒ **5 timer ticks per frame = 50 ms = 20 FPS** logic+render at the original 100 Hz. There is **no vsync/PIT hardware wait** in the reconstruction — pacing is purely the SDL timer counter; `unlock_surface_and_wait(n)` ([CPP] `bedlam_draw.cpp:51–60`) spins on `WAITING_TIMER` for n ticks and is used for pauses/cutscenes (0x20 = 32 ticks palette fades, 0x19 = 25 ticks pause screens, 0xC8 = 2 s, 0x7D0 = 20 s attract loops, [ASM] 82572–83351 etc.). `dos_sleep`/`sleep` only appear in teardown paths ([ASM] 102721, 44470). No `/NOSYNC`-style flag exists anywhere in these sources (nothing similar greps in [ASM]/[INC]/[CPP]) — if the original had such a switch it is not preserved here.

**SMK pacing** uses per-file microseconds/frame via libsmacker + an SDL frame timer ([CPP] `smk.cpp:124–146`).

---

## 3. Input

**Architecture:** original = Win32 `WindowProc` → `keyboard()` / `mouse_buttons()` (xrefs at [ASM] 38653, 38753). Reconstruction = SDL event pump `SDL_events()` ([CPP] `sdl_event.cpp:10–24]) → same ASM entry points.

**Keyboard state:** `PRESSED_KEY_ARR[257]` bytes of raw **PC/AT set-1 scancodes** (array + latch flags exported to ASM, [CPP] `keyboard.h:7–22`). The scancode table (0-based order = set-1) is `Scancode` enum [CPP] `keyboard.h:25–110]: ESC=1, '1'..'0'=2..11, …, P=0x19, M=0x32, SPACE=0x39, CTRL=0x1D, SHIFT=0x2A/0x36, F1..F10=0x3B..0x44, keypad cluster 0x47..0x53. SDL keycode→scancode map at [CPP] `keyboard.cpp:25–111]. `keyboard()` ([ASM] 38653–38746) maps VK 0x48/0x50/0x4B/0x4D (arrows) to +0x80 ("extended" codes 0xC8/0xD0/0xCB/0xCD) and latches `KEY_ESC, KEY_F1..F3, KEY_1..KEY_7, KEY_P, KEY_M`.

**Original bindings (facts from usage sites):**

| Key | Action | Evidence |
|---|---|---|
| ESC | menu / abort dialogs | `KEY_ESC` latched [ASM] 38676–38678; consumed everywhere in menus |
| F1 / F2 / F3 | select robot 1 / 2 / 3 (same as clicking the sidebar icons at x=0x1E7/0x219/0x24B) | [ASM] 18470–18595 (`sidebar_control`) |
| 1..7 | toggle weapon 1..7 on the active robot (`weapon_array` bitmask in `active_weapon_arr+2`) | [ASM] 18582–18636 |
| P | pause (loop waits 0x19 ticks until released) | [ASM] 99630–99644 |
| M **or** SPACE | toggle tactical map (`map_active`) | [ASM] 38735–38742 (latch), 18442–18455 |
| Arrow Up / Down | master volume +5 / −5 (0..100), autorepeat after `volume_timer ≥ 18` ticks; original codes 0xC8/0xD0, reconstruction switched to 0x48/0x50 (numpad) — original lines commented out | [ASM] 98947–99015 (comments at 98948, 98990) |
| typing | name entry via `get_pressed_keycode`/`get_symbol_from_keycode` (shift-aware via scancodes 0x2A/0x36) | [ASM] 43836–43887 |

**Mouse:** `mouse_buttons()` ([ASM] 38753–38806) maintains `mouse_buttons_state` bits 0 (left) / 1 (right) from (down,up) event pairs; SDL side `bedlam_mouse_buttons` + `mouse_update()` ([CPP] `mouse.cpp:32–81, 110–129]) update `CURSOR_POS_X/Y`, click coords, auto crosshair/cursor icon in-game. Cursor coords are scaled from window to 640×480 logical space ([CPP] `mouse.cpp:88–103]). Gameplay itself is mouse-driven (`mouse_l_click` = select/order, `mouse_r_click` = fire/target, [ASM] 16291, 23082). **There is no re-bindable controls data file** — controls are hard-coded; the only "control mapping data" is the SDL keycode→scancode table in `keyboard.cpp` (a reconstruction artifact).

Reconstruction-only additions: PRINTSCREEN = screenshot ([CPP] `sdl_event.cpp:107–110`), keypad +/- = viewport zoom ([CPP] `sdl_event.cpp:111–118`).

---

## 4. Audio

**Sample rate of .RAW — 11,025 Hz, 8-bit, mono.** Proofs in §9.

**Mixer (reconstruction):** `Sound::init()` opens SDL_mixer at **44,100 Hz, AUDIO_S16SYS, 2 channels, 128-sample buffer** and allocates channels = 160 chunks × 6 voices ([CPP] `sdl_sound.cpp:29–69]; `MIX_CHANNELS` redefined to 10 at line 13–14). Each loaded RAW becomes a `Mix_Chunk` (WAV header synthesized around the raw bytes). 3D-ish attenuation/panning is computed from screen-space distance (`get_volume`, `get_balance`, [CPP] `sdl_sound.cpp:288–322], mirroring [ASM] 81193+). `play_sound2` converts the original's 16.16 fixed-point rate/volume/balance into 44,100 Hz / 0..128 / 0..255 ([CPP] `sdl_sound.cpp:376–382]) and resamples the chunk on the fly.

**Original architecture visible in the ASM:** DirectSound wrappers `dsound_buf_is_stopped_`, `dsound_stop1`, `dsound_midi_play`, `dsound_is_playing`, `dsound_release_unused`, `duplicate_sound_buffer` (the duplication calls are commented out — replaced by channel groups) ([ASM] 291–376, 102464–102606; commented code at 271–279, 408–421). Sound data flows: file → `load_file` → `load_raw_to_soundbufer_(ptr, size, 11025, 8, 1)` → C++ chunk registry; channel index = `chunk_index*6 + voice`.

**Music (.MRW/.MRS):** described in §1.1. The sequencer (`midi_callback` runs from the 100 Hz timer tick — in the reconstruction from the 9 ms SDL timer) advances 4 tracks ([ASM] 4609–4620 loops channels 3..n), per (track, channel) state tables (`dword_45B036/45B03A`, 0x28/0x50-byte strides), reads events, and triggers MRW instrument buffers via `play_midi_chunk` with per-note resample/volume/balance. So: **.MRS = score + per-song parameters (tempo/effects), .MRW = that song's 11,025 Hz 8-bit mono instrument waveforms; the pair is loaded together by `load_midi(basename, song)`** ([ASM] 5564–5716). Five basenames: OPTIONS (main menu), SELECT, BRIEF, SHOP, DEBRIEF.

**Speech:** `load_speechs` ([ASM] 39980–40285) loads ~100 SPCH*.RAW via the same `load_raw_` (11,025 Hz 8-bit mono); gated by `speechs_enable` (in the reconstruction always on, [CPP] `options.cpp:211–215]). `play_speech` at [ASM] 49973.

**CDDA:** the original had full CD-audio support (`cd_audio_enable/cd_audio_loaded`, `cd_audio_init`, `play_cd_audio`, `stop_cd_audio`, `j_stop_cd_audio`); **all actual calls are commented out** in the reconstruction ([ASM] 38891–38894, 39209–39213, 410-comment at ~98902, 52950–52958). The 7 root WAVs (44.1 kHz stereo) are the ripped tracks; `ingame_music.cpp` stubs `play_wav_music`/`stop_wav_music`. Volume keys also scale CDDA volume separately (`volume*0x147>>7`, [ASM] 98975–98985).

**SMK audio:** decoded by libsmacker, pushed as chunks at the file's own rate/bitrate/channels ([CPP] `smk.cpp:150–166]) — deviation from streamed Smacker audio.

---

## 5. Deviations ledger (reconstruction vs. original)

| Area | Change | Evidence |
|---|---|---|
| Platform | Port to SDL2 (Win + Linux); DirectDraw/DirectSound/Win32 replaced by C++ shell | [CPP] whole tree; README "List of changes" |
| Audio output | Mixer at **44.1 kHz stereo 16-bit** ("44.1kHz Mixer. More channels. Sounds a little better than original 11kHz") | README; [CPP] `sdl_sound.cpp:39` |
| Audio voices | Channel-group scheme (6 voices per chunk, 160 chunks) replaces DirectSound buffer duplication (duplication code commented out) | [CPP] `sdl_sound.cpp:44–55`; [ASM] 271–279, 408–421 |
| Timer | SDL timer at **9 ms** instead of the original 100 Hz (10 ms) tick — "timer changed to 9 for more smoothly performance"; makes all tick-based logic/clocks ≈11% faster than original | [CPP] `main.cpp:35–36`; [ASM] 4535–4585 (level_clock assumes 100 ticks/s) |
| Volume keys | Arrow scancodes 0xC8/0xD0 replaced by 0x48/0x50 (original lines left commented) | [ASM] 98948, 98990 |
| CDDA | CD-audio playback disabled (calls commented; `play_wav_music` stub) — in-game music silent | [ASM] 38893, 39213, ~98902; [CPP] `ingame_music.cpp` |
| Speech option | Always enabled: "OPTION.BDL does not contain speechs then is always on" | [CPP] `options.cpp:211–215` |
| Cinematics detection | `cinematics_is_enable()` = existence of `GAMEGFX/BRF_DROP.SMK` (data-set sniffing instead of config) | [CPP] `options.cpp:217–224` |
| Misc/region flag | `get_misc()` = existence of `GAMEGFX/LOGO_UK.SMK` ("seems is censorship selector or UK/US selector") instead of config bit | [CPP] `options.cpp:236–246` |
| Language | Auto-detect via SDL locales (en/de/es/fr/it/nl), default OPTIONS.BDL created if missing | [CPP] `options.cpp:125–202` |
| Video | Arbitrary window resolutions with scaling table (640×480 … 2560×1440), aspect-corrected menus, linear filtering hint | [CPP] `options.cpp:14–42`, `sdl_window.cpp:52–120` |
| New features | Screenshot on PRINTSCREEN; viewport zoom on keypad +/-; debug console pause (`_DEBUG`) | [CPP] `sdl_event.cpp:107–118`, `main.cpp:13–17` |
| Options file | Reads/writes `SAVES/OPTIONS.BDL` 41-B struct; original root `CONFIG.BDL`/`OPTIONS.BDL` untouched | [CPP] `options.h:47`, `save.cpp:10–11` |
| Bug fixes | "Fix some bugs"; "All known crashes has been fixed" (specific fixes not itemized in code comments) | README lines 16–22 |
| Networking | Multiplayer UI/serialization present (`networking`, `network_in_briefing`, `packet_operation`, `send_game_who`) but **no transport API is imported** — single-player only per README | [ASM] 100447–102360; `bedlam.inc` extern list; README |
| Memory | Original 8 MB DOS memory scheme preserved as bump allocator (`find_next_free_mem`, 0x53EC60 arena = 5,000,000 B) | [ASM] 40286–40738, error strings 40316–40327 |
| Save files | Path changed to `SAVES/` subdir (README: chmod needed) | [CPP] `save.cpp:10–11`; README line 51 |

---

## 6. Global state inventory (differential-testing instrumentation points)

All are DGROUP symbols in [INC]; sizes from IDA array comments, alloc sites, or C++.

| Symbol | Size / stride | Contents |
|---|---|---|
| `robot_arr`, `robot_pos_x/y/z`, `robot_exist`, `active_weapon_arr`, `weapon_array`, `shield_sprite`, `robot_temperature`, … | 12 robots × 0xA8 (168) B; weapon sub-arrays 0x62 (98) B per player + misc 0x1C (28) B per player | Player robots: positions (8.8? fixed-point — `pos = value<<13 + 0xF00`), selected weapons bitmask, ammo, prices, temperature |
| `enemy_arr`, `xxx`, `yyy`, `dword_4CFFxx` cluster, `num_enemys` | 350 × 0x7E (126) B | Enemies: pos, type (`enemy_arr` word0 = AI type 1..7), HP (word), leg z-offsets, animation state |
| `humans_arr`, `num_humans` | 128 × 0x1E (30) B | Civilians/hostages |
| `turrets_arr`, `num_turrets` | 250 × 0x20 (32) B | Turrets: x/y/z, HP, state |
| `zone_arr` (+`dword_4DECB2/4DECB6`) | 27 × 12 B {zone, level, ended} | Campaign progress (26 used) |
| `save_data` | 900 B (5×180) | Save slots mirror |
| `hiscores` | 120 B (10×12) | Hiscore table mirror |
| `rnd_seed1`, `rnd_seed2` | 2 × dword | RNG states (see §8) |
| `mission_tot_ptr/dat_ptr/cgr_ptr/min_ptr/lnk_ptr/pos_ptr/bdg_ptr`, `mission_bin_ptr`, `mission_x_mapsize/y_mapsize/square`, `mission_bin_count` | buffers, see §1 | Loaded level data |
| `tot_buffer`, `tot_buffer_pl2`, `tile_exsist` | 0x1E B per tile type (+word/byte arrays) | Runtime tile-state DB (destroyed tiles etc.) |
| `current_money`, `money_before_level`, `current_hiscores`, `hiscores_before_level`, `difficulty`, `zone`, `zone_level`, `level_var`, `game_mode`, `robots_available`, `number_robots`, `active_robot`, `robot_offset` | dwords | Economy / mission / mode state (`game_mode`: 0=single, 1=network, 2=deathmatch) |
| `level_time_ms/_s/_min/_hours`, `level_timer_active` | dwords | Level clock (100 ticks/s) |
| `WAITING_TIMER`, `GAME_UPDATE_TIMER`, `timer1`, `pallete_timer`, `timer3`, `volume_timer`, `sidebar_update_timer`, `textbox_timer`, `blink_timer` | dwords | Timing (defined in [CPP] `sdl_timer.cpp:9–10], incremented in [ASM] 4535–4540 / [CPP] `sdl_timer.cpp:63–65`) |
| `PRESSED_KEY_ARR[257]`, `KEY_*` latches, `CURSOR_POS_X/Y`, `CURSOR_POS_LCLICK/RCLICK_X/Y`, `MOUSE_BUTTONS_STATE(1)` | [CPP] `keyboard.h:7`, `mouse.cpp:11–30` | Input state |
| `mrk_buffer` (0x90), `pad_ptr` (0x1F38), `starting_dropship` (0x150) | buffers | Spawn markers / pads / dropship schedule |
| shoot arrays (`create_shoots` / `get_empty_shoot_value` / `get_empty_en_shoot`), gibs/debris arrays (`get_empty_gibs_num`, `get_empty_debris`) | fixed pools ([ASM] 25873–25933, 36584–36660) | Projectiles + gore particles — good per-frame diff points |
| `enemy_arr` HP word, `weapon_ammo_pl2` | — | combat state |

For a replay/differential harness, the highest-value checkpoints are: `rnd_seed1/rnd_seed2`, `robot_pos_x/y/z` × 12, `enemy_arr`+`xxx/yyy`+`num_enemys`, `turrets_arr`+`num_turrets`, `tot_buffer` (map damage), `current_money`, `level_time_*`.

---

## 7. Mission data relationships (how a ZONE set loads together)

Per level, `game_level` ([ASM] 98492+, call order at lines 98584–98820):

1. `rnd_seed1 = 0x1E240` reset ([ASM] 98503).
2. LOAD_US/LOAD_UK.BIN + LOADPAL(U).PAL + FULLFONT.BIN ("loading" screen).
3. `load_raws` (SFX), `allocss` (all level buffers + `load_enemy_bins`: SPIDER/TERRA/CACO/HUMANS + SENTRY(G)/BIOMEX3(G) + zone-dependent BIOMEX1(G)/GRILLA(G)) ([ASM] 98584–98586, 40459–40685).
4. `get_current_mission_path()` then `load_tot_dat_cgr_bin_min_pad` ([ASM] 98746) loads, in order: **MISSION\<n\>.TOT → MISSION\<n\>.DAT → MISSION\<letter\>.CGR → MISSION\<letter or n\>.BIN → MISSION\<letter\>.MIN → MISSION\<letter\>.LNG (German) / .LNK (else)**; parses TOT header, DAT z-offsets, then **MISSION\<n\>.PAD**.
   - Note the split: per-mission files are TOT/DAT/PAD (+NME/TRT/MRK/POS/BDG below); per-zone shared files are CGR/MIN/LNK/LNG and (when the mission has none) BIN. Missions with unique art carry their own MISSION\<n\>.BIN (ZONEB M6, ZONED M5, ZONEE M6) — loaded because `var_buffer`-based name is used first, and mission-specific BIN only overrides via the same call? (see OPEN QUESTIONS #3).
5. `load_bins` ([ASM] 98747): all fixed GAMEGFX sprite banks + GAMEPAL.PAL + TXPAL1.PAL + DARKPAL.PAL (full list in §1.1).
6. `load_paltran` / `load_maptran` ([ASM] 98748–98749): PALTRAN0..7 + MAPTRAN0..7 (per-zone transforms).
7. `set_map_variablers` ([ASM] 98750; body 53120–60448) — giant hard-coded per-zone/per-level **mission script** (teleports, scripted events), switch on `zone`.
8. `clear_scanner`, then **`load_markers_mrk_file`** (98752: .MRK → robot spawns), **`load_enemy_nme_file`** (98753: first `.TRT` internally, then `.NME` → enemies/humans), `sub_412896`/`sub_412B6` (zero robot/turret aux arrays).
9. `load_pos_bdg_files` (98758: **.POS** then **.BDG**).
10. `set_map_variablers2` (zone pads/text script), `init_platforms`, `init_tiles` (builds tile render cache from TOT/CGR/LNK), `dirty_water`, `sub_44889A`, `load_dropship_bin` (DROPSHIP.BIN), `sub_41F867` (+`sub_439C20`), `elevators_platforms`, `sub_42568E`, then 9 more `load_raw_` (BEAMIN, THROW, PEXPLODE, BIOFIRE, CACODETH, SQUAWK, GRUNT1–3).
11. `load_evakuate_coord` ([ASM] 52363) reads evacuation coordinates (from mission script data/LANGUAGE), `play_level_smack` optionally plays the BRF movie, then the 20 FPS loop.

**37 missions vs deathmatch:** campaign = ZONEA1 + ZONEB..F × 1..5 + ZONEG1 = 26 (matches `zone_arr` 26 entries + level_var cap). Numbered sets 6/7 in zones B–F (10 sets) exist for **deathmatch** (`game_mode==2` ⇒ `zone_level+5`); they have empty NME (16 B) and (some) their own BIN. ZONEA M1 and ZONEG M1 also ship standalone. Total TOT/NME/… count = 37 ([DATA] census).

**Briefing flow:** `briefing_room` loads TOT/BIN/DAT for the mission (`load_mission_files_in_briefing`, [ASM] 96951–97023), plays `BRF_<zone><level>.SMK` + `.BIN`, reads `OVERVIEW_<zone><level>` text from LANGUAGE, and objective icon coords parsed via `atoi` ([ASM] 86889–86968).

---

## 8. RNG

Two independent 32-bit generators, byte-identical code, different state:

- `rnd2` — state `rnd_seed1`; `rnd1` — state `rnd_seed2` ([ASM] `rnd2` 4291–4311, `rnd1` 4317–4337).
- Algorithm (16-bit-register form): load hi/lo words of the 32-bit state; shuffle the top bytes down (`dl=ah; ah=al; al=bh; bh=bl; bl=0`) then `rcr dl,1; rcr ax,1; rcr bx,1` (a 32-bit rotate-right-through-carry across dl:ax:bx), add back the original state (16-bit `add`/`adc` pairs), then add the 32-bit constant **0x361962E9** (25321 low word, 13849 high word). New state = that; return value in `eax` (whole 32-bit state). Semantically: `state = ror33ish(state) + state + 0x361962E9` (carry chain makes it a 33-bit rotate; exact bit semantics derivable from the five instructions — reproduce instruction-for-instruction rather than "simplifying").
- `rnd3(max)` = `(rnd2() & 0x7FFF) / ((0x8000/max)-1)`, clamped to `max-1` ([ASM] 42516–42542); `rnd(max)` = same over `rnd1` ([ASM] 42548–42563).
- **Seeding:** `game_core1` sets `rnd_seed1 = 123456 (0x1E240)` and `rnd_seed2 = 234567 (0x39487)` at process start ([ASM] 38851–38859). **`game_level` re-seeds `rnd_seed1 = 0x1E240` at every level start** ([ASM] 98503) — no time-based reseeding anywhere. `rnd_seed2` is never reseeded ⇒ deterministic across a session. Excellent for differential testing: identical input ⇒ identical `rnd` sequences per level.

---

## 9. Original mixer rate claim — exact proof lines for 11,025 Hz 8-bit mono .RAW

Primary (C++ WAV wrapping, [CPP] `wav.cpp:36–54`):

```cpp
int WAV::load_from_path(const std::string &path)
{
    int ret_val = 0;
    const File file(path);
    const std::string file_extension = to_lower(file.get_extension());
    if (file_extension == ".raw")
    {
        load_from_mem(file.get_ptr(), file.get_size(), 11025, 8, 1);   // <-- 11025 Hz, 8 bit, 1 channel
```

plus the default arguments `[CPP] wav.h:38–40` (`samplerate = 11025, bits_per_sample = 8, num_channels = 1`) and `[CPP] sdl_sound.h:17–18` (`add_raw(..., int samplerate = 11025, int bitrate = 8, int num_channels = 1)`).

Secondary (original ASM call sites, hexadecimal rate 0x2B11 = 11025):

- SFX & speech loader ([ASM] `load_raw_` 259–270): `mov ecx, 8 ; size/bit` … `mov ebx, 2B11h ; discretization` … `push 1 ; channels` … `call load_raw_to_soundbufer_`.
- Music instrument loader ([ASM] `load_mrw_to_buffers` 102434–102442): `mov edx, [edi+edx*8] / [edi+edx*8+4] ; offset/filesize` … `push 1 ; channels / mov ecx, 8 ; bit / mov ebx, 2B11h ; discretization` … `call load_raw_to_soundbufer_`.

Also: README line 21 — "Improved Audio (44.1kHz Mixer. More channels. Sounds a little better than original 11kHz)".

---

## OPEN QUESTIONS (not answered by these sources — need binary RE of BEDLAM.EXW / BEDLAM.EXD)

1. **.MAP / .COL exact semantics** — same header/size family as .TOT (16 B/tile) but never loaded by the Win95 exe reconstruction. Are they DOS-exe-only (editor) or does the original runtime use them (e.g. .COL = per-tile collision/damage class)? Needs EXW strings/xrefs.
2. **.BLD and .CTG layouts** — .BLD (buildings) and .CTG (hypothesis: per-tile-type score/cost, cf. MISSIONn.TXT score table) have no loader in any of the three repos. Who reads them — editor EXE only?
3. **MISSION\<n\>.BIN override rule** — `load_tot_dat_cgr_bin_min_pad` always builds the BIN name from `var_buffer` (zone letter). How do ZONEB M6 / ZONED M5 / ZONEE M6 get their own MISSION6.BIN? (Possibly `get_current_mission_path` also overwrites `var_buffer` for those missions, or a second path exists elsewhere in the 4,000-line `game_level`.) Verify against EXW.
4. **.MRS event encoding** - **ANSWERED 2026-08-17 [EXW+DATA]**: full event grammar decoded from the EXW sequencer chain (MusicPump FUN_00402bac / MrsNextEvent FUN_00402e74 / MrsChunkStart FUN_004032a5 / MrsTriggerNote FUN_00402e46) and byte-validated against all 5 shipped files (28/28 chunk streams parse to the exact byte; chunk-1 loop timing == song length exactly; instrument ids within MRW n_inst; note bytes 0x4F-0x5E with 16.16 ratio table @00454174). See docs/RE-EXW-MUSIC.md sections 2/2b for the complete opcode table.
5. **.TRN internal structure** — PALTRANn/MAPTRANn record layout (only the 8-file indexing is proven).
6. **Editor-only files' producers** — .PTH (always 2 zero bytes), .RST (`c:\dev\mayhem\pc\gamegfx\state.rst`), .TXT notes, CRITIC.TXT, 44 editor MISSION\*.PAL (770 B): confirm the runtime never touches them, and whether the shipped mission editor (if any on the CD) is the only consumer.
7. **CONFIG.BDL (61 B, root)** — **ANSWERED 2026-08-17 [EXW+DATA]**: sound-card setup record (name "SOUNDBLASTER COMPATIBLE", ints incl. 5 and 0x0220 = SB IRQ5/base 220); EXW contains NO reference to it (string census; the ".BDL" literal @004597d6 is the tail of "SAVED.BDL" @004597d1). Installer/setup artifact, not game state — see docs/RE-EXW-MUSIC.md §3.
8. **DOS vs Win95 differences** — the reconstruction is Win95-derived; the DOS `BEDLAM.EXE`/`BEDLAM.EXD` (655 KB "EXD" overlay) may differ in file usage (e.g. CDDA-only music, no SMK, different .BDL handling). Our target binary (EXW/EXD) must be diffed against these findings.
9. **Deathmatch map selection** — exactly how the 10 deathmatch sets (zones B–F M6/M7) are enumerated in the UI (`networking`/`construct_menu`) — the reconstruction has networking gutted (no transport), so the original enumeration logic may be partially missing.
10. **Original timer hardware** — whether the Win95 original used timeSetEvent(10 ms) or a PIT-rate assumption of exactly 100 Hz (the 9 ms deviation note implies the original was 10 ms/100 Hz, but this is inference, not a cited source line).
11. **Fixed-point formats** — robot/enemy coordinates use `<<13` (8192) scaling and z uses `<<5` (32 px/tile); the exact fractional widths (8.19? 18.13?) should be confirmed from more arithmetic sites before freezing a reimplementation's math.
12. **`language_option` values ↔ files** — 0=ENG,1=GER,2=SPA,3=FRE,4=ITL,5=DCH is proven for LANGUAGE.\* and the GER-only `.LNG` swap; whether other languages also had LNG variants in other releases is unknown (shipped data has only 7 zone-level .LNG files).
