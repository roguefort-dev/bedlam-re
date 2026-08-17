/*-
 * ExportExwMainLoop.java — headless export of BEDLAM.EXW function inventory and
 * main-loop analysis from the already-imported Ghidra project "BedlamWatcom".
 *
 * Purpose
 *   Produces two text dumps used to locate the Win32 message pump / main loop
 *   of Bedlam (1996, Watcom-compiled PE32 GUI binary, openwatcomcpp cspec):
 *     1. exw-functions.txt  — every function: entry, name, body size, inbound refs
 *     2. exw-mainloop.txt   — structured sections: INFO, IMPORTS, ANCHOR_XREFS,
 *                             ENTRY_FN, CANDIDATE_DECOMP, LISTING_PUMP_HINT
 *
 * Usage (postScript against the existing program; never re-import):
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExportExwMainLoop.java <outputDir>
 *
 *   <outputDir> is optional and defaults to ".".
 *
 * NOTE: outputs are analysis artifacts and land in gitignored locations
 * (e.g. ghidra-project/analysis/) — never commit them.
 * Every anchor/section failure degrades gracefully: a SKIP line is logged
 * instead of aborting the run.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.symbol.ExternalLocation;
import ghidra.program.model.symbol.ExternalLocationIterator;
import ghidra.program.model.symbol.ExternalManager;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.ReferenceManager;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;
import ghidra.program.model.symbol.SymbolTable;

public class ExportExwMainLoop extends GhidraScript {

	/** Anchors tracked for cross-references (A and W variants where they exist). */
	private static final String[] ANCHORS = {
		"GetMessageA", "GetMessageW", "PeekMessageA", "PeekMessageW",
		"TranslateMessage", "DispatchMessageA", "DispatchMessageW", "PostQuitMessage",
		"RegisterClassA", "RegisterClassExA", "CreateWindowExA", "ShowWindow",
		"UpdateWindow", "SetTimer", "KillTimer", "timeGetTime", "timeBeginPeriod",
		"timeEndPeriod", "QueryPerformanceCounter", "GetTickCount", "WinMain", "main",
		"WatcomStartUp", "__CrtStartUp", "_WinMain", "_crtstart_",
	};

	/** Anchors that mark the message pump itself (highest decompile priority). */
	private static final Set<String> PUMP_ANCHORS = Set.of(
		"GetMessageA", "GetMessageW", "PeekMessageA", "PeekMessageW");

	/** Anchors that mark the frame/timing loop (second priority). */
	private static final Set<String> TIMER_ANCHORS = Set.of("SetTimer", "timeGetTime");

	/** Anchors that make a function a decompile candidate at all. */
	private static final Set<String> CANDIDATE_ANCHORS = Set.of(
		"GetMessageA", "GetMessageW", "PeekMessageA", "PeekMessageW",
		"DispatchMessageA", "DispatchMessageW", "RegisterClassA", "RegisterClassExA",
		"CreateWindowExA", "SetTimer", "timeGetTime", "QueryPerformanceCounter");

	private static final int MAX_CANDIDATES = 12;
	private static final int DECOMP_TIMEOUT_SECS = 90;

	private ReferenceManager refMgr;
	private SymbolTable symTable;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExportExwMainLoop: output dir = " + outDir.toAbsolutePath());

		refMgr = currentProgram.getReferenceManager();
		symTable = currentProgram.getSymbolTable();

		writeFunctionsFile(outDir.resolve("exw-functions.txt"));

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-mainloop.txt"), StandardCharsets.UTF_8))) {
			writeMainloopFile(out);
		}
		println("ExportExwMainLoop: done.");
	}

	// ------------------------------------------------------------------
	// exw-functions.txt
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
			println("ExportExwMainLoop: wrote " + fns.size() + " functions to " + file);
		}
	}

	/** Number of references whose destination is the function's entry point. */
	private int countInboundRefs(Function fn) {
		int n = 0;
		ReferenceIterator it = refMgr.getReferencesTo(fn.getEntryPoint());
		while (it.hasNext()) {
			it.next();
			n++;
		}
		return n;
	}

	// ------------------------------------------------------------------
	// exw-mainloop.txt
	// ------------------------------------------------------------------

	private void writeMainloopFile(PrintWriter out) {
		writeInfoSection(out);

		Map<String, Set<Address>> extAnchorIndex = new LinkedHashMap<>();
		writeImportsSection(out, extAnchorIndex);

		Map<String, Set<Function>> anchorFunctions = new LinkedHashMap<>();
		Set<Function> pumpFunctions = new LinkedHashSet<>();
		writeAnchorXrefsSection(out, extAnchorIndex, anchorFunctions, pumpFunctions);

		Address entry = firstExternalEntryPoint();
		DecompInterface decomp = newDecompiler();
		try {
			writeEntryFnSection(out, decomp, entry);
			writeCandidateDecompSection(out, decomp, anchorFunctions, pumpFunctions);
			writeListingPumpHintSection(out, pumpFunctions);
		}
		finally {
			decomp.dispose();
		}
	}

	/** INFO: program identity plus entry points and address bounds. */
	private void writeInfoSection(PrintWriter out) {
		section(out, "INFO");
		out.println("program:        " + currentProgram.getName());
		out.println("imageBase:      " + currentProgram.getImageBase());
		out.println("language:       " + currentProgram.getLanguageID());
		out.println("compilerSpec:   " + currentProgram.getCompilerSpec().getCompilerSpecID());
		out.println("minAddress:     " + currentProgram.getMemory().getMinAddress());
		out.println("maxAddress:     " + currentProgram.getMemory().getMaxAddress());
		AddressIterator it = symTable.getExternalEntryPointIterator();
		Address first = null;
		while (it.hasNext()) {
			Address a = it.next();
			if (first == null) {
				first = a;
				out.println("entryPoint:     " + a);
			}
			out.println("entryPointAll:  " + a);
		}
		if (first == null) {
			out.println("entryPoint:     SKIP none found");
		}
	}

	/**
	 * IMPORTS: every external location, one per line. As a side effect builds an
	 * index from anchor label -> external addresses for the ANCHOR_XREFS section.
	 */
	private void writeImportsSection(PrintWriter out, Map<String, Set<Address>> extAnchorIndex) {
		section(out, "IMPORTS");
		try {
			ExternalManager extMgr = currentProgram.getExternalManager();
			int count = 0;
			for (String lib : extMgr.getExternalLibraryNames()) {
				ExternalLocationIterator it = extMgr.getExternalLocations(lib);
				while (it.hasNext()) {
					ExternalLocation loc = it.next();
					Address a = loc.getAddress();
					out.println(lib + "\t" + a + "\t" + loc.getLabel());
					count++;
					if (a != null && isAnchor(loc.getLabel())) {
						extAnchorIndex.computeIfAbsent(loc.getLabel(), k -> new LinkedHashSet<>())
							.add(a);
					}
				}
			}
			out.println("importCount: " + count);
			println("ExportExwMainLoop: imports = " + count);
		}
		catch (Exception e) {
			out.println("SKIP imports enumeration failed: " + e);
			println("ExportExwMainLoop: SKIP imports: " + e);
		}
	}

	/**
	 * ANCHOR_XREFS: for each anchor name, every reference to any symbol or
	 * external location carrying that name, with containing-function context.
	 */
	private void writeAnchorXrefsSection(PrintWriter out,
			Map<String, Set<Address>> extAnchorIndex, Map<String, Set<Function>> anchorFunctions,
			Set<Function> pumpFunctions) {
		section(out, "ANCHOR_XREFS");
		for (String name : ANCHORS) {
			Set<Address> targets = resolveAnchorAddresses(name, extAnchorIndex);
			if (targets.isEmpty()) {
				out.println("SKIP " + name + " (not found)");
				println("ExportExwMainLoop: SKIP anchor " + name + " (not found)");
				continue;
			}
			for (Address target : targets) {
				int n = 0;
				ReferenceIterator it = refMgr.getReferencesTo(target);
				while (it.hasNext()) {
					Reference ref = it.next();
					Function fn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					String fnEntry = fn != null ? fn.getEntryPoint().toString() : "?";
					String fnName = fn != null ? fn.getName() : "<no function>";
					out.println(name + "\t" + target + "\tcallsite=" + ref.getFromAddress() +
						"\tfn=" + fnEntry + "\t" + fnName + "\trefType=" +
						ref.getReferenceType() + "\tisCall=" +
						ref.getReferenceType().isCall());
					if (fn != null) {
						anchorFunctions.computeIfAbsent(name, k -> new LinkedHashSet<>()).add(fn);
						if (PUMP_ANCHORS.contains(name)) {
							pumpFunctions.add(fn);
						}
					}
					n++;
				}
				println("ExportExwMainLoop: anchor " + name + " @ " + target + " xrefs=" + n);
			}
		}
	}

	/**
	 * Collects candidate addresses for an anchor: external-location addresses plus
	 * any in-program symbol (e.g. a thunk) with that exact name.
	 */
	private Set<Address> resolveAnchorAddresses(String name,
			Map<String, Set<Address>> extAnchorIndex) {
		Set<Address> targets = new LinkedHashSet<>();
		Set<Address> ext = extAnchorIndex.get(name);
		if (ext != null) {
			targets.addAll(ext);
		}
		SymbolIterator si = symTable.getSymbols(name);
		while (si.hasNext()) {
			Symbol s = si.next();
			targets.add(s.getAddress());
		}
		return targets;
	}

	/** ENTRY_FN: decompiled C of the function containing the program entry point. */
	private void writeEntryFnSection(PrintWriter out, DecompInterface decomp, Address entry) {
		section(out, "ENTRY_FN");
		if (entry == null) {
			out.println("SKIP no external entry point in symbol table");
			return;
		}
		Function fn = currentProgram.getFunctionManager().getFunctionContaining(entry);
		if (fn == null) {
			out.println("SKIP no function contains entry point " + entry);
			println("ExportExwMainLoop: SKIP entry fn at " + entry);
			return;
		}
		decompileAndPrint(out, decomp, fn, null);
	}

	/**
	 * CANDIDATE_DECOMP: decompiles distinct functions referencing pump/timer/window
	 * anchors (capped), pump hitters first. Also emits a PSEUDOCALLS summary line.
	 */
	private void writeCandidateDecompSection(PrintWriter out, DecompInterface decomp,
			Map<String, Set<Function>> anchorFunctions, Set<Function> pumpFunctions) {
		section(out, "CANDIDATE_DECOMP");
		Map<Function, Integer> scored = new LinkedHashMap<>();
		for (Map.Entry<String, Set<Function>> e : anchorFunctions.entrySet()) {
			if (!CANDIDATE_ANCHORS.contains(e.getKey())) {
				continue;
			}
			for (Function fn : e.getValue()) {
				int score = pumpFunctions.contains(fn) ? 0
					: hitsTimerAnchor(fn, anchorFunctions) ? 1 : 2;
				scored.merge(fn, score, Math::min);
			}
		}
		List<Function> ordered = new ArrayList<>(scored.keySet());
		ordered.sort(Comparator.comparing((Function f) -> scored.get(f))
			.thenComparing(Function::getEntryPoint));
		if (ordered.isEmpty()) {
			out.println("SKIP no functions referenced any candidate anchor");
			println("ExportExwMainLoop: SKIP candidate decomp (no anchors hit)");
			return;
		}
		if (ordered.size() > MAX_CANDIDATES) {
			out.println("NOTE truncating " + ordered.size() + " candidates to " + MAX_CANDIDATES);
			ordered = ordered.subList(0, MAX_CANDIDATES);
		}
		for (Function fn : ordered) {
			decompileAndPrint(out, decomp, fn, scored.get(fn));
		}
	}

	/** True if the function was seen calling SetTimer or timeGetTime. */
	private boolean hitsTimerAnchor(Function fn, Map<String, Set<Function>> anchorFunctions) {
		for (String anchor : TIMER_ANCHORS) {
			Set<Function> hitters = anchorFunctions.get(anchor);
			if (hitters != null && hitters.contains(fn)) {
				return true;
			}
		}
		return false;
	}

	/** LISTING_PUMP_HINT: raw listing of the first GetMessage/PeekMessage caller. */
	private void writeListingPumpHintSection(PrintWriter out, Set<Function> pumpFunctions) {
		section(out, "LISTING_PUMP_HINT");
		if (pumpFunctions.isEmpty()) {
			out.println("SKIP no function calls GetMessage*/PeekMessage*");
			return;
		}
		Function fn = pumpFunctions.stream()
			.min(Comparator.comparing(Function::getEntryPoint)).get();
		out.println("function: " + fn.getEntryPoint() + " " + fn.getName());
		InstructionIterator it =
			currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			Instruction inst = it.next();
			String bytes;
			try {
				bytes = toHex(inst.getBytes());
			}
			catch (MemoryAccessException e) {
				bytes = "??";
			}
			out.println(inst.getAddress() + "\t" + bytes + "\t" + inst);
		}
		println("ExportExwMainLoop: listed pump candidate " + fn.getName());
	}

	// ------------------------------------------------------------------
	// Decompilation helpers
	// ------------------------------------------------------------------

	/** Creates and opens a decompiler with default options and C output enabled. */
	private DecompInterface newDecompiler() {
		DecompInterface decomp = new DecompInterface();
		DecompileOptions options = new DecompileOptions();
		decomp.setOptions(options);
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		return decomp;
	}

	/**
	 * Prints the DECOMP header (plus optional priority tag), a PSEUDOCALLS summary,
	 * then the decompiled C body; failures degrade to an inline SKIP-style note.
	 */
	private void decompileAndPrint(PrintWriter out, DecompInterface decomp, Function fn,
			Integer priority) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() +
			(priority != null ? " priority=" + priority : "") + " -----");
		out.println("PSEUDOCALLS: " + pseudoCalls(fn));
		try {
			DecompileResults results =
				decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
			if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
				out.println(results.getDecompiledFunction().getC());
			}
			else {
				out.println("// DECOMP FAILED: " + results.getErrorMessage());
				println("ExportExwMainLoop: decomp failed for " + fn.getName() + ": " +
					results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExportExwMainLoop: decomp exception for " + fn.getName() + ": " + e);
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
				Address to = ref.getToAddress();
				targets.add(describeTarget(to));
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
		println("ExportExwMainLoop: section " + name);
	}

	/** True if the label matches one of the hardcoded anchors. */
	private boolean isAnchor(String label) {
		for (String anchor : ANCHORS) {
			if (anchor.equals(label)) {
				return true;
			}
		}
		return false;
	}

	/** First external entry point declared by the loader, or null. */
	private Address firstExternalEntryPoint() {
		AddressIterator it = symTable.getExternalEntryPointIterator();
		return it.hasNext() ? it.next() : null;
	}

	/** Lowercase hex rendering of an instruction's bytes. */
	private String toHex(byte[] bytes) {
		StringBuilder sb = new StringBuilder(bytes.length * 3);
		for (byte b : bytes) {
			if (!sb.isEmpty()) {
				sb.append(' ');
			}
			sb.append(String.format("%02x", b));
		}
		return sb.toString();
	}
}
