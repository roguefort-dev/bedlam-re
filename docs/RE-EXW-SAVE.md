# RE: BEDLAM.EXW — the save surface (P6 QoL `p6-save-slots` re-anchor)

Provenance: objdump-only decode from the committed
`ghidra-project/exw-text-objdump.txt` (text window 0x401000..0x453056) +
read-only string/byte probes of `game-data/BEDLAM/BEDLAM.EXW`
(VA 0x454000 = file 0x52600, the D135 anchor). No Ghidra run, no
corpus write; `MANIFEST.sha256` verified clean before AND after every
game-data read. Tag discipline: [verified] = read in the objdump/probe
this run, or already pinned in the cited owning section; [inferred] =
strong deduction; [hypothesis] = unconfirmed. Addresses are EXW VAs.

Purpose (PLAN §6 P6 QoL: "save slots + metadata + opt-in autosave"):
re-anchor the ORIGINAL save surface in one artifact before the platform
unit, so the modern slot selection / metadata presentation / autosave
policy stand on EXW facts. This file both COLLECTS the already-verified
facts (each cited to its owning section) and adds the NEW decode of the
writer side + the slot metadata text, which had never been walked.

## 1. The slot domain (collected, [verified])

- SAVED.BDL = 900 B = 5 x 180 exactly; slot stride 0xB4 (180); slot
  grammar: name 8 B @+0x00, completed-missions bitmask dword @+0x08,
  zone SIGNED word @+0x0C, hiscore/score dword @+0x0E, money dword
  @+0x12, difficulty SIGNED word @+0x16, weapon rows from +0x18
  (RE-EXW-SIM §7j.70 — the restore-arm instruction walk).
- EMPTY-slot predicate: the DWORD at +0x0C tested against zero
  (0x43c283 `je 0x43c558`); the shipped file's four "EMPTY" slots are
  exactly this shape (§7j.70 SHIPPED CORPUS).
- The 5-slot staging buffer 0x4eae58 (900 B) with the save-armed flag
  dword 0x4eae54 immediately before it (RE-EXW-TITLEMENU sec 7 table).
- Engine READ side already lands this byte-faithfully:
  `bedlam-game` save.rs (`SAVED_NAME`/`SAVED_LEN`/`SAVED_SLOTS`/
  `import_saved_slot`), import-only, bounds-checked, fuzzed — the
  PLAN §6 P5 save-compatibility criterion (D51 staging seam).

## 2. The persistence substrate — REGISTRY, not files (NEW this run, [verified])

The EXW build stores the savegame in the SAME registry hive as the
config (HKCU\Software\Mirage\Bedlam\1.00, the §7j.56 correction family:
RegCreateKeyExA FUN_0044ed40, query FUN_0044ee98, write FUN_0044ed98,
close/commit FUN_0044ed84):

