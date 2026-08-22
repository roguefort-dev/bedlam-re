/*-
 * EXDInputTwin4.java - W5-followup hop 4 (final): instruction addresses
 * for the two difficulty sites in FUN_00023967, program-wide refs to the
 * difficulty cell 0x119558 (writer census incl. the (d+1)%3 cycle twin),
 * and the FUN_00021112 decompile head (click-order twin identity).
 * Usage: EXDInputTwin4 <outFile>
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

public class EXDInputTwin4 extends GhidraScript {

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

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDInputTwin4 <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD input-twin census hop 4");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// ---- A. difficulty refs program-wide ----
			hdr("A refs to difficulty cell 0x119558");
			for (Reference r : getReferencesTo(toAddr(0x119558L))) {
				Function cf = currentProgram.getFunctionManager()
					.getFunctionContaining(r.getFromAddress());
				out.println(String.format("REF %08x %s [%s]", r.getFromAddress().getOffset(),
					r.getReferenceType(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
			}

			// ---- B. disasm FUN_00023967, filter locally ----
			hdr("B disasm FUN_00023967 (full)");
			Function fn = currentProgram.getFunctionManager().getFunctionAt(toAddr(0x23967L));
			if (fn != null) {
				for (Address p = fn.getEntryPoint(); p.compareTo(fn.getBody().getMaxAddress()) <= 0;) {
					Instruction ins = currentProgram.getListing().getInstructionAt(p);
					if (ins == null) {
						p = p.add(1);
						continue;
					}
					out.println(p + " " + ins);
					p = ins.getMaxAddress().add(1);
				}
			}

			// ---- C. click-order twin identity ----
			hdr("C decompile FUN_00021112 head (click-order twin)");
			decompFn(0x21112L, 150);

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
