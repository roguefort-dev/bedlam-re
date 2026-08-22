/*-
 * EXDRobotBackhalf2.java - W7-followup hop 2: decompile the robot-record
 * writer/reader family found by hop 1's census (spawn initializer, move
 * twin, probe twin, weapon-group readers, armor pad charge, death flag).
 * Usage: EXDRobotBackhalf2 <outFile>
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

public class EXDRobotBackhalf2 extends GhidraScript {

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
			throw new IllegalArgumentException("usage: EXDRobotBackhalf2 <outFile>");
		}
		try (PrintWriter w =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			out.println("# EXD robot back-half probe hop 2 (writer family decompiles)");
			di = new DecompInterface();
			di.setOptions(new DecompileOptions());
			di.toggleCCode(true);
			di.openProgram(currentProgram);

			// The spawn/respawn initializer candidate (hits nearly every
			// back-half offset incl. variant/kind/hp/armor/charges/pool).
			decompFn("spawn-init-A", 0x1ef61L, 800);
			// The other all-fields writer (kind @0xf6d5e, battery, pool,
			// charges, pod timer).
			decompFn("spawn-init-B", 0x1d9cdL, 800);
			// The robot_move twin (dir_byte/facing/anim + pos).
			decompFn("move", 0x1d274L, 600);
			// The move_is_possible twin (probe_z cache words +0x1A..+0x28).
			decompFn("probes", 0x1e440L, 600);
			// Weapon-group readers (the [EAX*8+0xf6d62]-form functions).
			decompFn("wgroup-A", 0x18e24L, 500);
			decompFn("wgroup-B", 0x180a1L, 700);
			decompFn("wgroup-C", 0x191a8L, 400);
			// The armor pad-charge twin (armor +20 behind the pool).
			decompFn("pad-charge", 0x20deaL, 400);
			// The death_flag writer candidate.
			decompFn("death-flag", 0x5961cL, 200);
			// Alarm word reader (FUN_000191a8 hits 0xf6d68) + the +0x32 word
			// twin (FUN_00020fd5).
			decompFn("word32-alarm", 0x20fd5L, 300);
			// The 14.6KB phase monolith head (context: which phase calls
			// which twin).
			decompFn("monolith-head", 0x1476dL, 260);

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
