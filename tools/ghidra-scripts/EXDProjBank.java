/*-
 * EXDProjBank.java - W12-S3 hop: pin the EXD twin of the 50x0x22
 * projectile bank (EXW 0x4cc654) + re-confirm the 0x36 bank twin
 * (0x980d4, RE-EXD-MAP sec 5c) from the enemy-tick family the
 * MissionShell loop calls 4x/frame (FUN_000212f2/FUN_00022a52) and
 * the artillery free-slot spawn (FUN_00023295).
 * Usage: EXDProjBank <outFile>
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
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class EXDProjBank extends GhidraScript {

	private DecompInterface di;
	private PrintWriter out;

	private void decompFn(String tag, long addr, int maxLines) {
		out.println();
		out.println("===== " + tag + " FUN_" + String.format("%08x", addr) + " =====");
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

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDProjBank <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD projectile-bank probe (W12-S3): the enemy tick family +");
			out.println("# the artillery spawn free-slot finder.");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// The MissionShell enemy x4 family (RE-EXD-MAP sec 2):
			// FUN_000212f2(i) + FUN_00022a52(i) + FUN_0002a0f7 on odd i.
			// One of these is the FUN_00412010 projectile-tick twin.
			decompFn("enemy-tick-A", 0x212f2L, 700);
			decompFn("enemy-tick-B", 0x22a52L, 700);
			decompFn("enemy-tick-C", 0x2a0f7L, 500);
			// The artillery-case free-slot spawn into the 0x36 bank
			// (sec 5c: FUN_00023295) — re-confirms base 0x980d4 + the
			// 400 bound + the record field writes.
			decompFn("artillery-spawn", 0x23295L, 400);

			// Census: every instruction whose mnemonic is IMUL with an
			// 0x22 (or 0x36) immediate — the stride idioms over the
			// whole program (bounds the bank family).
			out.println();
			out.println("===== stride-imul census (0x22 / 0x36) =====");
			InstructionIterator it =
				currentProgram.getListing().getInstructions(true);
			int shown = 0;
			while (it.hasNext() && shown < 200) {
				Instruction ins = it.next();
				String txt = ins.toString();
				if (txt.startsWith("IMUL") && (txt.contains(",0x22,") || txt.contains(",0x22 ")
					|| txt.contains(",0x36,") || txt.contains(",0x36 ") || txt.endsWith(",0x22")
					|| txt.endsWith(",0x36"))) {
					out.println(String.format("%08x  %s", ins.getAddress().getOffset(), txt));
					shown++;
				}
			}
			out.println("// stride census shown " + shown);
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
