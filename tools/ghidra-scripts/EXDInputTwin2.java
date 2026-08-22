/*-
 * EXDInputTwin2.java - W5-followup hop 2: decompile the consumer twin
 * candidate FUN_00019ee9 (pre-robots slot, CMP 0x26 hit) + FUN_0005b066
 * (the click-order/builder candidate called just before), census ALL
 * refs to the pinned keystore base 0x894d4 (writer = DOS key handler,
 * InputReset memset twin, readers), and re-hunt the difficulty cell via
 * per-function immediate triples {0xac,0xec,0x12c} (172/236/300) and
 * {0x280,0x2c0,0x300} (640/704/768) + div-by-3 sites.
 * Usage: EXDInputTwin2 <outFile>
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
import java.util.TreeSet;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;

public class EXDInputTwin2 extends GhidraScript {

	private DecompInterface di;
	private PrintWriter out;

	private void hdr(String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	private Function fnAt(long addr) {
		return currentProgram.getFunctionManager().getFunctionAt(toAddr(addr));
	}

	private void decompFn(long addr, int maxLines) {
		Function fn = fnAt(addr);
		if (fn == null) {
			out.println("// no function at " + String.format("%08x", addr));
			return;
		}
		out.println("// fn " + fn.getName() + "@" + fn.getEntryPoint() + " size "
			+ fn.getBody().getNumAddresses());
		DecompileResults r = di.decompileFunction(fn, 180, monitor);
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

	private void disasmRange(long lo, long hi) {
		Address p = toAddr(lo);
		Address end = toAddr(hi);
		while (p.compareTo(end) <= 0) {
			Instruction ins = currentProgram.getListing().getInstructionAt(p);
			if (ins == null) {
				p = p.add(1);
				continue;
			}
			out.println(p + " " + ins);
			p = ins.getMaxAddress().add(1);
		}
	}

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDInputTwin2 <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD input-twin census hop 2");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// ---- A. consumer twin candidate (pre-robots slot) ----
			hdr("A decompile FUN_00019ee9 (consumer candidate)");
			decompFn(0x19ee9L, 420);

			hdr("A2 decompile FUN_0005b066 (click-order/builder candidate)");
			decompFn(0x5b066L, 300);

			// ---- B. keystore 0x894d4 ref census ----
			hdr("B refs to keystore base cell 0x894d4 (full listing census)");
			// every instruction whose text mentions 894d4 with an indexed form
			InstructionIterator ii = currentProgram.getListing().getInstructions(true);
			Set<Long> fns = new TreeSet<>();
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				String t = ins.toString();
				if (t.contains("894d4")) {
					Function f = currentProgram.getFunctionManager()
						.getFunctionContaining(ins.getAddress());
					String fs = (f == null ? "-" : f.getName() + "@" + f.getEntryPoint());
					out.println(ins.getAddress() + " " + t + "   [" + fs + "]");
					if (f != null) {
						fns.add(f.getEntryPoint().getOffset());
					}
				}
			}
			hdr("B2 functions touching 894d4 (bytes above 0x894d4+256 are NOT keystore)");
			for (Long a : fns) {
				out.println(String.format("%08x", a));
			}
			// the byte range 0x894d4..0x895d3 = keystore; indexed refs may use
			// base 0x894d4 or 0x894d5-style splinters; also scan for 0x894d5..0x894ff
			hdr("B3 raw refs to ANY keystore-internal address 0x894d4..0x89500 (non-indexed)");
			ii = currentProgram.getListing().getInstructions(true);
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				String t = ins.toString();
				for (long a = 0x894d5L; a <= 0x89500L; a++) {
					if (t.contains(String.format("%x", a))) {
						Function f = currentProgram.getFunctionManager()
							.getFunctionContaining(ins.getAddress());
						out.println(ins.getAddress() + " " + t + "  hits " + String.format("%08x", a)
							+ " [" + (f == null ? "-" : f.getName() + "@" + f.getEntryPoint()) + "]");
						break;
					}
				}
			}

			// ---- C. difficulty hunt 2: immediate triples per function ----
			hdr("C per-function immediate triples");
			Map<Long, Set<Long>> fnImms = new HashMap<>();
			long[][] triples = {
				{ 0xacL, 0xecL, 0x12cL }, // 172/236/300 range
				{ 0x280L, 0x2c0L, 0x300L }, // 640/704/768 leash
			};
			ii = currentProgram.getListing().getInstructions(true);
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				String t = ins.toString();
				java.util.regex.Matcher m =
					java.util.regex.Pattern.compile("0x([0-9a-f]+)\\b").matcher(t);
				Set<Long> immset = null;
				while (m.find()) {
					long v = Long.parseLong(m.group(1), 16);
					if (v == 0xac || v == 0xec || v == 0x12c || v == 0x280 || v == 0x2c0
						|| v == 0x300) {
						if (immset == null) {
							Function f = currentProgram.getFunctionManager()
								.getFunctionContaining(ins.getAddress());
							if (f == null) {
								break;
							}
							immset = fnImms.computeIfAbsent(f.getEntryPoint().getOffset(),
								k -> new HashSet<>());
						}
						immset.add(v);
					}
				}
			}
			for (Map.Entry<Long, Set<Long>> e : fnImms.entrySet()) {
				for (long[] tr : triples) {
					boolean all = true;
					for (long v : tr) {
						if (!e.getValue().contains(v)) {
							all = false;
							break;
						}
					}
					if (all) {
						out.println(String.format("TRIPLE %s COMPLETE in fn %08x: %s",
							java.util.Arrays.toString(tr), e.getKey(), e.getValue()));
					}
				}
			}
			out.println("// (fn: immsets with any member)");
			for (Map.Entry<Long, Set<Long>> e : fnImms.entrySet()) {
				out.println(String.format("%08x %s", e.getKey(), e.getValue()));
			}

			// ---- D. div/mod-3 sites (IDIV with a 3 loaded) ----
			hdr("D functions with IDIV + literal 3 load");
			ii = currentProgram.getListing().getInstructions(true);
			Set<Long> divfns = new TreeSet<>();
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				if (!ins.getMnemonicString().equalsIgnoreCase("IDIV")) {
					continue;
				}
				Function f = currentProgram.getFunctionManager()
					.getFunctionContaining(ins.getAddress());
				if (f == null) {
					continue;
				}
				// look back 12 instructions for a MOV reg,0x3
				Address p = ins.getAddress();
				int seen = 0;
				for (int k = 0; k < 14 && seen < 12; k++) {
					Instruction prev = currentProgram.getListing().getInstructionBefore(p);
					if (prev == null) {
						break;
					}
					p = prev.getAddress();
					seen++;
					String pt = prev.toString();
					if (prev.getMnemonicString().equalsIgnoreCase("MOV")
						&& (pt.endsWith(",0x3") || pt.endsWith(",3"))) {
						divfns.add(f.getEntryPoint().getOffset());
						out.println(ins.getAddress() + " " + ins + "   (3 from " + pt + " @ "
							+ prev.getAddress() + ") in "
							+ String.format("%08x", f.getEntryPoint().getOffset()));
						break;
					}
				}
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
