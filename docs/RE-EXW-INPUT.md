# RE: BEDLAM.EXW - input/control map (EXW)

Provenance: Ghidra headless `-process BEDLAM.EXW -noanalysis` + postScripts
`tools/ghidra-scripts/ExwInputSinks.java` (pass A) and
`tools/ghidra-scripts/ExwInputReaders.java` (pass B; listing-text census +
refs census + auto-decompile, 17 readers), follow-ups via `DecompList.java`
(FUN_0044771c / FUN_00448ef1 / FUN_00449c94 / FUN_00420608 / FUN_0041cc7f).
Raw dumps: `ghidra-project/exw-input-sinks.txt`, `exw-input-readers{,2,3}.txt`,
logs `process-exw-input-*.log` (all gitignored). Plus a raw-image pointer
probe (python, /tmp scratch) for 0x4edc44-family addresses.

Tags: [verified] = read in decompile/listing/disasm; [inferred] = strong
deduction; [hypothesis] = plausible, unconfirmed. Addresses are EXW VAs.

## 1. Pipeline: Win32 -> game state

```
WM_KEYDOWN(0x100)        scan = hiword(lParam)&0xff  -> KeySink(scan, 0)
WM_KEYUP(0x101)          scan = hiword(lParam)&0xff  -> KeySink(scan, 1)
WM_SYSCOMMAND(0x112)     wParam 0xF100 (SC_KEYMENU)  -> KeySink(0x44, 0)  [Alt==F10]
                          wParam 0xF140              -> eaten (return 1)
WM_LBUTTONDOWN/UP/DBL(0x201/2/3) -> MouseSink(0, 0/1/2)
WM_RBUTTONDOWN/UP/DBL(0x204/5/6) -> MouseSink(1, 0/1/2)
WM_KEYDOWN F(0x46)       -> FKeyHandler (screenshot; unchanged, see TICK doc)
per 100Hz tick           -> GetCursorPos -> MousePosHandler (hw cursor) +
                            CursorToGame (scroll update FUN_00425ab9)
```
**Corrections to RE-EXW-MAINLOOP.md sec 6:** the second key message is
**WM_KEYUP = 0x101** (doc said WM_SYSKEYDOWN; 0x104 WM_SYSKEYDOWN is in an
unhandled range); the key byte is the **BIOS/OEM scan code**, not a virtual
key; Alt activation (SC_KEYMENU) is synthesized as scan 0x44 (F10) down
[all verified from WndProc listing].

## 2. KeySink @0041be05(scan, down) [verified]

```c
if (scan==0x48||scan==0x4b||scan==0x4d||scan==0x50) scan += 0x80;  // arrows
if (scan < 0x100) g_keystore[scan] = (down == 0);   // 1 = held, 0 = released
```
- **g_keystore @004edc44, 256 bytes, scan-code indexed, 1 = key currently
  held** (WM_KEYDOWN passes down=0 -> stores 1; WM_KEYUP passes down=1 -> 0).
- Arrow scan codes 0x48 Up / 0x4b Left / 0x4d Right / 0x50 Down are remapped
  to 0xc8/0xcb/0xcd/0xd0 (+0x80) before storage -> bytes at 0x4edd0c/0f/11/14.
- Edge-latch dwords set to 1 while the key is held (re-checked on every key
  event; NOT edge-triggered despite the name - they are level-sampled into
  one-shot semantics by readers clearing them):

| scan | key | latch dword |
|---|---|---|
| 0x01 | ESC | 004edb50 (a.k.a. g_input_seen) |
| 0x02..0x08 | 1..7 | 004edc18..004edc30 (step 4) |
| 0x19 | P | 004edc34 |
| 0x32 | M or 0x39 Space | 004edc08 |
| 0x3b/0x3c/0x3d | F1/F2/F3 | 004edc0c/10/14 |

## 3. MouseSink @0041bf35(button, state) [verified]

```c
if (button==0 && state==0) g_mouse_flags |= 1;   // left down
if (button==0 && state==1) g_mouse_flags &= ~1;  // left up
if (button==1 && state==0) g_mouse_flags |= 2;   // right down
if (button==1 && state==1) g_mouse_flags &= ~2;  // right up
// state==2 (double-click) matches no branch -> NO-OP [verified dead]
```
- **g_mouse_flags @004dc6e4**: bit0 = left held, bit1 = right held.
- GameGoRelease (mission start) clears it to 0.

## 4. Cursor / camera [verified, cross-ref RE-EXW-TICK.md]

