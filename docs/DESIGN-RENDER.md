# DESIGN-RENDER - bedlam-render crate design note (P3; elaborates D9/D12/D16/D17)

Status: IMPLEMENTED AS SKELETON 2026-08-18 (ff8fb17 + d2b7fb8): engine/
bedlam-render (secs 1-7; 12 tests) and engine/bedlam-platform (secs 8-9
parity path; 9 tests incl. a real offscreen GPU round-trip that SKIPS,
never fails, on hosts without an adapter). Pinned in code: the Frame/
palette/parity-hash contract, VgaExpand policy, fixed pass ORDER, camera
clamp + interpolation-off golden config, palette_dirty derivation, pure
scale/uv geometry, and the wgpu upload/palette-expand/fullscreen-triangle
scaler (D20 initial target). Pass CONTENT remains stub until the P4
map/sprite/text passes land. Every RE claim below carries an address
anchor + confidence tag per PLAN sec 9; design choices are tagged
[design]. Where this note proposes something the RE has not settled, it
says so in sec 11 instead of guessing.

## 1. The contract (D9, restated mechanically)

bedlam-render is a PURE function from game state to a canonical frame:

    Frame := { indices: [u8; 640*480], palette: [Vga6; 256] }

- 8-bit indexed framebuffer, exactly 640x480 - the canonical dims of the
  original (words 00456ec6/00456ec8, consumed by CursorToGame@0044b428 and
  the F-key BMP writer biWidth/biHeight) [verified, RE-EXW-TICK.md].
- Palette entries stay 6-bit VGA (the .PAL / FadeStep representation).
- Parity, goldens, and cross-OS determinism anchor on THIS representation.
- Everything above it - scaling, letterboxing, refresh rate, interpolation -
  is presentation (bedlam-platform) and NEVER feeds back into render or sim
  (D12, PLAN sec 7). The selected wgpu GPU path changes nothing in this parity contract
  (PLAN P3 wording).

## 2. RE basis (what the original does, with anchors)

| # | Fact | Anchor | Tag |
|---|------|--------|-----|
| 1 | Software fb = 640x480x1B: mission loop MemCopy arg 0x4b000 = 307200; PresentCopy@00425a1e = SurfaceLock + 480 row copies of 640 B + Unlock | RE-EXW-PACER sec 1-2 | verified |
| 2 | Present chain: LockStaging@0044ac5c spin-until-Lock, DDFlipOrBlt@0044ad18 (fullscreen Flip vt+0x2c / windowed Blt vt+0x14), one g_frame_count@0046ae68++ per pass, no software clock => vsync-locked 60 Hz-class | D16 | verified |
| 3 | Palette files: 770 B = 2 B lead-in + 768 B of 6-bit triples; the 40 mission .PAL are byte-identical (one global palette); GAMEGFX holds 60 more .PAL + 16 .TRN 256-byte remap LUTs | FORMATS-MISSION sec 11, GROUNDWORK | verified |
| 4 | Upload: SetPaletteRGB@0044aed4 writes PALETTEENTRY {r<<2, g<<2, b<<2, flags=1} into 004ee9f4 then palette SetEntries (+0x18 on 004ee9d0), retry once; IsLost/Restore bracket | RE-EXW-TICK.md tick2 | verified |
| 5 | Palette banks: SetPaletteIndex@0041d714 (guards: idx != last @004dc9f4, non-reentrant) copies 24 rows x 0x18 B at read stride 0x20 from table base @004edd7c (+ idx*4 + 2); bank cycle 0x90..0x97 advances every 8th 100 Hz tick (12.5 Hz); scroll-region bank 0x5d vs 0 when scroll x >= 0x1e0, gated by @004edb80 | RE-EXW-TICK.md | verified structure / inferred semantics |
| 6 | Fade engine: FadeSetup@0041cbf0(target, steps) seeds 768 x 16.16 accumulators @004edc38 from the current palette; FadeStep@00425901 (50 Hz while fading) does cur += step, out = cur >> 8 into the 6-bit buffer @004edc3c, SetPaletteRGB(all 256), decrement @004ede10; GameMain uses 10 steps = 200 ms; cancel FUN_00420100 | RE-EXW-TICK.md tick2 | verified |
| 7 | Palette-dirty handshake: word 004ee9b6 set by SetPaletteRGB, re-applied then cleared by DDFlipOrBlt each present | RE-EXW-PACER sec 2 | verified |
| 8 | Entry-0 quirks: initial entry 0 = black fullscreen / white windowed; AppActivate@0044b1c0 re-uploads entries 1..254 only (SetEntries(0, 0, 0xFE, &entries[1])) | RE-EXW-TICK.md | verified |
| 9 | Composition order per mission-loop pass: full-fb copy (307200) -> AnimSprites@0043f5b1 (24 slots x 0xe) -> queued row blit FUN_00402a56 -> DrawOverlays@0043fb80 (15+15 text) -> AnimEntities@0043f68d (300 slots x 0xc) -> PresentCopy | RE-EXW-PACER sec 1 | verified |
| 10 | Camera = cursor mapped into 640x480 game coords at service rate (CursorToGame@0044b428); scroll clamped x 9..631, y 9..463 | RE-EXW-TICK.md | verified |
| 11 | Cinematics paced by _SmackWait (Smaker-internal timing), not the sim loop | D16 | verified |

