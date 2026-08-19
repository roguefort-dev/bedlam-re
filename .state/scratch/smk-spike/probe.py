import struct, os
p="game-data/BEDLAM/GAMEGFX/TITLE.SMK"
d=open(p,"rb").read(104)
magic,w,h,frames,ms,flags=struct.unpack("<4sIIIIi",d[:24])
sizes=struct.unpack("<7I",d[24:52])
rates=struct.unpack("<7I",d[68:96])
print("magic",magic,"w",w,"h",h,"frames",frames,"ms",ms,"flags",hex(flags))
print("audio_sizes",sizes)
print("rates",[hex(r) for r in rates])
print("filesize",os.path.getsize(p))