- TimerCallback polls GetCursorPos at up to 100Hz (gated by active/not-paused)
  -> MousePosHandler@0044b4fc (hardware-cursor surface update).
- ScrollUpdate@00425ab9 (100Hz TickWorker satellite):
  `snapshot g_scroll_flags@004eddcc = g_mouse_flags`;
  CursorToGame@0044b428 maps cursor into 640x480 game space; +9 margin,
  clamp x [9,631] y [9,463] -> g_cursor_x/y @004eddc4/8;
  bit0 (left)  -> drag anchor @004eddf8/fc;
  bit1 (right) -> drag anchor @004ede00/04 (GameGoRelease resets anchors -1);
  any button   -> g_drag_active@004ede60 = 1.
- **Camera/scroll control = cursor position + mouse drag only.** No keyboard
  reader exists for the scroll path (see sec 6 finding).

## 5. Reader census (who consumes what) [verified]

17 reader functions hit the input globals (226 listing hits). Grouped:

- **Any-key wait family**: FUN_0041f9d1 scans codes 1..0xFE, SKIPS both
  shifts (0x2a/0x36), consumes (zeroes) the byte, returns the code;
  twin variant at 0x4207d1 (EAX return). InputReset@0041f9b5 ->
  impl @0x4207b5: memset256(g_keystore) + clear 004dc6c4.
- **Menu hotkeys**: FUN_0040d197 (all 12 latches) and FUN_0040b835 (same
  set) - F1/F2/F3, digits 1..7, M/Space/ESC dispatch per screen.
- **Name entry**: FUN_0043a5fc - AnyKeyWait -> FUN_0041fa02 (scan->char),
  Backspace scan 0xe (and 0xd3), 8-char buffer @004e444c, waits for key
  release via keystore[sc], cursor blink off g_frame_count & 0xc [verified].
- **Mission shell** FUN_0044771c (loads LOAD_UK/LOADPAL/FULLFONT, resets all
  latches at start + unpause):
  - Up/Down arrows = **music volume** -5/+5, clamp 0..100 (g_music_volume
    @004ddb2c), applied as vol>>1 via master-vol setter FUN_0044c630; repeat
    gated by counter DAT_0046ae88 < 0x12 (set 0x14 on fire) [verified];
  - P latch = **pause toggle**: shows panel, busy-waits for P again
    (`while(g_P_latch==0)`), then clears ALL latches [verified];
  - ESC latch exit path + keystore[ESC] checks throughout.
- **Mission loop** FUN_0043d00b + FUN_0043e7d4 + FUN_00440e45: ESC latch +
  g_scroll_flags (click dispatch; FUN_0043e7d4 = zone/level map select using
  completion table @004decb6 [inferred from constants]).
- FUN_0041ec81: click-select hotspot x[494..626] y[195..327] [verified
  bounds; role inferred = in-mission panel].
- GameMain / HEREIAM@00448ef1 / FUN_00449c94 / FUN_0044567c / FUN_00446938:
  ESC + click gating per screen.

## 6. Headline finding: Left/Right arrows are DEAD [verified, 3-way]

No reader exists for keystore bytes 0xcb (Left) / 0xcd (Right) - arrow bytes
0x4edd0f/0x4edd11. Proven three ways:
1. listing-text census over all instructions: zero hits for 4edd0f/4edd11
   (while 4edd0c/4edd14 Up/Down hit in the volume code);
2. raw-image probe for dword pointers to those VAs: zero (the 0x4edc44-base
   "hits" turned out to be instruction displacements: sink write, AnyKeyWait
   x2, name-entry wait - all disassembled and accounted for);
3. no GetAsyncKeyState/GetKeyState/GetKeyboardState imports (count 0 in the
   EXW image) - no alternate keyboard path exists.
Parity implication: keyboard = hotkeys + volume + pause + any-key-continue
ONLY; all gameplay pointing/scrolling is mouse. (Bedlam-2 DOS build may
differ - separate binary, separate census when imported.)

## 7. Rust-facing control model (P4)

Canonical input event stream feeding the deterministic sim:
- KeyEvent{scan: u8 (post-remap), down: bool}   -> keystore + latches
- MouseEvent{button: Left|Right, down: bool}    -> g_mouse_flags bits
  (double-click events are dropped)
- CursorPos{x,y} per 100Hz tick (game 640x480 space after CursorToGame)
Only the sinks above mutate input state; replay = feed the same ordered
event list. g_input_seen semantics: ESC sets it; screens clear it.
