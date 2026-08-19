import struct
d = open("game-data/BEDLAM/GAMEGFX/TITLE.SMK","rb").read()
u32 = lambda o: struct.unpack_from("<I", d, o)[0]
print("len", len(d))
print("magic", d[:4], "w", u32(4), "h", u32(8), "frames", u32(12), "rate_raw", u32(16), hex(u32(16)), u32(16)-2**32, "flags", hex(u32(20)))
print("audio_max_buffer[7]", [u32(24+4*i) for i in range(7)])
print("tree_chunk_size(packed)", u32(52))
print("tree_size[4] unpacked", [u32(56+4*i) for i in range(4)])
rates = [u32(72+4*i) for i in range(7)]
print("rate[7] packed", [hex(r) for r in rates])
for i,r in enumerate(rates):
    if r & 0x40000000:
        print(f"  track {i}: EXISTS rate={r & 0xFFFFFF}Hz dpcm={bool(r & 0x80000000)} bink={bool(r & 0x0C000000)} 16bit={bool(r & 0x20000000)} stereo={bool(r & 0x10000000)}")
print("dummy", hex(u32(100)))
n = u32(12) + (1 if u32(20) & 1 else 0)
print("total_frames incl ring", n)
ftab = [u32(104+4*i) for i in range(n)]
print("first 3 frame-size words", [hex(x) for x in ftab[:3]])
ftypes_off = 104 + 4*n
ftypes = d[ftypes_off:ftypes_off+n]
from collections import Counter
print("frame_type histogram (top)", Counter(ftypes).most_common(5))
print("keyframes", sum(1 for x in ftab if x & 1))
