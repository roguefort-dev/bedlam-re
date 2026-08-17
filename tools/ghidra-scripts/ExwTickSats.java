/*-
 * ExwTickSats.java - dump pass for the remaining unnamed service-tick
 * satellites of BEDLAM.EXW (project BedlamWatcom, NEVER re-import).
 * Scope = RE-EXW-TICK.md callees still carrying FUN_ names:
 *   tick body       : 00402b0c (counters+fade fire), 00425ab9 (scroll step),
 *                     0041d714 (palette index apply)
 *   ddraw/palette   : 0044b040 (get current palette), 0044b7b0 (lost/restore),
 *                     0044bbac / 0044bb84 / 0044bc90 / 0044bcf4 (prepare+commit)
 *   screenshot      : 0044d1f2 (filename builder)
 *   stdio layer     : 0044e729 0044e815 0044f34b 0044e30b 0044e217 0044ea0b
 *   misc            : 0044ac5c (surface Lock candidate), 0044f237 (alloc)
 *   trampoline      : 00451fbc (Watcom CRT thread start; CREATE if missing)
 * For each: decompile + full listing + call targets. Dump-only, no renames.
 * Output: <outDir>/exw-tick-sats.txt
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.SymbolTable;

public class ExwTickSats extends GhidraScript {

	private static final String[] DUMP_FNS = {
		"00402b0c", "00425ab9", "0041d714", "0044b040", "0044b7b0",
		"0044bbac", "0044bb84", "0044bc90", "0044bcf4", "0044d1f2",
		"0044e729", "0044e815", "0044f34b", "0044e30b", "0044e217",
		"0044ea0b", "0044ac5c", "0044f237", "00451fbc",
	};

	/** create-as-function if missing (not yet functions in the project). */
	private static final String[] ENSURE_FNS = { "00451fbc" };

	private static final int DECOMP_TIMEOUT_SECS = 120;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwTickSats: output dir = " + outDir.toAbsolutePath());

		// Ensure requested functions exist (create + disassemble, no rename).
		for (String addrStr : ENSURE_FNS) {
			Address addr = parse(addrStr);
			if (addr == null) {
				continue;
			}
			Function existing = currentProgram.getFunctionManager().getFunctionAt(addr);
			if (existing != null) {
				println("ensure: " + addrStr + " already a function (" + existing.getName() + ")");
				continue;
			}
			try {
				if (currentProgram.getListing().getInstructionAt(addr) == null) {
					disassemble(addr);
				}
				Function created = createFunction(addr, "FUN_" + addrStr);
				println("ensure: " + addrStr + " createFunction -> " + created);
			}
			catch (Exception e) {
				println("ensure: " + addrStr + " FAILED: " + e);
			}
		}

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-tick-sats.txt"), StandardCharsets.UTF_8))) {
			out.println("program: " + currentProgram.getName());
			DecompInterface decomp = new DecompInterface();
			decomp.setOptions(new DecompileOptions());
			decomp.toggleCCode(true);
			decomp.openProgram(currentProgram);
			for (String addrStr : DUMP_FNS) {
				out.println();
				out.println("===== " + addrStr + " =====");
				Function fn = currentProgram.getFunctionManager().getFunctionAt(parse(addrStr));
				if (fn == null) {
					out.println("SKIP no function at " + addrStr);
					continue;
				}
				out.println("function: " + fn.getEntryPoint() + " " + fn.getName() +
					" (" + fn.getBody().getNumAddresses() + " bytes)");
				out.println("calls: " + pseudoCalls(fn));
				try {
					DecompileResults results = decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
					out.println(results.decompileCompleted() && results.getDecompiledFunction() != null
						? results.getDecompiledFunction().getC()
						: "// DECOMP FAILED: " + results.getErrorMessage());
				}
				catch (Exception e) {
					out.println("// DECOMP FAILED: " + e);
				}
				out.println("--- listing ---");
				InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
				while (it.hasNext()) {
					Instruction inst = it.next();
					out.println(inst.getAddress() + "\t" + inst.toString());
					for (Reference ref : inst.getReferencesFrom()) {
						if (ref.getReferenceType().isCall()) {
							out.println("    ; calls -> " + ref.getToAddress());
						}
					}
				}
			}
			decomp.dispose();
			out.println();
			out.println("done.");
		}
		println("ExwTickSats: done.");
	}

	private String pseudoCalls(Function fn) {
		List<String> targets = new ArrayList<>();
		InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			Instruction inst = it.next();
			for (Reference ref : inst.getReferencesFrom()) {
				if (!ref.getReferenceType().isCall()) {
					continue;
				}
				Address to = ref.getToAddress();
				Function f = to != null ? currentProgram.getFunctionManager().getFunctionAt(to) : null;
				String desc = f != null ? f.getName() : String.valueOf(to);
				if (!targets.contains(desc)) {
					targets.add(desc);
				}
			}
		}
		return String.join(";", targets);
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