- String table (read-only probe, VA range 0x4597d0..0x45984f):
  `"SAVED.BDL"` @0x4597d1 (FORMATS-MISSION cites 0x4597d6 — the string
  START is 0x4597d1; byte @0x4597d0 is 0x00), `"SAVEGAME"` @0x4597db,
  `"HISCORES"` @0x4597e4 / 0x4597f4 / 0x4597fd / 0x459849, `"Player"`
  @0x4597ed, `"EMPTY"` @0x45980f, `"SAVEGAME"` again @0x459806 /
  0x459815 / 0x45981e. `ZONE`/`\MISSION` path fragments sit at
  0x4597c2/0x4597c7 (the mission-path builders' constants).
- **FUN_00446f4f = the SAVEGAME loader** [verified]: open key
  (0x446f54); existence probe 0x44ee98 for value "SAVEGAME" with size
  0x384 = 900 (0x446f59..0x446f63); if the value EXISTS → the actual
  read FUN_00446911("SAVEGAME", 0x4eae58, 0x384) → 0x44ede4 →
  RegQueryValueExA (IAT 0x4f010c) into the slot buffer (0x447073..
  0x447087); if MISSING → initialize FIVE fresh slots at 0x4eae58:
  name "EMPTY" (0x45980f) + the whole 180-B payload zeroed
  (0x446f7a..0x447055 — the zeroing walk covers +0x08..+0xB3, i.e. the
  exists-dword too), then WRITE the image back as the value
  (0x44705d: FUN_0044ed98("SAVEGAME", 0x4eae58, 0x384)) — first-run
  creation of the five-slot store.
- **FUN_00446ebc = the HISCORES twin** [verified]: same family, value
  "HISCORES", size 0x78 = 120 (10 entries x 12 B: 8-char name + dword
  score, the hall-of-fame); on missing → ten "Player" (0x4597ed)
  zero-score entries written back (0x446ec2..0x446f2f). Called from the
  NameEntryScreen tail (0x446f4a jmp 0x43c801).
- Callers of FUN_00446f4f (exhaustive) [verified]: 0x43abe4
  (NameEntryScreen menu-1 sel 1 "Start Saved Game", RE-EXW-TITLEMENU
  sec 4), 0x445db9 (menu 3 builder), 0x446127 (menu 4 builder),
  0x4469df (the save screen, sec 3 below).
- **The `SAVED.BDL` FILE is never opened by EXW savegame code**
  [verified by call census]: the save screen builds the string
  `<runtime string @0x4de544> + "SAVED.BDL" + <8-char name>`
  (0x44694c..0x4469dd: strcpy of 0x4de544, strcat "SAVED.BDL" via
  strlen/repnz-scas append, then the 8 name chars from
  0x4e43e0+9*[0x4edb90] uppercased through FUN_00444f067) at
  [esp+0x308] — and then NEVER passes it to any call: the function's
  complete callee set is UI/palette/menu (0x445b5c, 0x44653a, 0x43a48e,
  0x425a1e, 0x425a03, 0x4258d0, 0x41cbf0, 0x42392d, 0x402965,
  0x402aaa, 0x44f067) + the registry family + FUN_00446f4f. Zero file
  API. The path build is DOS-build leftover, the §7j.56 CONFIG.BDL
  pattern repeating on the save side: the on-disk `SAVED.BDL` is the
  DOS savegame our engine imports (sec 1); EXW's own persistence is the
  registry value.

## 3. The SAVE screen — the whole writer side (NEW this run, [verified])

**FUN_004446938 (0x446938..0x446ebb) = the save screen**, called from
exactly ONE site 0x43ef3e [verified] inside the campaign-shell screen
function (the big 0x43e8xx..0x43fxxx mission/zone dispatch consumer —
zone cell 0x4edd8c / mission cell 0x4edd88 selects at 0x43ee65..0x43eedc;
the container's identity as the campaign shell is [inferred] from that
dispatch + the 0x4e8378/0x4e7ed8 0x4a0-B map buffers).

Entry gate at the call site (0x43eee1..0x43ef3e) [verified] — the
SAVE BUTTON of the campaign shell:
- `[0x4eae54] != 0` — the save-armed flag (armed by GameMain at
  0x41c67a right after a FUN_004474ef mission-completion marking;
  cleared at 0x41c425 after a FUN_0044745e restore-context call, and by
  the save screen itself on commit at 0x446e83; role [inferred]:
  "a saveable campaign session exists"),
- `[0x4eddcc] != 0` — a mouse click,
- `[0x4edb88] == 0` — SINGLE-PLAYER only (the coop/h2h variants never
  offer the save screen),
- cursor in the button region `y >= 0x1af` and `x > 0x244` (screen
  bottom-right),
- then a click SFX (handle 0x4edfc8) and `call 0x446938`.

Body [verified]: loads the current slots (FUN_00446f4f at 0x4469df),
builds menu id 4 (the SAME five-slot item list construction as menu 3,
0x445b5c(4, ...) at 0x4469f2 — this answers RE-EXW-TITLEMENU sec 8's
open question "who calls FUN_00445b5c(4)": the save screen does), then
runs the standard menu loop: the 0xdc..0x1a4 x-strip / 0x18-row
bottom-anchored hit test (0x446a9a..0x446b23), hover SFX MENU1 + click
SFX MENU2 (0x446bbb/0x446c0f), redraw through PresentCopy/0x44653a.

On click [verified]:
- sel == 5 (Cancel) → return without touching the store (0x446c2e).
- sel in 0..=4 → **the slot write** (0x446c39..0x446e77):
  `ebp := 0x4eae58 + sel*0xB4`; name := the 8-char string at
  [esp+0x40c] (built at entry from 0x4e43e0+9*[0x4edb90], uppercased;
  0x4e444c — the typed DEFAULTNAME — IS entry 12 of that table
  (0x4e43e0+9*12 = 0x4e444c) [verified arithmetic, entry-table role
  inferred]); **mask := the live completion walk** over the campaign
  completion table at 0x4decae (stride 0xC records {sub@+0, zone@+4,
  done@+8}, edx 0..0x144): for records of the CURRENT zone ([0x4edd8c])
  with done != 0, set bits 1/2/4/8/0x10 by sub 1..5 (jump table
  0x446924) — i.e. the CURRENT stage's completed-sub bitmask, exactly
  the §7j.70 MASK SEMANTICS; then the fixed grammar in order:
  +0x08 mask dword (0x446ca9), +0x0C zone word 0x4edd8c (0x446caf),
  +0x0E score 0x4dd40c (0x446cbc), +0x12 money 0x46ae70 (0x446cc7),
  +0x16 difficulty word 0x46cbf8 (0x446cd2), then the 7x7-word weapon
  rows from 0x4de664+0x62*[0x4edb90] (0x446cdf..), the 0x1C-stride
  chassis row (0x4deafc, §7j.67 boundary) and the misc dwords
  0x46cd48/0x46cd5c/0x46cd70 (0x446e2f..0x446e75) — the §7j.64/B4
  census arrays. **This write order is instruction-for-instruction the
  mirror of the §7j.70 restore read.**
- commit: clear the armed flag (0x446e83), open key, WRITE the whole
  900-B image as value "SAVEGAME" (0x446e77..0x446e98
  FUN_0044ed98("SAVEGAME", 0x4eae58, 0x384)), close (0x446e9d).

## 4. The exhaustive writer census — NO AUTOSAVE anywhere (NEW, [verified])

Every caller of the registry write FUN_0044ed98 in the whole text
(objdump census): FUN_0042540c x5 (the config keys
SOUND/SPEECH/CINEMATICS/ACTIONPAN/LANGUAGE+DEFAULTNAME — TITLEMENU
sec 4), 0x446f2a (HISCORES first-run init), **0x446e98 (the save
screen's SAVEGAME commit)**, **0x44706c (the SAVEGAME first-run
five-EMPTY init)**, 0x44763a (the HISCORES hall-of-fame update at game
over: score 0x4dd40c + name 0x4e43e0+9*idx written back), and the
typed-default helpers inside FUN_0044ede4 (0x44ee1e/0x44ee5c/0x44ee81
— write-back of a default when a config value is missing).

Consequences [verified]:
1. EXACTLY TWO writers of the SAVEGAME value exist in EXW: the save
   screen's slot commit and the first-run EMPTY initialization.
2. There is NO timer, NO mission-complete hook, NO shutdown hook and
   NO periodic path that writes the savegame. The ONLY way a real slot
   gets written is the player clicking a slot row on the save screen,
   which is reachable ONLY from the campaign-shell SAVE button, which
   exists ONLY in single-player. **The shipped game never autosaves.**
3. The load side is equally user-initiated: title menu 1 "Start Saved
   Game" → FUN_00446f4f → menu 3 (five slots by name, "EMPTY" for
   empty) → click → the restore arm 0x43c26e (§7j.70; RE-EXW-TITLEMENU
   sec 5). A mid-game "load" (adopting a session over a running one)
   does not exist — a restore happens at the title menu, before the
   episode loop starts.

