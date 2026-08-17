/*-
 * ExportExwFollowup.java — second-pass targeted dump for BEDLAM.EXW main-loop
 * analysis. Answers, from the already-imported BedlamWatcom project:
 *   CALLERS   — who calls WinMain-equiv (0044d320), pump (0044d93c), startup
 *               (004520ed), timer-init (0044da64), and who reads the quit /
 *               mode flag globals (004ef690, 004ef692, 004ef69e, 004ef8fc).
 *   TIMER     — call sites of timeSetEvent/timeKillEvent/CreateThread/Sleep.
 *   DECOMP    — FUN_004520ed (Watcom startup), FUN_0044da64 (timeBeginPeriod).
 *   WNDPROC   — create function at LAB_0044dacc and decompile (message handler).
 *   SHUTDOWN  — create function at LAB_0044d6e8 and decompile (exit handler
 *               installed by the entry stub via _DAT_004ef8fc).
 *
 * Usage:
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExportExwFollowup.java <outputDir>
 *
 * Degrades gracefully: failures print SKIP lines. Outputs are analysis
 * artifacts in gitignored locations.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.ExternalLocation;
import ghidra.program.model.symbol.ExternalLocationIterator;
import ghidra.program.model.symbol.ExternalManager;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExportExwFollowup extends GhidraScript {

	private static final String[] CALLER_TARGETS = {
		"0044d320", "0044d93c", "004520ed", "0044da64",
		"004ef690", "004ef692", "004ef69e", "004ef8fc",
	};

	private static final String[] TIMER_IMPORTS = {
		"timeSetEvent", "timeKillEvent", "CreateThread", "Sleep", "WaitForSingleObject",
		"SuspendThread", "TerminateThread",
	};

	private static final String[] DECOMP_TARGETS = { "004520ed", "0044da64" };

	private static final String WNDPROC_ADDR = "0044dacc";
	private static final String SHUTDOWN_ADDR = "0044d6e8";

	private static final int DECOMP_TIMEOUT_SECS = 90;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outDir = Paths.get(args.length > 0 ? args[0] : ".");
		Files.createDirectories(outDir);
		println("ExportExwFollowup: output dir = " + outDir.toAbsolutePath());

		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(outDir.resolve("exw-followup.txt"),
				StandardCharsets.UTF_8))) {

			section(out, "CALLERS");
			for (String addrStr : CALLER_TARGETS) {
				Address target = parse(addrStr);
				if (target == null) {
					out.println("SKIP bad address " + addrStr);
					continue;
				}
				int n = 0;
				ReferenceIterator it =
					currentProgram.getReferenceManager().getReferencesTo(target);
				while (it.hasNext()) {
					Reference ref = it.next();
					Function fn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					out.println(target + "\tcallsite/use=" + ref.getFromAddress() +
						"\tfn=" + (fn != null ? fn.getEntryPoint() + " " + fn.getName()
							: "<no function>") +
						"\trefType=" + ref.getReferenceType());
					n++;
				}
				if (n == 0) {
					out.println(target + "\tNO REFERENCES");
				}
				println("ExportExwFollowup: callers of " + target + " = " + n);
			}

			section(out, "TIMER");
			ExternalManager extMgr = currentProgram.getExternalManager();
			for (String lib : extMgr.getExternalLibraryNames()) {
				ExternalLocationIterator it = extMgr.getExternalLocations(lib);
				while (it.hasNext()) {
					ExternalLocation loc = it.next();
					if (!isWanted(loc.getLabel())) {
						continue;
					}
					Address a = loc.getAddress();
					if (a == null) {
						continue;
					}
					ReferenceIterator ri =
						currentProgram.getReferenceManager().getReferencesTo(a);
					while (ri.hasNext()) {
						Reference ref = ri.next();
						Function fn = currentProgram.getFunctionManager()
							.getFunctionContaining(ref.getFromAddress());
						out.println(loc.getLabel() + "\tcallsite=" + ref.getFromAddress() +
							"\tfn=" + (fn != null ? fn.getEntryPoint() + " " + fn.getName()
								: "<no function>") +
							"\tisCall=" + ref.getReferenceType().isCall());
					}
				}
			}

			section(out, "DECOMP");
			DecompInterface decomp = newDecompiler();
			try {
				for (String addrStr : DECOMP_TARGETS) {
					Function fn = currentProgram.getFunctionManager()
						.getFunctionAt(parse(addrStr));
					if (fn == null) {
						out.println("SKIP no function at " + addrStr);
						continue;
					}
					decompileAndPrint(out, decomp, fn);
				}

				section(out, "WNDPROC");
				dumpCreateAndDecompile(out, decomp, WNDPROC_ADDR, "BedlamWndProc", 0x200);

				section(out, "SHUTDOWN");
				dumpCreateAndDecompile(out, decomp, SHUTDOWN_ADDR, "BedlamShutdown", 0x180);
			}
			finally {
				decomp.dispose();
			}
		}
		println("ExportExwFollowup: done.");
	}

	/** Creates a function at addr if missing, decompiles it, falls back to listing. */
	private void dumpCreateAndDecompile(PrintWriter out, DecompInterface decomp,
			String addrStr, String name, int listingLen) {
		Address addr = parse(addrStr);
		if (addr == null) {
			out.println("SKIP bad address " + addrStr);
			return;
		}
		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		if (fn == null) {
			try {
				fn = createFunction(addr, name);
			}
			catch (Exception e) {
				println("ExportExwFollowup: createFunction failed at " + addrStr + ": " + e);
			}
		}
		if (fn != null) {
			decompileAndPrint(out, decomp, fn);
			return;
		}
		out.println("NOTE no function at " + addrStr + "; raw listing follows");
		dumpListing(out, addr, listingLen);
	}

	private void dumpListing(PrintWriter out, Address start, int len) {
		InstructionIterator it = currentProgram.getListing().getInstructions(start, true);
		Address end = start.add(len);
		while (it.hasNext()) {
			Instruction inst = it.next();
			if (inst.getAddress().compareTo(end) >= 0) {
				break;
			}
			out.println(inst.getAddress() + "\t" + inst);
		}
	}

	private DecompInterface newDecompiler() {
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		return decomp;
	}

	private void decompileAndPrint(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		try {
			DecompileResults results =
				decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
			if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
				out.println(results.getDecompiledFunction().getC());
			}
			else {
				out.println("// DECOMP FAILED: " + results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
		}
	}

	private boolean isWanted(String label) {
		for (String wanted : TIMER_IMPORTS) {
			if (wanted.equals(label)) {
				return true;
			}
		}
		return false;
	}

	private Address parse(String addrStr) {
		try {
			return currentProgram.getAddressFactory().getAddress(addrStr);
		}
		catch (Exception e) {
			return null;
		}
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExportExwFollowup: section " + name);
	}
}
