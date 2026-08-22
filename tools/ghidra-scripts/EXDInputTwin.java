/*-
 * EXDInputTwin.java - W5-followup: the EXD input-twin census.
 * Targets (queue item 2): (1) the command-ring consumer + count twin +
 * order-target triple via the EXD MissionShell pre-tick call cluster;
 * (2) the keystore base via the any-key-scan shift-skip pair
 * (CMP 0x2a + CMP 0x36, EXW FUN_0041f9d1 scans 1..0xFE) and memset-256
 * (EXW InputReset 0x4207b5); (3) the difficulty cell via the 7j.17
 * critter constants {172,236,300} / {640,704,768} and mod-3 magic
 * 0x55555556 ((d+1)%3 at name entry).
 * Usage: EXDInputTwin <outFile>
 * Ghidra discipline: -process BEDLAM.EXD -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
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
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;

public class EXDInputTwin extends GhidraScript {

	private DecompInterface di;
	private PrintWriter out;

	private void hdr(String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	private void disasmFn(long addr, int maxIns) {
		Function fn = currentProgram.getFunctionManager().getFunctionAt(toAddr(addr));
		if (fn == null) {
			out.println("// no function at " + String.format("%08x", addr));
			return;
		}
		out.println("// fn " + fn.getName() + "@" + fn.getEntryPoint() + " size "
			+ fn.getBody().getNumAddresses());
		int n = 0;
		for (Address p = fn.getEntryPoint(); p.compareTo(fn.getBody().getMaxAddress()) <= 0
			&& n < maxIns;) {
			Instruction ins = currentProgram.getListing().getInstructionAt(p);
			if (ins == null) {
				p = p.add(1);
				continue;
			}
			out.println(p + " " + ins);
			p = ins.getMaxAddress().add(1);
			n++;
		}
		if (n >= maxIns) {
			out.println("// DISASM TRUNCATED at " + maxIns + " instructions");
		}
	}

	private void decompFn(long addr, int maxLines) {
		Function fn = currentProgram.getFunctionManager().getFunctionAt(toAddr(addr));
		if (fn == null) {
			out.println("// no function at " + String.format("%08x", addr));
			return;
		}
		out.println("// fn " + fn.getName() + "@" + fn.getEntryPoint() + " size "
			+ fn.getBody().getNumAddresses());
		DecompileResults r = di.decompileFunction(fn, 120, monitor);
		if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
			String[] lines = r.getDecompiledFunction().getC().split("[\r\n]+");
			for (int i = 0; i < lines.length && i < maxLines; i++) {
				out.println(lines[i]);
			}
			if (lines.length > maxLines) {
				out.println("// DECOMP TRUNCATED " + (lines.length - maxLines));
			}
		}
		else {
			out.println("// DECOMP FAILED: " + r.getErrorMessage());
		}
	}

	private void refsTo(String tag, long addr) {
		hdr("REFS to " + String.format("%08x", addr) + " (" + tag + ")");
		for (Reference r : getReferencesTo(toAddr(addr))) {
			Function cf = currentProgram.getFunctionManager()
				.getFunctionContaining(r.getFromAddress());
			out.println(String.format("REF %08x %s [%s]", r.getFromAddress().getOffset(),
				r.getReferenceType(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
		}
	}

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDInputTwin <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD input-twin census (W5-followup, queue item 2)");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// ---- PASS 1: MissionShell pre-tick call cluster (command consumer?) ----
			hdr("P1 mission-loop pre-tick candidates (decompile)");
			// EXD MissionShell FUN_000596ed loop head calls before the epilogue tick
			// FUN_00023967 (from exd-probe7.txt lines 456-509):
			long[] cands = { 0x5a76dL, 0x34895L, 0x348c2L, 0x359b3L, 0x12fcfL };
			for (long c : cands) {
				decompFn(c, 220);
			}

			// ---- listing census infra ----
			Map<String, Set<Long>> byImm = new HashMap<>();
			String[] imms = { "0x2a", "0x36", "0x55555556", "0x26" };
			for (String s : imms) {
				byImm.put(s, new HashSet<>());
			}
			InstructionIterator ii = currentProgram.getListing().getInstructions(true);
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				String m = ins.getMnemonicString();
				boolean isCmp = m.equals("CMP") || m.equals("cmp");
				String t = ins.toString();
				for (String s : imms) {
					boolean hit = t.matches(".*\\b" + s + "\\b.*");
					if (!hit) {
						continue;
					}
					if (s.equals("0x55555556") || isCmp) {
						Function f = currentProgram.getFunctionManager()
							.getFunctionContaining(ins.getAddress());
						if (f != null) {
							byImm.get(s).add(f.getEntryPoint().getOffset());
						}
						else {
							byImm.get(s).add(ins.getAddress().getOffset());
						}
					}
				}
			}

			// ---- PASS 2: keystore = any-key scan (shift-skip pair) ----
			hdr("P2 functions with CMP 0x2a (shift-skip L)");
			for (Long a : byImm.get("0x2a")) {
				out.println(String.format("%08x", a));
			}
			hdr("P2 functions with CMP 0x36 (shift-skip R)");
			for (Long a : byImm.get("0x36")) {
				out.println(String.format("%08x", a));
			}
			Set<Long> pair = new HashSet<>(byImm.get("0x2a"));
			pair.retainAll(byImm.get("0x36"));
			hdr("P2 INTERSECTION (both shift-skips = AnyKeyWait family)");
			for (Long a : pair) {
				out.println(String.format("%08x", a));
				disasmFn(a, 260);
			}

			// ---- PASS 3a: difficulty data tables {172,236,300} / {640,704,768} ----
			hdr("P3a memory pattern scan");
			Memory mem = currentProgram.getMemory();
			byte[][] pats = {
				{ (byte) 0xAC, 0, 0, 0, (byte) 0xEC, 0, 0, 0, 0x2C, 1, 0, 0 }, // dword 172/236/300
				{ (byte) 0xAC, 0, (byte) 0xEC, 0, 0x2C, 1 }, // word form
				{ (byte) 0x80, 2, 0, 0, (byte) 0xC0, 2, 0, 0, 0, 3, 0, 0 }, // dword 640/704/768
				{ (byte) 0x80, 2, (byte) 0xC0, 2, 0, 3 }, // word form
			};
			String[] pnames = { "d172_236_300_dword", "d172_236_300_word", "l640_704_768_dword",
				"l640_704_768_word" };
			for (MemoryBlock b : mem.getBlocks()) {
				long s = b.getStart().getOffset(), e = b.getEnd().getOffset();
				if (s < 0x80000L || s > 0x130000L) {
					continue; // data blocks only
				}
				int len = (int) (e - s + 1);
				byte[] buf = new byte[len];
				try {
					mem.getBytes(b.getStart(), buf);
				}
				catch (Exception ex) {
					out.println("// block " + b.getName() + " read fail " + ex);
					continue;
				}
				for (int pi = 0; pi < pats.length; pi++) {
					for (int o = 0; o + pats[pi].length <= len; o++) {
						boolean ok = true;
						for (int k = 0; k < pats[pi].length; k++) {
							if (buf[o + k] != pats[pi][k]) {
								ok = false;
								break;
							}
						}
						if (ok) {
							out.println(pnames[pi] + " @ " + String.format("%08x", s + o));
						}
					}
				}
			}

			// ---- PASS 3b: mod-3 magic census ----
			hdr("P3b functions with 0x55555556 (div/mod 3 magic)");
			for (Long a : byImm.get("0x55555556")) {
				out.println(String.format("%08x", a));
			}

			// ---- PASS 3c: 39-case weapon switch bound (id-2 <= 0x26) ----
			hdr("P3c functions with CMP 0x26 (weapon id-2 bound candidates)");
			for (Long a : byImm.get("0x26")) {
				out.println(String.format("%08x", a));
			}
			out.println();
			out.println("# DONE");
		}
		finally {
			if (di != null) {
				di.dispose();
			}
		}
	}
}
