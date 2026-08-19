import struct
d=open('game-data/BEDLAM/GAMEGFX/TITLE.SMK','rb').read(104+4*1228+16)
raws=[struct.unpack('<I',d[104+4*i:108+4*i])[0] for i in range(1228)]
bad=[(i,r) for i,r in enumerate(raws) if (r & 0xFFFF_FFFC) % 4 != 0]
flags=sorted(set(r & 3 for r in raws))
print('total',len(raws),'non-4aligned sizes:',len(bad),'low-flag-bits seen:',flags)
print('first3 raws:',[hex(r) for r in raws[:3]])
