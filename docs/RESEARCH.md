# Tooling & Ecosystem Research (2026-08-17, web-verified by research agent)
Primary sources: official repos/releases, crates.io/docs.rs. Status: verified except where marked UNCERTAIN.

## RE toolchain
- Ghidra 12.1.2 (2026-06-05) — PE32 i386 + MZ loaders built-in; scriptable; PRIMARY.
  https://github.com/NationalSecurityAgency/ghidra/releases
- yetmorecode/ghidra-lx-loader v12.0.1 (Jan 2026) — LE/LX loader (DOS/4GW, DOS32A, OS/2 LX).
  UNCERTAIN: built for Ghidra 12.0 — verify under 12.1.2. https://github.com/yetmorecode/ghidra-lx-loader
- Watcom watcall calling convention NOT in upstream Ghidra (issue #156, open since 2019):
  args EAX/EDX/EBX/ECX, >4 on stack, callee cleans (RETN n). Community cspecs in #156
  (ghidrawatcall.zip), 0xBEEEF/GhiOWat. Wart: cspec is global — Win32 cdecl/stdcall imports
  need per-function overrides. No maintained Watcom FID exists; can generate from Open Watcom CLIB.
  https://github.com/NationalSecurityAgency/ghidra/issues/156
- IDA Free 9.x: free tier now has cloud x86/x64 decompiler + local debugger; no IDAPython; non-commercial.
- RetDec: limited maintenance mode; PE+i386 yes, DOS MZ/LE no. https://github.com/avast/retdec
- Reko 0.12.3 (2026-06-16): active; PE ◼◼◼, MZ ◼◼◼, no LE/LX. https://github.com/uxmal/reko
- Binary Ninja Free 5.1: x86 decompiler, no plugins/API. rizin+rz-ghidra 0.8.0 active; r2 LE/LX = raw fallback.
- Case study (Watcom LE + Ghidra, 2026): alexbevi.com/blog/2026/03/14/reverse-engineering-a-dos-game-with-ghidra-and-codex

## Rust ecosystem (candidates; final call per docs/DECISIONS.md at P3/P4 spikes)
- SMK video: smk 0.1.0 (2026-04, pure Rust, libsmacker 1.2.0 port, bit-identical output; NEW+unproven → vendor/fork) · libsmacker/libsmacker-sys · ffmpeg-next 9.0.0 (maintenance mode) offline only.
- Presentation: softbuffer 0.4.8 (rust-windowing, 3M dl/mo) + winit 0.30.13 — default candidate;
  pixels 0.17.2 (wgpu 29) only if GPU post-processing needed.
- Audio: cpal 0.18.1 (custom mix graph) > rodio 0.22.2 for our use.
- GM MIDI: rustysynth 1.3.6 (pure Rust SF2 synth + SMF sequencer, MIT) — recommended; oxisynth 0.1.0 alternative.
- Gamepad: gilrs 0.11 (+ winit-input-map 0.6.1 glue if useful).

## Prior work — CRITICAL
- 8street/Bedlam — C++(+ASM) reconstruction of original exe; compiles; "fully playable in
  single player mode"; SDL2 + SDL2_mixer + libsmacker-1.2.0; Windows+Linux; 109 commits; 2025.
  README links author IDA database of original. Original mixer was 11 kHz (port upgraded 44.1).
  Companions: 8street/ReversedBedlam, 8street/BedlamTools (asset viewer).
  → Use as cross-reference oracle + secondary behavior oracle; NOT a porting source.
- No other prior RE/source port of Bedlam (1996) found.

## HMI / audio facts
- HMI music (MIDI-family) documented on Modding Wiki/VGMPf (HMP/HMI signatures); no public
  spec for HMI .RAW PCM (headerless; rate set at play time). For Bedlam: 11025 Hz 8-bit mono
  per 8street wav.cpp (load_from_mem(..., 11025, 8, 1)) — still confirm from EXD HMI init.
- Our .MRW/.MRS files do NOT match HMIMIDIP signatures — treat as custom; cross-ref 8street player code.