## 3. Type sketch (API proposal) [design]

    // bedlam-render
    pub const CANON_W: u32 = 640;
    pub const CANON_H: u32 = 480;

    /// 6-bit VGA palette entry - the canonical representation (never
    /// expanded inside bedlam-render).
    pub type Vga6 = [u8; 3];                       // components masked 0..63

    pub struct Frame {
        pub indices: Box<[u8; 307200]>,            // 640*480 palette indices
        pub palette:  [Vga6; 256],
        pub palette_dirty: bool,                   // sec 2 fact 7 analog
    }

    pub struct RenderInput {                       // lifetimes elided here
        pub state:      &SimSnapshot,              // from bedlam-core (narrow trait)
        pub prev_state: Option<&SimSnapshot>,      // camera interpolation only
        pub alpha:      f32,                       // 0..1; presentation hint ONLY
    }

    pub fn render(input: &RenderInput) -> Frame;

Notes: render holds no clock, no I/O, no threads; it is callable from any
host frame rate and from tests unchanged. palette_dirty exists so
presentation can skip palette re-upload between frames (the 004ee9b6 role);
it is derived, not stored, in hashed state.

## 4. Palette policy: 6-bit canon, expansion at the edge [design]

The canonical palette is 6-bit everywhere inside render/core. Expansion to
8-bit happens once, in presentation, under a named policy:

- VgaExpand::Original (DEFAULT): v << 2 - exactly what SetPaletteRGB@0044aed4
  uploads [verified]. Brightest entry maps to 252, never 255, matching the
  original render.
- VgaExpand::Full: (v << 2) | (v >> 4) - full-range, for users who prefer
  true whites.

COMPATIBILITY NOTE: engine/bedlam-assets pal.rs Palette already expands via
(v << 2) | (v >> 4). That is a TOOLING representation (PNG dumps etc.) and
is fine as-is, but it is NOT the render contract; do not let render consume
it as canon. A later unit may add a Vga6 type to bedlam-assets and demote
the expanded form (out of scope here). Goldens (sec 10) hash the 6-bit canon,
so the expansion policy can never regress parity.

## 5. Ownership and the determinism boundary [design, per D17]

| Concern | Owner | Hashed in sim state? |
|---|---|---|
| sim/physics ticks (60 Hz fixed) | bedlam-core | yes |
| service satellites (100 Hz service events, 50 Hz fade, 12.5 Hz bank cycle, free-running counters) | bedlam-core (integer substeps, sec 6) | yes |
| palette bank index + fade accumulators/countdown | bedlam-core | yes |
| camera/scroll values | bedlam-core | yes |
| frame composition (this note) | bedlam-render | no (derived data) |
| interpolation alpha, vsync, refresh, window, input sampling | bedlam-platform | no (never enters core) |

Rule: anything derived from host timing (alpha, present-time frame index)
may shape the FRAME but never the STATE. Goldens capture frames at sim-tick
boundaries with interpolation off (prev_state = None), so interpolated
mid-tick frames are outside parity by construction - which is what D12/D17
intend (frame-rate-driven systems excluded from the hash).

## 6. Timing integration: the 300 Hz microstep scheduler [design, implements D17]

D17 fixes the model: 60 Hz sim accumulator; satellites as integer substeps.
Concretely:

- Each 60 Hz sim tick runs 5 microsteps of a 300 Hz service clock
  (300 = lcm(60, 100, 50, 12.5); 5 microsteps per tick).
- Service event (the TickWorker analog): global microstep counter % 3 == 0
  => 100 Hz.
- Fade step (while fading): % 6 == 0 => 50 Hz - every 2nd service event,
  matching bit0 of the original 100 Hz divider @004edbc8 [verified].
- Palette bank cycle advance: % 24 == 0 => 12.5 Hz - every 8th service
  event, matching (ctr & 7) == 0 [verified].
- Phase: the counter is zeroed at boot release, mirroring FUN_0041e19d
  zeroing divider 004edbc8 [verified]. A 10-step fade therefore lasts
  exactly 200 ms as in the original.

The microstep counter, fade accumulators, countdown, and bank index are
integer sim state => same hash at 15/60/240 Hz host (D17 determinism test).

## 7. Frame composition passes [design; pass ORDER is fact 9, verified]

    pass 0  world layer      map tiles + static scenery (POS objects)
    pass 1  AnimSprites      24-slot sprite animator analog
    pass 2  row blits        queued dirty-row updates (FUN_00402a56 analog)
    pass 3  DrawOverlays     15+15 text overlays
    pass 4  AnimEntities     300-slot entity animator analog
    (present)

