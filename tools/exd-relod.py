#!/usr/bin/env python3
"""exd-relod — build a relocated linear image of BEDLAM.EXD and objdump .object1.

Mirrors the yetmorecode ghidra-lx-loader (LeLoader) semantics step by step so
the produced addresses are byte-compatible with the imported Ghidra program
(RE-EXD-MAP §1: object1 0x10000..0x72800, object2 0x80000..0x12583e):

  - MZ stub -> lfanew (e_lfarlc==0x40 new-exe style)
  - LE/LX header fields per yetmorecode Header.java offsets
  - object table (24 B): size, base, flags, pageTableIndex, pageCount
  - page map: LE = 4 B big-endian {page#:24, flags:8}; LX = 8 B {off:32,size:16,flags:16}
    page file offset = lfamz + dataPagesOffset + (page#-1)*pageSize
    (last object's last page = lastPageSize bytes)
  - fixup page table (pageCount+1 dwords) + fixup record table; record parse
    per FixupRecord.java (source type nibble, 0x20 source-list flag, target
    flags 0x03 type / 0x10 t32 / 0x40 obj16 / 0x04 additive / ...).
    Application: off32 := base(targetObj)+targetOffset (4 B LE), off16 the
    16-bit truncation, sel16 := selector(obj#)=obj#-1, off32s self-relative.

Usage: exd-relod.py <BEDLAM.EXD> <out-image.bin> <out-objdump.txt> [vma-objdump]
The objdump covers object1 only (the .text W1 map range).
"""
import struct
import subprocess
import sys

SRC_MASK = 0xF
S_BYTE, S_SEL16, S_P1616, S_OFF16, S_P1632, S_OFF32, S_OFF32S = 0, 2, 3, 5, 6, 7, 8
SRC_SOURCE_LIST = 0x20
T_TYPE_MASK, T_ADD, T_CHAIN, T_T32, T_ADD32, T_OBJ16, T_ORD8 = 0x3, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80
T_INTERNAL, T_IMPORT_ORD, T_IMPORT_NAME, T_ENTRY = 0, 1, 2, 3


