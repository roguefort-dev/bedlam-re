/*-
 * ExwFontDrawer.java - FULLFONT.BIN glyph pass (P5/D35 prep): RE the EXW
 * text drawer FUN_0043c87c (47 inbound refs, the generic font blitter of
 * the four LAB_0041c69e loading-row draws) - decompile it plus every
 * direct CALL target found in its listing (cap 10). Disassemble the
 * GameMain zone-complete tail (0x0041c69e..0x0041c9f0) to pin: the
 * FUN_0043c87c argument setup per draw, and the font-ramp copy loop
 * exact byte counts (24 dwords + a tail loop whose counter the
 * decompiler ate). Dump the draw-string data: 0x0046bc4c, 0x0046bc7c,
 * 0x0046bfdc, and the 0x30-stride table at 0x0046af5c (indices
 * 0x50..=0x58 used by (zone+0x51)*0x30, zone 2..6 -> 0x53..0x57).
 * Runs against the already-imported BedlamWatcom project, NEVER
 * re-import:
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExwFontDrawer.java <outputFile>
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.Memory;

public class ExwFontDrawer extends GhidraScript {

	/* the text drawer + GameMain tail window */
	private static final String DRAWER = "0043c87c";
	private static final String TAIL_LO = "0041c69e";
	private static final String TAIL_HI = "0041c9f0";

	/* string/table data regions: {start, length, label} */
	private static final Object[][] DATA = {
		{ "0046bc4c", 0x40, "draw1 string DAT_0046bc4c (x=0x96)" },
		{ "0046bc7c", 0x40, "draw2 string DAT_0046bc7c (x=0xb4)" },
		{ "0046bfdc", 0x40, "draw4 string DAT_0046bfdc (zone6 x=0x104)" },
		{ "0046af5c", 0x30 * 0x5a, "0x30-stride table DAT_0046af5c (idx 0..0x59; draws use 0x53..0x57)" },
		{ "00410413", 0x80, "FUN_00410493 jumptable (32 dwords)" },
		{ "004103f4", 0x20, "FUN_00410493 match table" },
	};

	private static final int MAX_AUTO = 10;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-font-drawer.txt");
		if (outPath.getParent() != null) {
			Files.createDirectories(outPath.getParent());
		}
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outPath, StandardCharsets.UTF_8))) {

			section(out, "INFO");
			out.println("program: " + currentProgram.getName());

			section(out, "DRAWER");
			Function drawer = fnAt(DRAWER);
			if (drawer == null) {
				out.println("SKIP no function at " + DRAWER);
			}
			else {
				dumpFn(out, decomp, drawer);
				Set<String> callees = new LinkedHashSet<>();
				InstructionIterator it = currentProgram.getListing()
					.getInstructions(drawer.getBody(), true);
				while (it.hasNext()) {
					Instruction ins = it.next();
					if (ins.getMnemonicString().startsWith("CALL")) {
						for (Address t : ins.getFlows()) {
							callees.add(t.toString());
						}
					}
				}
				out.println("call targets: " + callees);

				section(out, "DRAWER_CALLEES");
				int n = 0;
				for (String ep : callees) {
					if (n >= MAX_AUTO) {
						break;
					}
					Function fn = fnAt(ep);
					if (fn == null) {
						continue;
					}
					dumpFn(out, decomp, fn);
					n++;
				}
				out.println("// decompiled " + n + " of " + callees.size()
					+ " callees (cap " + MAX_AUTO + ")");
			}

			section(out, "GAMEMAIN_TAIL_LISTING");
			Address lo = parse(TAIL_LO);
			Address hi = parse(TAIL_HI);
			InstructionIterator it = currentProgram.getListing()
				.getInstructions(lo, true);
			while (it.hasNext()) {
				Instruction ins = it.next();
				if (ins.getAddress().getOffset() > hi.getOffset()) {
					break;
				}
				out.println(ins.toString());
			}

			section(out, "DATA");
			Memory mem = currentProgram.getMemory();
			for (Object[] d : DATA) {
				Address a = parse((String) d[0]);
				int len = (Integer) d[1];
				out.println("----- " + d[0] + " len=0x" + Integer.toHexString(len)
					+ " : " + d[2] + " -----");
				byte[] buf = new byte[len];
				try {
					for (int k = 0; k < len; k++) {
						buf[k] = mem.getByte(a.add(k));
					}
				}
				catch (Exception ex) {
					out.println("// READ FAILED at " + d[0] + ": " + ex);
					continue;
				}
				for (int off = 0; off < len; off += 16) {
					StringBuilder hex = new StringBuilder();
					StringBuilder asc = new StringBuilder();
					for (int k = 0; k < 16 && off + k < len; k++) {
						int b = buf[off + k] & 0xff;
						hex.append(String.format("%02x ", b));
						asc.append(b >= 0x20 && b < 0x7f ? (char) b : (char) 46);
					}
					out.println(String.format("+%04x  %-48s |%s|",
						off, hex.toString().trim(), asc.toString()));
				}
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwFontDrawer: done -> " + outPath);
	}

	private void dumpFn(PrintWriter out, DecompInterface decomp, Function fn)
			throws Exception {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		try {
			DecompileResults r = decomp.decompileFunction(fn, 120, monitor);
			out.println(r.decompileCompleted() && r.getDecompiledFunction() != null
				? r.getDecompiledFunction().getC()
				: "// DECOMP FAILED: " + r.getErrorMessage());
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
		}
		out.println("listing:");
		InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			out.println(it.next().toString());
		}
	}

	private void section(PrintWriter out, String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	private Function fnAt(String addrStr) {
		Address a = parse(addrStr);
		return a == null ? null : currentProgram.getFunctionManager().getFunctionContaining(a);
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
