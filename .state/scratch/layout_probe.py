import struct, glob, os
def parse(p):
    d=open(p,'rb').read(104)
    frames=struct.unpack('<I',d[12:16])[0]
    sizes=struct.unpack('<7I',d[24:52])
    ratesA=struct.unpack('<7I',d[68:96])
    ratesB=struct.unpack('<7I',d[72:100])
    return frames,sizes,ratesA,ratesB
def tracks(rates):
    out=[]
    for i,r in enumerate(rates):
        if r & 0x40000000:
            out.append((i, r&0xFFFFFF, 16 if r&0x20000000 else 8, 2 if r&0x10000000 else 1, 'DPCM' if r&0x80000000 else 'RAW', bool(r&0x0C000000)))
    return out
okA=okB=True
for p in sorted(glob.glob('game-data/BEDLAM/GAMEGFX/*.SMK')):
    frames,sizes,rA,rB=parse(p)
    tA,tB=tracks(rA),tracks(rB)
    goodA = all(sizes[i]>0 for i,*_ in tA)
    goodB = all(sizes[i]>0 for i,*_ in tB)
    if tA or tB:
        print('%-16s frames=%5d sizes=%s A=%s B=%s goodA=%s goodB=%s' % (os.path.basename(p),frames,sizes[:3],tA,tB,goodA,goodB))
    okA = okA and goodA; okB = okB and goodB
print('LAYOUT-A all consistent:',okA,' LAYOUT-B all consistent:',okB)
