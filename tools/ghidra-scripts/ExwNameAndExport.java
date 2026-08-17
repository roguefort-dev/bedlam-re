/*-
 * ExwNameAndExport.java — finalize pass for BEDLAM.EXW in the already-imported
 * Ghidra project "BedlamWatcom": apply semantic names to the resolved startup
 * chain, promote the one remaining function (timer callback), decompile the
 * remaining thread-body candidates, and re-export the function inventory.
 *
 * Outputs (in the optional output-dir argument, default "."):
 *   1. exw-functions.txt — same inventory format as ExportExwMainLoop.java
 *      (header: program, imageBase, languageID, compilerSpecID, functionCount;
 *       rows: entry<TAB>name<TAB>bodySizeBytes<TAB>inboundRefs, address-sorted),
 *      regenerated AFTER naming so the new names are reflected.
 *   2. exw-finalize.txt — INFO, NAMES, DECOMP sections.
 *
 * Naming plan (addresses anchored in exw-followup.txt / prior analysis):
 *   0044d6e8 -> WinMain           (currently misnamed BedlamShutdown; it is the
 *                                   Watcom WinMain called via global 004ef8fc)
 *   0044d320 -> InitInstance      (window class + CreateWindowExA + DirectDraw)
 *   0044d93c -> MsgPump           (PeekMessage/GetMessage/DispatchMessage loop)
 *   0044d9c0 -> GameThreadStart   (suspected CreateThread wrapper, ~61 bytes)
 *   0044da64 -> TimerInit         (spin-wait, timeBeginPeriod, timeSetEvent)
 *   0044de58 -> TimerCallback     (timeSetEvent periodic callback; likely NOT
 *                                   yet a function — disassemble + create)
 *   004520ed -> WatcomCrtStartup
 *   0044dacc    keep BedlamWndProc (no rename)
 *
 * Usage (postScript against the existing program; never re-import):
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExwNameAndExport.java <outputDir>
 *
 * Program modifications (renames / created functions) persist in the project
 * via the headless -process run. NOTE: outputs are analysis artifacts and
 * belong in gitignored locations (e.g. ghidra-project/analysis/) — never
 * commit them. Every naming/decomp failure degrades gracefully: a SKIP line
 * is logged instead of aborting the run.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwNameAndExport extends GhidraScript {

	/** { address, new name, rationale note } — applied in this order. */
	private static final String[][] NAMES = {
		{ "0044d6e8", "WinMain", "Watcom WinMain, called via global 004ef8fc (was misnamed BedlamShutdown)" },
		{ "0044d320", "InitInstance", "window class + CreateWindowExA + DirectDraw init" },
		{ "0044d93c", "MsgPump", "PeekMessage/GetMessage/DispatchMessage loop" },
		{ "0044d9c0", "GameThreadStart", "suspected CreateThread wrapper (~61 bytes)" },
		{ "0044da64", "TimerInit", "spin-wait, timeBeginPeriod, timeSetEvent" },
		{ "0044de58", "TimerCallback", "timeSetEvent periodic callback (likely not yet a function; created here)" },
		{ "004520ed", "WatcomCrtStartup", "Watcom CRT startup" },
	};

	/** Function kept as-is; no rename requested. */
	private static final String WNDPROC_ADDR = "0044dacc";
	private static final String WNDPROC_NAME = "BedlamWndProc";

	/** { address, label, note } — decompiled in this order. */
	private static final String[][] DECOMP_TARGETS = {
		{ "0044d9c0", "GameThreadStart", "suspected CreateThread wrapper" },
		{ "0044de58", "TimerCallback", "timeSetEvent periodic callback (promoted above)" },
		{ "0044a9ac", "FUN_0044a9ac", "thread body candidate, called from InitInstance right before _SmackSoundUseDirectSound" },
	};

	private static final int DECOMP_TIMEOUT_SECS = 90;

	private SymbolTable symTable;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwNameAndExport: output dir = " + outDir.toAbsolutePath());

		symTable = currentProgram.getSymbolTable();

		// NAME first so both output files reflect the new symbol table state.
		int countBefore = countFunctions();
		List<String> nameLog = new ArrayList<>();
		for (String[] entry : NAMES) {
			nameFunction(nameLog, entry[0], entry[1], entry[2]);
		}
		logWndprocKept(nameLog);
		int countAfter = countFunctions();

		// Inventory re-export first: cheap, and survives any decomp hiccup.
		writeFunctionsFile(outDir.resolve("exw-functions.txt"));

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-finalize.txt"), StandardCharsets.UTF_8))) {
			writeInfoSection(out, countBefore, countAfter);
			writeNamesSection(out, nameLog);
			writeDecompSection(out);
		}
		println("ExwNameAndExport: done.");
	}

	// ------------------------------------------------------------------
	// exw-functions.txt (same format as ExportExwMainLoop.java)
	// ------------------------------------------------------------------

	/** Writes the flat function inventory (already address-sorted by the iterator). */
	private void writeFunctionsFile(Path file) throws Exception {
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(file, StandardCharsets.UTF_8))) {
			List<Function> fns = new ArrayList<>();
			FunctionIterator it = currentProgram.getFunctionManager().getFunctions(true);
			while (it.hasNext()) {
				fns.add(it.next());
			}
			out.println("# program:        " + currentProgram.getName());
			out.println("# imageBase:      " + currentProgram.getImageBase());
			out.println("# languageID:     " + currentProgram.getLanguageID());
			out.println("# compilerSpecID: " +
				currentProgram.getCompilerSpec().getCompilerSpecID());
			out.println("# functionCount:  " + fns.size());
			out.println("# columns: entry<TAB>name<TAB>bodySizeBytes<TAB>inboundRefs");
			for (Function fn : fns) {
				out.println(fn.getEntryPoint() + "\t" + fn.getName() + "\t" +
					fn.getBody().getNumAddresses() + "\t" + countInboundRefs(fn));
			}
			println("ExwNameAndExport: wrote " + fns.size() + " functions to " + file);
		}
	}

	/** Number of references whose destination is the function's entry point. */
	private int countInboundRefs(Function fn) {
		int n = 0;
		ReferenceIterator it =
			currentProgram.getReferenceManager().getReferencesTo(fn.getEntryPoint());
		while (it.hasNext()) {
			it.next();
			n++;
		}
		return n;
	}

	/** Total function count in the program. */
	private int countFunctions() {
		int n = 0;
		FunctionIterator it = currentProgram.getFunctionManager().getFunctions(true);
		while (it.hasNext()) {
			it.next();
			n++;
		}
		return n;
	}

	// ------------------------------------------------------------------
	// Naming
	// ------------------------------------------------------------------

	/**
	 * Ensures the function at addrStr carries newName: renames an existing
	 * function (setName SourceType.USER_DEFINED, logging old->new) or, when no
	 * function exists there, promotes the address (disassemble if needed, then
	 * createFunction(addr, newName) from FlatProgramAPI). Appends exactly one
	 * log line describing the outcome; never aborts the run.
	 */
	private void nameFunction(List<String> log, String addrStr, String newName, String note) {
		Address addr = parse(addrStr);
		if (addr == null) {
			log.add(addrStr + "\t" + newName + "\tSKIP bad address\t" + note);
			println("ExwNameAndExport: SKIP " + addrStr + " (bad address)");
			return;
		}

		// Promotion prerequisite: an instruction must exist (e.g. TimerCallback).
		try {
			if (currentProgram.getListing().getInstructionAt(addr) == null) {
				disassemble(addr); // FlatProgramAPI; return value unneeded
				if (currentProgram.getListing().getInstructionAt(addr) == null) {
					log.add(addr + "\t" + newName +
						"\tSKIP disassemble produced no instruction\t" + note);
					println("ExwNameAndExport: SKIP " + addr + " (no instruction)");
					return;
				}
			}
		}
		catch (Exception e) {
			log.add(addr + "\t" + newName + "\tSKIP disassemble failed: " + e + "\t" + note);
			println("ExwNameAndExport: SKIP disassemble " + addr + ": " + e);
			return;
		}

		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		if (fn != null) {
			renameFunction(log, addr, fn, newName, note);
			return;
		}

		Function containing = currentProgram.getFunctionManager().getFunctionContaining(addr);
		if (containing != null) {
			log.add(addr + "\t" + newName + "\tSKIP inside existing function " +
				containing.getEntryPoint() + " " + containing.getName() + "\t" + note);
			println("ExwNameAndExport: SKIP " + addr + " (inside " + containing.getName() + ")");
			return;
		}

		try {
			fn = createFunction(addr, newName); // FlatProgramAPI
		}
		catch (Exception e) {
			log.add(addr + "\t" + newName + "\tSKIP createFunction failed: " + e + "\t" + note);
			println("ExwNameAndExport: SKIP createFunction " + addr + ": " + e);
			return;
		}
		if (fn == null) {
			log.add(addr + "\t" + newName + "\tSKIP createFunction returned null\t" + note);
			println("ExwNameAndExport: SKIP createFunction " + addr + " (null)");
			return;
		}
		String createdAs = fn.getName();
		if (!createdAs.equals(newName)) {
			// A name collision forced a suffixed name; try to force the target.
			try {
				fn.setName(newName, SourceType.USER_DEFINED);
				log.add(addr + "\tcreated " + createdAs + " -> " + newName +
					"\tCREATED+RENAMED\t" + note);
			}
			catch (Exception e) {
				log.add(addr + "\tcreated as " + createdAs + "\tSKIP setName failed: " + e +
					"\t" + note);
				println("ExwNameAndExport: SKIP rename " + addr + " after create: " + e);
				return;
			}
		}
		else {
			log.add(addr + "\t" + newName + "\tCREATED\t" + note);
		}
		println("ExwNameAndExport: created " + newName + " at " + addr);
	}

	/** Renames an existing function, logging old->new; SKIP on any failure. */
	private void renameFunction(List<String> log, Address addr, Function fn, String newName,
			String note) {
		String oldName = fn.getName();
		if (oldName.equals(newName)) {
			log.add(addr + "\t" + newName + "\tALREADY NAMED\t" + note);
			println("ExwNameAndExport: " + addr + " already named " + newName);
			return;
		}
		try {
			fn.setName(newName, SourceType.USER_DEFINED);
			log.add(addr + "\t" + oldName + " -> " + newName + "\tRENAMED\t" + note);
			println("ExwNameAndExport: " + addr + " " + oldName + " -> " + newName);
		}
		catch (Exception e) {
			log.add(addr + "\t" + oldName + " -> " + newName + "\tSKIP setName failed: " + e +
				"\t" + note);
			println("ExwNameAndExport: SKIP rename " + addr + ": " + e);
		}
	}

	/** 0044dacc keeps its BedlamWndProc name; no rename requested. */
	private void logWndprocKept(List<String> log) {
		Address addr = parse(WNDPROC_ADDR);
		Function fn = addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr)
				: null;
		if (fn == null) {
			log.add(WNDPROC_ADDR + "\t" + WNDPROC_NAME +
				"\tSKIP no function at address (expected " + WNDPROC_NAME + ")\twindow procedure");
			println("ExwNameAndExport: SKIP " + WNDPROC_ADDR + " (no function)");
			return;
		}
		String cur = fn.getName();
		String note = WNDPROC_NAME.equals(cur) ? "window procedure"
				: "window procedure (expected " + WNDPROC_NAME + ", found " + cur + ")";
		log.add(WNDPROC_ADDR + "\t" + cur + "\tSKIP kept (no rename requested)\t" + note);
		println("ExwNameAndExport: " + WNDPROC_ADDR + " kept as " + cur);
	}

	// ------------------------------------------------------------------
	// exw-finalize.txt sections
	// ------------------------------------------------------------------

	/** INFO: program identity plus function counts before/after naming. */
	private void writeInfoSection(PrintWriter out, int countBefore, int countAfter) {
		section(out, "INFO");
		out.println("program:             " + currentProgram.getName());
		out.println("imageBase:           " + currentProgram.getImageBase());
		out.println("language:            " + currentProgram.getLanguageID());
		out.println("compilerSpec:        " +
			currentProgram.getCompilerSpec().getCompilerSpecID());
		out.println("functionCountBefore: " + countBefore);
		out.println("functionCountAfter:  " + countAfter);
		out.println("namesRequested:      " + NAMES.length);
	}

	/** NAMES: one line per naming action, in application order. */
	private void writeNamesSection(PrintWriter out, List<String> log) {
		section(out, "NAMES");
		out.println("# columns: address<TAB>old -> new (or name)<TAB>status<TAB>note");
		for (String line : log) {
			out.println(line);
		}
	}

	/**
	 * DECOMP: ordered decompilation (C body + PSEUDOCALLS summary per entry,
	 * exactly like the prior export scripts). Only exact entry-point matches
	 * are decompiled; anything else degrades to a SKIP line.
	 */
	private void writeDecompSection(PrintWriter out) {
		section(out, "DECOMP");
		DecompInterface decomp = newDecompiler();
		try {
			for (String[] target : DECOMP_TARGETS) {
				Address addr = parse(target[0]);
				Function fn = addr != null
						? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
				if (fn == null) {
					out.println("SKIP " + target[0] + " (" + target[1] +
						") no function entry at address\t" + target[2]);
					println("ExwNameAndExport: SKIP decomp " + target[0] + " (no function)");
					continue;
				}
				out.println("// note: " + target[2]);
				decompileAndPrint(out, decomp, fn);
			}
		}
		finally {
			decomp.dispose();
		}
	}

	// ------------------------------------------------------------------
	// Decompilation helpers (as in ExportExwMainLoop / ExwLoopFollowup)
	// ------------------------------------------------------------------

	/** Creates and opens a decompiler with default options and C output enabled. */
	private DecompInterface newDecompiler() {
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		return decomp;
	}

	/**
	 * Prints the DECOMP header, a PSEUDOCALLS summary, then the decompiled C
	 * body; failures degrade to an inline SKIP-style note.
	 */
	private void decompileAndPrint(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		out.println("PSEUDOCALLS: " + pseudoCalls(fn));
		try {
			DecompileResults results =
				decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
			if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
				out.println(results.getDecompiledFunction().getC());
			}
			else {
				out.println("// DECOMP FAILED: " + results.getErrorMessage());
				println("ExwNameAndExport: decomp failed for " + fn.getName() + ": " +
					results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExwNameAndExport: decomp exception for " + fn.getName() + ": " + e);
		}
	}

	/** Semicolon-separated targets of every call instruction in the function body. */
	private String pseudoCalls(Function fn) {
		Set<String> targets = new TreeSet<>();
		InstructionIterator it =
			currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			Instruction inst = it.next();
			for (Reference ref : inst.getReferencesFrom()) {
				if (!ref.getReferenceType().isCall()) {
					continue;
				}
				targets.add(describeTarget(ref.getToAddress()));
			}
		}
		return String.join(";", targets);
	}

	/** Resolves a call target to a function or symbol name, else its address. */
	private String describeTarget(Address to) {
		if (to == null) {
			return "<indirect>";
		}
		Function f = currentProgram.getFunctionManager().getFunctionAt(to);
		if (f != null) {
			return f.getName();
		}
		Symbol s = symTable.getPrimarySymbol(to);
		if (s != null) {
			return s.getName();
		}
		return to.toString();
	}

	// ------------------------------------------------------------------
	// Small utilities
	// ------------------------------------------------------------------

	/** Emits a section marker line. */
	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwNameAndExport: section " + name);
	}

	/** Parses a hex address string against the current program, or null. */
	private Address parse(String addrStr) {
		try {
			return currentProgram.getAddressFactory().getAddress(addrStr);
		}
		catch (Exception e) {
			return null;
		}
	}
}