- Keep the original pass order: occlusion semantics are parity-relevant
  (text over entities over sprites).
- Dirty-row optimization (pass 2) is permitted ONLY if output-identical to
  the full recomposition; the original full-fb copy (fact 9) is the
  correctness reference, not a requirement to copy bytes we cannot see.
- Camera application: passes read world/sprite positions offset by the
  camera that came in with the snapshot (optionally interpolated, sec 5).
  Sprites stay grid-quantized - interpolation touches camera/scroll ONLY
  (D12); a sub-pixel blitter stays an off-by-default option (sec 9).

## 8. Presentation contract (bedlam-platform side) [design, per D9/D12]

Input: (&Frame, PresentMode). Presentation:

1. Converts palette to RGBA once per palette_dirty (004ee9b6 analog) under
   the sec 4 policy; re-upload skips otherwise.
2. Scales the FULL 640x480 source rect per mode: integer nearest-neighbor +
   4:3 pillarbox (default), fit (letterbox), fill (crop), smooth (linear).
3. Presents vsync-locked at any refresh or uncapped (D12); frame pacing at
   240 Hz is a CI-manual game-feel proxy; input-to-present <= 1 original
   frame (PLAN P6).
4. Smacker: decode native frames (bedlam-assets smk), scale in presentation;
   NOT composited into the indexed fb. Bit depth + palette sharing of movie
   frames is an open question (sec 11) - the contract just requires movies
   to bypass the indexed path.
5. The 1996 desktop-palette dance (AppActivate SYSPAL_NOSTATIC handling,
   windowed white entry 0, SetEntries skipping entry 0) is NOT replicated -
   T3 free divergence, recorded here for archaeology only.

## 9. wgpu backend and enhanced mode (D20; enhanced features OFF by default)

- Backend: wgpu (Vulkan/DX12/Metal selected by wgpu; no raw Vulkan surface in engine crates).
- Parity mode: upload index texture + palette, expand/sample on GPU, scale the
  full canonical 640x480 frame with nearest/integer default or selected filter.
- Enhanced mode: native-output-resolution world/UI passes are allowed
  incrementally; always non-parity and UI-flagged. It shares sim/assets with
  parity mode and must never feed resolution or GPU timing into sim state.
- Extended widescreen viewport (shows more map = gameplay change) remains an
  additional explicit option, separate from merely rendering at high DPI.
- Sprite sub-pixel interpolation remains explicit and off by default.
- Enhanced layout targets are 16:9 and 16:10 only (D21). Author at 16:10
  with a 16:9 safe region; other ratios use fit/letterbox/pillarbox. Controls
  and gameplay-critical information never depend on the extra 16:10 height.
- AI-upscaled/generative-fill derivatives are optional external HD packs;
  repository content is limited to tooling, recipes, masks, provenance,
  manifests, and hashes.

## 10. Goldens and testing [design, per PLAN P4]

- Golden artifacts = decoded pixel bytes + palette hash, never PNG bytes.
- Parity hash = hash(indices) + hash(palette-as-6-bit) at sim-tick
  boundaries (interpolation off): resolution- and expansion-agnostic by
  construction.
- Cross-OS exactness applies to OUR renderer (same indices + palette on
  every OS); vs emulator frames it is perceptual thresholds only (T2).
- Determinism test (D17): identical state hash under 15/60/240 Hz host
  scripts; frame hashes asserted only at tick boundaries.

## 11. Open questions (each names its answer source, PLAN P2 exit style)

1. Roles of the 4 DD surfaces 004ee9bc/c0/c8/cc - already a queued cosmetic
   RE item; affects RE completeness, not this contract.
2. SetPaletteIndex table math (idx*4 pointer slots vs flat 768 B stride) -
   one more RE look when the palette-bank pass is implemented.
3. TXPAL 256x256 tables (fire/text xlat per 8street hint) - may become a
   render-pass input; semantics unverified.
4. .PAL 2-byte lead-in meaning + whether the engine applies gamma
   (FORMATS-MISSION sec 11 open item).
5. Smacker frame bit depth and palette sharing (movies vs indexed path).
6. .TRN remap LUT application point in the draw pipeline (sprite tinting?)
   - the P2c renderer pass answers it.
7. Whether FUN_00402a56 row blits imply a row-dirty model worth mirroring
   for performance parity - optional, output-identity is the gate.

## Provenance

RE facts above restate (with anchors) results from the 2026-08-17 RE runs
recorded in docs/RE-EXW-PACER.md, docs/RE-EXW-TICK.md, docs/RE-EXW-MAINLOOP.md
and DECISIONS D9/D12/D16/D17; no new RE was performed for this note. Design
sections are proposals for the implementing unit to follow or amend with a
DECISIONS entry if deviating.
