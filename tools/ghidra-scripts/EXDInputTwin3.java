/*-
 * EXDInputTwin3.java - W5-followup hop 3: pin the difficulty cell inside
 * the two {172,236,300}-carrying functions (FUN_00023967 epilogue tick,
 * FUN_0001476d draw-chain monolith), confirm the keystore writer ISR +
 * InputReset twin (FUN_0003064d), and census refs to the command-count
 * cell 0x119588 (builder bump site) + the order-target triple 0x10e0a4/
 * a8/ac (the click-order writer = second anchor).
 * Usage: EXDInputTwin3 <outFile>
 * Ghidra discipline: -process BEDLAM.EXD -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

public class EXDInputTwin3 extends GhidraScript {

	private DecompInterface di;
	private PrintWriter out;

	private void hdr(String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	private void decompFn(long addr, int maxLines) {
		Function fn = currentProgram.getFunctionManager().getFunctionAt(toAddr(addr));
		if (fn == null) {
			out.println("// no function at " + String.format("%08x", addr));
			return;
		}
		out.println("// fn " + fn.getName() + "@" + fn.getEntryPoint() + " size "
			+ fn.getBody().getNumAddresses());
		DecompileResults r = di.decompileFunction(fn, 240, monitor);
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
			throw new IllegalArgumentException("usage: EXDInputTwin3 <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD input-twin census hop 3");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// ---- A. difficulty carriers ----
			hdr("A decompile FUN_00023967 (epilogue tick, has 172/236/300)");
			decompFn(0x23967L, 700);
			hdr("A2 decompile FUN_0001476d (draw monolith, 172/236/300 + 640/704/768 + IDIV3)");
			decompFn(0x1476dL, 700);

			// ---- B. keystore writer ISR region + InputReset twin ----
			hdr("B disasm keystore writer region 0x30430..0x304f5");
			disasmRange(0x30430L, 0x304f5L);
			hdr("B2 decompile FUN_0003064d (InputReset twin candidate)");
			decompFn(0x3064dL, 120);
			hdr("B3 decompile FUN_000306b8 (shift/ctrl reader)");
			decompFn(0x306b8L, 120);
			hdr("B4 disasm 0x307c1..0x30810 (AnyKeyWait twin-2/shift check family)");
			disasmRange(0x307c1L, 0x30810L);

			// ---- C. command count cell census ----
			refsTo("command count cell", 0x119588L);

			// ---- D. order-target writer census (click-order twin = 2nd anchor) ----
			refsTo("order target X", 0x10e0a4L);
			refsTo("order target Y", 0x10e0a8L);
			refsTo("order target Z", 0x10e0acL);

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
