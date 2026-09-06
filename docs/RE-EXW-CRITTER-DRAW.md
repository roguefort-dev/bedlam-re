# Enemy drawing and production integration

## Dispatch and asset ownership (2026-09-07)

[verified, EXW] FUN403938 walks the critter bank in record order,
0x405343..0x40632d, presence word+0x24 gates each record. The raw
seven-entry jump table at0x40391c is selected by kind-1 at0x406305.
This is separate from the personnel/POI draw loop before0x405343.
The old RE-EXW-SIM writer-census description of0x406190 as POI is
incorrect: the dispatch table identifies it as critter kind7.

| Kind | Entry | Sprite pointer | Runtime asset |
| --- | --- | --- | --- |
| 1 | 0x405772 | 0x4edd84 | SPIDER.BIN |
| 2 | 0x405ffa | 0x4ede30 | TERRA.BIN |
| 3 | 0x405c9b | 0x4edda4 | SENTRY.BIN (SENTRYG in German) |
| 4 | 0x405961 | 0x4edda8 | BIOMEX3.BIN (BIOMEX3G in German) |
| 5/6 | 0x405353 | 0x4edda0 | BIOMEX1.BIN through zone4; GRILLA.BIN above4; German variants |
| 7 | 0x406190 | 0x4eddac | CACO.BIN |

[verified, EXW] Asset loader0x41d828..0x41d952 binds these pointers.
String VAs0x4586d6/0x4586e9/0x4586fb identify SPIDER/TERRA/CACO;
0x458733/0x45875b identify SENTRY/BIOMEX3; zone test0x41d912
chooses GRILLA (0x458783) or BIOMEX1 (0x4587ab). Language branches
precede the latter three loads. HUMANS.BIN at0x45870c instead binds
0x46cbc8 for the separate personnel bank. Dormant beam animation
uses0x46af38, not one of these body banks; its asset identity remains
to be rechecked before integration.

[verified, read-only B-2 NME] The eight section counts are
3,22,16,0,6,5,0,0. Each section's spawn count still depends on its
loader rules and difficulty; these are source record counts, not
runtime enemy counts. Thus drawing only one kind does not suffice.
Current core stage_critters accepts all eight sections, including
personnel. Production WorldAssets currently never fetches NME.

## Projection and hit records

[verified, EXW] Critter positions use x/y dwords at record+0x36/0x3a
and z at+0x3e. Kinds1/2/3/5/6/7 shift x/y right8 before subtracting
the Q5 camera. Kind4 deliberately uses raw x/y (0x405961..0x405995).
Kind2 alone shifts z right8 for drawing and right13 for the layer
(0x40603d..0x406048,0x406099..a2); others subtract raw z and use
z>>5. Do not normalize all kinds with the same fixed-point conversion.

All branches add screen x base0x110 to x-y plus the fine-camera
column. Kind5/6 use screen y base0x110; kinds2/4/7 use0x100.
Kind1/3 y-base instructions still need to be recorded explicitly.
The row offset and shake are added before enqueuing through0x40798e.
Regular screen bounds are x in0..0x23f, y in0..0x23e, exclusive;
kind3 uses the smaller0x238/0x235 bounds.

[verified, EXW] Drawing produces target hit rectangles (bank0x4787bc,
capacity120) with type=critter index+1. Thus production integration
must connect click targeting as well as sprite pixels. W2 hit box
for kinds5/6 uses width60,height64,z+32 (0x4056f1..767); other
branches use64x64, with kind1 rawz, kind3/4 z+16, kind2 z>>8,
and kind7 rawz. Dying/ballistic branches omit hit boxes as recorded
in RE-EXW-SIM's writer census. Rendering writes visibility word+0x70;
dword reads at0x4d0008 followed by SAR16 instead read the adjacent
facing word+0x72 and must not be mistaken for AI visibility gates.

## Frame-selection anchors

[verified, EXW] Kind1 (0x405807..0x40585d): if signed dir word+0x58
is not -1, frame = 5*frame_word(+0x5a) + signed countdown%5;
otherwise frame=5*frame_word. Fuse+0x7c selects draw mode0x130,
otherwise0x12c. Kind2 heading sector = (((heading+8)&255)>>4)+2,
wrapped to15. If signed frame_word>4, sprite=16+sector; otherwise
add table0x454524[frame_word]. Fuse again selects0x130/0x12c.
The five table values remain to be extracted before implementation.

[verified, EXW] Kind7 sprite=(global frame+critter index)&15,
layer=min(z>>5,7), normal/fuse draw modes0x12c/0x130
(0x406229..0x4062a6). Modes6/7 draw their body but omit the hit record
(0x4062ab..0x4062c6).

[verified, EXW] Kind3 normal selectors at0x405d81..0x405e11:
mode8 uses6*(heading>>5); modes3/10 add table0x454b20[countdown];
mode2 uses48+4*(heading>>5)+countdown; mode7 or dormant countdown<6
uses80+(heading>>5); other states default0, except newly dormant
countdown>respawn_delay-3 uses1. Dormant countdown in inclusive
6..delay-3 suppresses the normal body and enters beam logic.
Fuse overrides normal mode to0x130. The walk table and beam intervals
still need their final cross-check.

[verified, EXW] Kind4 default walking frame is5*heading+old anim.
The draw itself increments anim and resets it to0 once the incremented
value reaches5 (0x405ac0..0x405af3). This presentation-time write
must not be added twice if simulation already updates the same cell.
Mode2 uses52+4*heading+countdown; mode5 uses68+heading; mode6 chooses
that same frame for countdown>2, otherwise72+heading; mode7 and early
dormant use72+heading; newly dormant uses56. The normal body suppression
interval matches the other respawning kinds. Remaining beam/frame
and draw-mode details are still open.

[verified, EXW] Kinds5/6 share0x405353..0x405767. Sector is
((heading+8)&255)>>4. Mode5 uses128+4*sector+(anim>0);
mode7/early dormant uses131+4*sector; mode6 uses128+4*sector+
min(signed anim>>1,3). Modes8/3/10 use6*sector+anim; mode2 uses
96+2*sector+(anim&1). Newly dormant uses32. Kind6 changes normal
blend modes on some paths; beam suppression and target-box exceptions
still require the final pass. These are implementation prerequisites,
not a claim that enemy drawing or production staging is complete.
