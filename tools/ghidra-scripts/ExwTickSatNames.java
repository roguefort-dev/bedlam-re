/*-
 * ExwTickSatNames.java - persist names derived from the exw-tick-sats.txt
 * dump pass (tick-satellite naming run 2026-08-18). NEVER re-import.
 * Names + plate comments for the 19 dumped satellites; data labels for
 * the new globals incl. the four DD surface roles.
 * Output: <outDir>/exw-tick-sat-names.txt
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwTickSatNames extends GhidraScript {

	private static final String[][] FN_NAMES = {
		{ "00402b0c", "TickCounters", "per 100Hz tick: ++five free counters (004edb84/edbc8/edbcc/edba4/edba8), PlayClockTick, fire FadeStep when (004edbc8 & 1) && g_fade_ticks_left" },
		{ "00425ab9", "ScrollUpdate", "cursor->640x480 scroll coords clamp 9..631/9..463; mouse-btn edge latch (004edb48 arm / 004ede14 fire); bit0/bit1 direction copies; region palette 0x5d when x >= 0x1e0" },
		{ "0041d714", "SetPaletteIndex", "apply palette bank idx: guard last-applied+reentrant; CursorSizeSet(0x18); PalSurfPrep; Lock palbank surf 004ee9c8 (best-effort for idx 0x90..0x97, retry otherwise); clear 0x400; copy 24 rows x 24B from table @004edd7c+idx*4 at stride 0x20; Unlock; record idx" },
		{ "0044b040", "GetPalette6bit", "convert PALETTEENTRY array @004ee9f4 (stride 4) >> 2 into 768B 6-bit snapshot @004eee60; return 0x4eee60" },
		{ "0044b7b0", "CursorBlt", "blit cursor sprite surface 004ee9cc onto primary 004ee9bc (+0x14, DDBLT_WAIT 0x1000000), rect clipped to window edges capped at g_cursor_size; IsLost/Restore both surfaces; parks g_cursor_x (00457398) = 0xffff after; gate: primary present && 00457396 hi != 0xffff" },
		{ "0044bbac", "CursorSizeSet", "store AX -> g_cursor_size (004eedfc); if g_cursor_active (004ee9d8) == 1: call CursorBlt first, park g_cursor_x, keep active flag 1" },
		{ "0044bb84", "PalSurfPrep", "single +0x74 vtable call on palbank surf 004ee9c8, args (this, 8, &8B local {AX,AX}) - SetColorKey-shaped, method unresolved [hypothesis]" },
		{ "0044bc90", "PalSurfLock", "fullscreen gate (004ef676 hi == 1); IsLost/Restore + Lock(+0x64, DDLOCK_WAIT, desc 0x6c) on palbank surf 004ee9c8; return desc lpSurface or 0" },
		{ "0044bcf4", "PalSurfUnlock", "Unlock(+0x80) on palbank surf 004ee9c8, fullscreen gate" },
		{ "0044d1f2", "BmpNameBuild", "screenshot filename builder: 0044f40e format engine w/ template, then NUL-terminate" },
		{ "0044e729", "Fopen", "Watcom CRT fopen(path, mode): tail-call 0044e6f0(path, mode, 0)" },
		{ "0044e815", "Fwrite", "Watcom CRT buffered fwrite(buf EAX, size EDX, n EBX, FILE* ECX): REP MOVSD into FILE buffer, flush via 0045036c, direct write 00450a92 when buffer empty and chunk >= bufsize; returns written/size" },
		{ "0044f34b", "Putc", "Watcom CRT buffered putc: newline expands to CR+LF unless FILE flag 0x40; flush threshold 0x400 (0x600 for newline); returns byte or EOF" },
		{ "0044e30b", "Ftell", "Watcom CRT ftell: 004504c8 lseek(cur) adjusted +- by buffered fill level (FILE+4)" },
		{ "0044e217", "Fseek", "Watcom CRT fseek(stream, off EDX, whence EBX 0/1/2): flush write buf / rewind read buf then 0045048e lseek; returns 0 or -1" },
		{ "0044ea0b", "Fclose", "Watcom CRT fclose: walk open-file list @004ef788 matching node+4, then 0044ea4a(stream, 1)" },
		{ "0044ac5c", "LockStaging", "restore primary 004ee9bc; Lock(+0x64, DDLOCK_WAIT) staging surface 004ee9c0 (gates 004ef670 + fullscreen); cache pitch 004ee9e8, >>2 @004ee9f0, -0x280 @004ee9ec; return base ptr" },
		{ "0044f237", "Malloc", "Watcom CRT malloc(size EAX): round4 min 0xc, free-list walk from 004577d8 via 00450e3e, largest-free tracking @004577e0, heap grow 004510f5/00451150; errno slot 004ef78c = 0" },
		{ "00451fbc", "CrtThreadTrampoline", "Watcom CRT per-thread start: runtime init once (004ef77c), TIB from arg ([0]=start, [4]=arg, [0xc]=stack), 00450260 register, CALL start(PUSH arg), 00450259 exit; RET 4" },
	};

	private static final String[][] DATA_NAMES = {
		{ "004eee60", "g_pal6_snap" },
		{ "004eedfc", "g_cursor_size" },
		{ "004ee9d8", "g_cursor_active" },
		{ "004ede14", "g_scroll_btn_latch" },
		{ "004edb48", "g_scroll_btn_armed" },
		{ "004ee9bc", "g_dd_surf_primary" },
		{ "004ee9c0", "g_dd_surf_staging" },
		{ "004ee9c8", "g_dd_surf_palbank" },
		{ "004ee9cc", "g_dd_surf_cursor" },
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwTickSatNames: output dir = " + outDir.toAbsolutePath());

		SymbolTable symTable = currentProgram.getSymbolTable();
		List<String> log = new ArrayList<>();

		for (String[] entry : FN_NAMES) {
			Address addr = parse(entry[0]);
			if (addr == null) {
				log.add(entry[0] + "\tSKIP bad address");
				continue;
			}
			Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
			if (fn == null) {
				try {
					if (currentProgram.getListing().getInstructionAt(addr) == null) {
						disassemble(addr);
					}
					fn = createFunction(addr, entry[1]);
				}
				catch (Exception e) {
					log.add(addr + "\tSKIP create failed: " + e);
					continue;
				}
			}
			if (fn == null) {
				log.add(addr + "\tSKIP createFunction null");
				continue;
			}
			String old = fn.getName();
			try {
				if (!old.equals(entry[1])) {
					fn.setName(entry[1], SourceType.USER_DEFINED);
					log.add(addr + "\t" + old + " -> " + entry[1]);
				}
				else {
					log.add(addr + "\tALREADY " + entry[1]);
				}
				currentProgram.getListing().getCodeUnitAt(addr).setComment(CodeUnit.PLATE_COMMENT, entry[2]);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP rename/comment: " + e);
			}
		}
		for (String[] entry : DATA_NAMES) {
			Address addr = parse(entry[0]);
			if (addr == null) {
				continue;
			}
			try {
				Symbol s = symTable.getPrimarySymbol(addr);
				if (s != null && s.getName().equals(entry[1])) {
					log.add(addr + "\tALREADY " + entry[1]);
				}
				else {
					createLabel(addr, entry[1], true, SourceType.USER_DEFINED);
					log.add(addr + "\tLABEL -> " + entry[1] + (s != null ? " (was " + s.getName() + ")" : ""));
				}
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP label: " + e);
			}
		}

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-tick-sat-names.txt"), StandardCharsets.UTF_8))) {
			for (String line : log) {
				out.println(line);
				println(line);
			}
		}
		println("ExwTickSatNames: done.");
	}

	private Address parse(String addrStr) {
		try {
			return currentProgram.getAddressFactory().getAddress(addrStr);
		}
		catch (Exception e) {
			return null;
		}
	}
}
