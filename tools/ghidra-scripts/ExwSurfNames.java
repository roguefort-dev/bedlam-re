/*-
 * ExwSurfNames.java - persist pass for the DD surface creation-order
 * confirmation (2026-08-18, dump exw-surf-confirm.txt). NEVER re-import.
 * Renames FUN_0044a9ac -> DDStagingProbe, plate comment on DDInitSurfaces
 * with the creation-order facts, labels for the two new globals.
 * Output: <outDir>/exw-surf-names.txt
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

public class ExwSurfNames extends GhidraScript {

	private static final String[][] FN_NAMES = {
		{ "0044a9ac", "DDStagingProbe", "fullscreen staging/backbuffer probe + double clear, run from the DD-init chain right after DDInitSurfaces and from DDShutdown before release (2 callers): saves present gate 004ef670 (sets 1), Unlock+FlipOrBlt; sentinel 0x12345678 written through LockStaging (retried max 20), Unlock+FlipOrBlt, relock + readback -> word 004ee9e4 = 1 when staging memory SURVIVES the present (persistent backbuffer); then two full clears of staging (480 rows x 160 dwords, row stride pitch/4 @004ee9f0) each followed by Unlock+FlipOrBlt; restores 004ef670" },
		{ "0044a660", "DDInitSurfaces", "creation order (2026-08-18 confirm pass): FULLSCREEN = CreateSurface dwSize 0x6c flags 0x21 caps 0x4218 (COMPLEX|FLIP|PRIMARYSURFACE|0x4000; caps byte0x6d &= 0xbf drops the 0x4000 bit on retry) backbuffercount 1 -> 004ee9bc chain head, then surface +0x30 GetAttachedSurface(caps DDSCAPS_BACKBUFFER=4) -> 004ee9c0 = implicit backbuffer; WINDOWED = CreateSurface(PRIMARYSURFACE 0x200) -> 004ee9bc, then CreateSurface flags 0x7 (CAPS|HEIGHT|WIDTH) caps 0x40 (OFFSCREENPLAIN) w x h -> 004ee9c0; clipper CreateClipper -> 004ee9d4 + SetHWnd + primary SetClipper(+0x70) windowed only; CreatePalette(0x44, entries 004ee9f4) -> 004ee9d0 + primary SetPalette(+0x7c) when RC_PALETTE; tail calls FUN_0044b9c4 + FUN_0044ba3c [inferred: palbank 004ee9c8 / cursor 004ee9cc surface creators]; word 004ee9e4 cleared 0 before the probe" },
	};

	private static final String[][] DATA_NAMES = {
		{ "004ee9e4", "g_staging_persistent" },
		{ "004ee9b6", "g_pal_dirty" },
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwSurfNames: output dir = " + outDir.toAbsolutePath());

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
				log.add(addr + "\tSKIP no function");
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
			Files.newBufferedWriter(outDir.resolve("exw-surf-names.txt"), StandardCharsets.UTF_8))) {
			for (String line : log) {
				out.println(line);
				println(line);
			}
		}
		println("ExwSurfNames: done.");
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