def u8(b, o):
    return b[o]


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def main():
    if len(sys.argv) < 4:
        print(__doc__)
        return 2
    path, out_img, out_dump = sys.argv[1], sys.argv[2], sys.argv[3]
    exe = open(path, "rb").read()

    # --- MZ -> lfanew -----------------------------------------------------
    assert exe[:2] == b"MZ", "no MZ stub"
    lfarlc = u16(exe, 0x18)
    lfamz = 0
    if lfarlc == 0x40:
        lfanew = u32(exe, 0x3C)
    else:
        # old-exe style secondary header walk (not needed for BEDLAM.EXD)
        e_cp, e_cblp = u16(exe, 4), u16(exe, 2)
        sec = (e_cp - 1) * 512 + e_cblp
        while True:
            if exe[sec:sec + 2] != b"MZ":
                break
            nxt = u16(exe, sec + 0x1A)
            if nxt == 0 or nxt == 0xFFFF:
                break
            sec = nxt
        lfamz = sec
        lfanew = sec + u32(exe, sec + 0x3C)
    sig = exe[lfanew:lfanew + 2]
    is_le = sig == b"LE"
    is_lx = sig == b"LX"
    assert is_le or is_lx, f"bad LE/LX sig {sig!r} @0x{lfanew:x}"

    h = lfanew  # header-relative offsets, per yetmorecode Header.java read order
    page_size = u32(exe, h + 0x28)
    last_page = u32(exe, h + 0x2C)  # LE: bytes on last page; LX: page shift
    objtab = u32(exe, h + 0x40)
    objcnt = u32(exe, h + 0x44)
    pagemap = u32(exe, h + 0x48)
    fixpagetab = u32(exe, h + 0x68)
    fixrectab = u32(exe, h + 0x6C)
    datapages = u32(exe, h + 0x80)  # file-relative
    pagecnt = u32(exe, h + 0x14)

    objects = []
    for i in range(objcnt):
        o = h + objtab + i * 24
        objects.append(dict(
            number=i + 1,
            size=u32(exe, o),
            base=u32(exe, o + 4),
            flags=u32(exe, o + 8),
            pti=u32(exe, o + 12),
            pc=u32(exe, o + 16),
        ))
        base = objects[-1]["base"] or (i + 1) * 0x100000  # Options.getBaseAddress
        objects[-1]["mapbase"] = base

    pages = []
    for p in range(pagecnt):
        if is_le:
            data = struct.unpack_from(">I", exe, h + pagemap + p * 4)[0]
            po, fl, dsz = (data & 0xFFFFFF00) >> 8, data & 0xFF, page_size
        else:
            po = u32(exe, h + pagemap + p * 8)
            dsz = u16(exe, h + pagemap + p * 8 + 4)
            fl = u16(exe, h + pagemap + p * 8 + 6)
        pages.append(dict(no=p + 1, off=po, flags=fl, size=dsz))

    # --- fixup tables ------------------------------------------------------
    fpt = [u32(exe, h + fixpagetab + i * 4) for i in range(pagecnt + 1)]

    stats = {}

    def parse_fixup(rec_off):
        """returns (sourceType, targetFlags, sourceOffset(s), objectNumber,
        targetOffset, size) mirroring FixupRecord.java"""
        src = u8(exe, rec_off)
        tgt = u8(exe, rec_off + 1)
        size = 2
        if src & SRC_SOURCE_LIST:
            cnt = u8(exe, rec_off + 2)
            size += 1
            soff = None  # list trails; unsupported (asserted against below)
        else:
            soff = u16(exe, rec_off + 2)
            size += 2
        if tgt & T_OBJ16:
            objnum = u16(exe, rec_off + size)
            size += 2
        else:
            objnum = u8(exe, rec_off + size)
            size += 1
        toff = 0
        ttype = tgt & T_TYPE_MASK
        if ttype == T_INTERNAL:
            st = src & SRC_MASK
            if st != S_SEL16:
                if tgt & T_T32:
                    toff = u32(exe, rec_off + size)
                    size += 4
                else:
                    toff = u16(exe, rec_off + size)
                    size += 2
        else:
            # import ordinal/name/entry: skip (none in BEDLAM.EXD; asserted)
            raise AssertionError(f"non-internal fixup tgt=0x{tgt:02x}")
        if src & SRC_SOURCE_LIST:
            size += cnt * 2
            raise AssertionError("source-list fixup encountered")
        if tgt & T_ADD:
            size += 4 if tgt & T_ADD32 else 2
        return src, tgt, soff, objnum, toff, size

    # --- assemble object blocks + apply fixups ----------------------------
    top = max(o["mapbase"] + o["size"] for o in objects)
    img = bytearray(top)
    for oi, obj in enumerate(objects):
        block = bytearray(obj["size"])
        for i in range(obj["pc"]):
            pno = obj["pti"] + i  # 1-based global page number
            ent = pages[pno - 1]
            fo = lfamz + datapages + (ent["off"] - 1) * page_size
            is_last = (oi == objcnt - 1) and (i == obj["pc"] - 1)
            n = last_page if (is_le and is_last) else (ent["size"] if is_lx else page_size)
            block[i * page_size:i * page_size + n] = exe[fo:fo + n]
        img[obj["mapbase"]:obj["mapbase"] + obj["size"]] = block

    for obj in objects:
        for i in range(obj["pc"]):
            pno = obj["pti"] + i
            fb, fe = fpt[pno - 1], fpt[pno]
            cur = fb
            while cur < fe:
                src, tgt, soff, objnum, toff, size = parse_fixup(h + fixrectab + cur)
                st = src & SRC_MASK
                tgtbase = objects[objnum - 1]["mapbase"]
                addr = obj["mapbase"] + i * page_size + soff  # linear source
                stats[st] = stats.get(st, 0) + 1
                if st == S_OFF32:
                    val = tgtbase + toff
                    img[addr:addr + 4] = struct.pack("<I", val)
                elif st == S_OFF32S:
                    val = tgtbase + toff - (addr + 4)
                    img[addr:addr + 4] = struct.pack("<i", val)
                elif st == S_OFF16:
                    val = (tgtbase + toff) & 0xFFFF
                    img[addr:addr + 2] = struct.pack("<H", val)
                elif st == S_SEL16:
                    img[addr:addr + 2] = struct.pack("<H", objnum - 1)
                elif st == S_BYTE:
                    img[addr:addr + 1] = bytes([(tgtbase + toff) & 0xFF])
                elif st == S_P1632:
                    img[addr:addr + 4] = struct.pack("<I", tgtbase + toff)
                    img[addr + 4:addr + 6] = struct.pack("<H", objnum - 1)
                elif st == S_P1616:
                    img[addr:addr + 2] = struct.pack("<H", (toff if toff < 0x10000 else toff & 0xFFFF))
                    img[addr + 2:addr + 4] = struct.pack("<H", objnum - 1)
                cur += size

    open(out_img, "wb").write(img)

    names = {0: "byte", 2: "sel16", 3: "p1616", 5: "off16", 6: "p1632", 7: "off32", 8: "off32s"}
    print("sig", sig.decode(), "lfanew", hex(lfanew), "pageSize", hex(page_size), "pages", pagecnt)
    for o in objects:
        print(f"obj{o['number']}: size=0x{o['size']:x} base=0x{o['mapbase']:x} flags=0x{o['flags']:x} "
              f"pti={o['pti']} pages={o['pc']} -> ..0x{o['mapbase'] + o['size']:x}")
    for k in sorted(stats):
        print(f"fixups[{names.get(k, k)}] = {stats[k]}")

    # --- objdump object1 ----------------------------------------------------
    # the image is a full linear image from address 0, so printed offsets ARE
    # linear addresses (no --adjust-vma); object1 = the W1 .text range.
    o1 = objects[0]
    start = o1["mapbase"]
    length = o1["size"]
    dump = subprocess.run(
        ["objdump", "-D", "-b", "binary", "-m", "i386", "-M", "intel",
         f"--start-address={start}",
         f"--stop-address={start + length}", out_img],
        capture_output=True, text=True, check=True)
    open(out_dump, "w").write(dump.stdout)
    print("wrote", out_dump, len(dump.stdout), "bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