## 5. The slot metadata presentation (NEW this run, [verified])

**FUN_004473cd(eax = slot index) = the save-game level text builder**
(the fun the menu builders use; TITLEMENU sec 2 had it as "int->decimal
used for save-game level text" — the actual format is NOT decimal):
- zone := dword[slot+0x0A] >> 16 — the +0x0C zone word through the
  same widened access as the empty predicate (0x4473d0..0x4473da);
- zone == 0 → return "" (0x459848);
- else build at 0x4eb5d8: `' '` + `'A' + zone - 1` (0x4473f2
  `add al,0x40` — zone 1 → 'A' .. zone 7 → 'G') + one digit '1'..'5'
  per SET bit of the mask dword [slot+0x08] (bit 0 → '1', bit 1 → '2',
  bit 2 → '3', bit 3 → '4', bit 4 → '5'; 0x447407..). Examples:
  zone 3 mask 0b10011 → `" C125"` (bits 0/1/4 = subs 1/2/5); zone 2
  mask 0b11111 → `" B12345"`.

**The menu line for a used slot** (menu 3 builder 0x445db0..0x445e37,
menu 4 identical; also the save screen's list): the slot's 8-char name
copied into the menu item string, space-padded (0x20) out to 8 chars,
then FUN_004473cd(slot)'s level text appended — i.e.
`"NAME8   C125"` (name, padded, stage letter, done-mission digits).
An empty slot shows `"EMPTY"` (0x45980f; both the first-run
initialization 0x446f84 and the menu builders use it — TITLEMENU
sec 2's menu-3 row). The item count is 6: five slots + Cancel(32)
(TITLEMENU sec 2).

So the original's WHOLE slot metadata surface is: the 8-char name,
the stage letter + completed-sub digits, and (in the slot record but
not shown on the menu line) score/money/difficulty.

## 6. Consequences for the P6 unit (bounds, no new claims)

- The SLOT COUNT and metadata domain are the original's own: five
  slots, 8-char names, the `" Z<digits>"` level text, the EMPTY
  predicate. A modern slot-selection/metadata surface should reuse the
  import (bedlam-game save.rs) and reproduce FUN_004473cd's text
  byte-for-byte for presentation.
- The autosave POLICY anchor: never default. Off is the shipped
  posture (sec 4); any opt-in autosave is a modern platform addition,
  and its honest shape mirrors the original's own save opportunities:
  single-player campaign boundaries (the armed-flag + shell-button
  gating), never mid-mission.
- The engine READ side stays as landed (import-only, byte-faithful).
  No SAVED.BDL writer is owed or allowed for parity (PLAN §6 P5: new
  saves use the new versioned format; §7j.70 ENGINE CONSEQUENCE). The
  new-format writer remains future engine work; when it lands it lands
  config-not-state per the D201 posture (a restore ADOPTS the saved
  session — the title-menu restore shape of sec 4.3 — never a mid-run
  mutation; FORMAT_VERSION and every hash pin byte-stable).

## 7. Open questions

- The campaign-shell container function of 0x43ef3e (0x43e8xx..
  0x43fxxx) is not yet fully walked (its identity is [inferred] from
  the zone/mission dispatch + map buffers); the SAVE-button gate
  conditions themselves are [verified] as listed in sec 3.
- The completion table at 0x4decae (27 records x 0xC {sub, zone,
  done}) is the live source of both the save mask and the FUN_004474ef
  restore replay; its writers (the mission-complete marking paths) are
  the §7j.70 replay's dual and stay un-walked [hypothesis: they are
  the debrief/episode-advance writers; not needed for this unit].
