/*-
 * EXDRobotBackhalf.java - W7-followup: pin the remaining canonical
 * robot-record field offsets in EXD (RE-EXD-MAP sec 8 coverage-gap
 * census) by decoding the robots() monolith FUN_0001c7dc's record
 * accesses + a program-wide immediate census of the robot-bank base
 * family (0xf6d34..0xf6ddc, stride 0xA8) and the move-target arrays.
 * Usage: EXDRobotBackhalf <outFile>
 * Ghidra discipline: -process BEDLAM.EXD -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.regex.Pattern;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;

public class EXDRobotBackhalf extends GhidraScript {

	private static final long ROBOT_BASE = 0xf6d34L;

	private DecompInterface di;
	private PrintWriter out;

	private void hdr(String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDRobotBackhalf <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD robot back-half probe (W7-followup)");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// ---- A. program-wide immediate census: every instruction whose
			// text mentions a robot-bank family constant 0xf6d34..0xf6de0
			// (base + any record offset), grouped per containing function.
			hdr("A robot-base family immediate census (0xf6d3x/0xf6ddx/0xf6dex)");
			Pattern fam = Pattern.compile("f6d([3-9a-e])[0-9a-f]", Pattern.CASE_INSENSITIVE);
			Function cur = null;
			InstructionIterator it =
				currentProgram.getListing().getInstructions(true);
			while (it.hasNext()) {
				Instruction ins = it.next();
				String t = ins.toString();
				if (fam.matcher(t).find()) {
					Function f = currentProgram.getFunctionManager()
						.getFunctionContaining(ins.getAddress());
					if (f != cur) {
						cur = f;
						out.println();
						out.println(String.format("-- FN %s@%s size %d",
							f == null ? "-" : f.getName(),
							f == null ? "-" : f.getEntryPoint(),
							f == null ? 0 : f.getBody().getNumAddresses()));
					}
					out.println(ins.getAddress() + " " + t);
				}
			}

			// ---- B. refs to the move-target arrays + robot count + player
			// type (extent + indexing evidence)
			hdr("B refs to move-target 0xf75ec/0xf761c, count 0x11958c, cap 0x11950c, player type 0x1075c0");
			for (long cell : new long[] {0xf75ecL, 0xf761cL, 0x11958cL, 0x11950cL, 0x1075c0L}) {
				out.println();
				out.println(String.format("-- cell %08x", cell));
				for (Reference r : getReferencesTo(toAddr(cell))) {
					Function cf = currentProgram.getFunctionManager()
						.getFunctionContaining(r.getFromAddress());
					out.println(String.format("REF %08x %s [%s]", r.getFromAddress().getOffset(),
						r.getReferenceType(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
				}
			}

			// ---- C. full disassembly of the robots() monolith
			hdr("C disasm FUN_0001c7dc (robots() monolith, full)");
			Function fn = currentProgram.getFunctionManager().getFunctionAt(toAddr(0x1c7dcL));
			if (fn != null) {
				out.println("// fn " + fn.getName() + "@" + fn.getEntryPoint() + " size "
					+ fn.getBody().getNumAddresses());
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
			else {
				out.println("// no function at 0x1c7dc");
			}

			// ---- D. decompile FUN_0001c7dc (full; the field-role evidence)
			hdr("D decompile FUN_0001c7dc (robots() monolith, full)");
			decompFn(0x1c7dcL, 9000);

			// ---- E. decompile the function containing the stat-copy switch
			// (0x9240c = the 0x2a/0x2b/0x2c extras per sec 5b)
			hdr("E decompile fn containing 0x9240c (order-table stat copy)");
			Function sf = currentProgram.getFunctionManager().getFunctionContaining(toAddr(0x9240cL));
			if (sf != null) {
				out.println("// fn " + sf.getName() + "@" + sf.getEntryPoint() + " size "
					+ sf.getBody().getNumAddresses());
				// decompile by entry
				decompFn(sf.getEntryPoint().getOffset(), 700);
			}
			else {
				out.println("// no function containing 0x9240c");
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
}
