/*-
 * ExwGameMainHop.java - GameMain second hop for BEDLAM.EXW in the already
 * imported Ghidra project "BedlamWatcom". NEVER re-import (AGENTS.md rule).
 *
 * Targets (.state/NEXT.md top task, docs/RE-EXW-GAMETHREAD.md open list 2/5):
 *   1. FUN_0043d00b - gameplay advance (per-frame sim/render?); its 004ede10
 *      read is a fade-status check; find the REAL rate mechanism inside.
 *   2. FUN_00440e45 - zone/level manager (returns quit status).
 *   3. FUN_00448ef1 - reads divider 004edbc8 four times (pacer candidate).
 *   4. FUN_00402b48 - called every 100Hz tick by FUN_00402b0c.
 *   5. FUN_00402965 - called everywhere [hyp: yield/commit].
 *   Plus: xref census (readers!) of the five 100Hz tick counters, and a
 *   program-wide caller census of wait-capable imports (Sleep, timeGetTime,
 *   WaitForSingleObject, GetTickCount, PeekMessage, GetMessage) to settle
 *   whether ANY wait exists on the game thread.
 *
 * Output: <outDir>/exw-gamemainhop.txt (analysis artifact, gitignored).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

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

public class ExwGameMainHop extends GhidraScript {

	/** { address, name, note } - named then decompiled, in this order. */
	private static final String[][] STATIC_TARGETS = {
		{ "0043d00b", "FUN_0043d00b", "gameplay advance; 004ede10 read = fade status" },
		{ "00440e45", "FUN_00440e45", "zone/level manager" },
		{ "00448ef1", "FUN_00448ef1", "divider 004edbc8 consumer x4 (pacer candidate)" },
		{ "00402b48", "FUN_00402b48", "called every 100Hz tick by FUN_00402b0c" },
		{ "00402965", "FUN_00402965", "called everywhere [hyp yield/commit]" },
	};

	/** Direct callees of these roots are auto-decompiled (one level). */
	private static final String[] AUTO_CALLEE_ROOTS = { "0043d00b", "00448ef1" };

	/** Listing dumps (exact instructions for the small helpers + the advance). */
	private static final String[] LISTING_TARGETS =
		{ "00402965", "00402b48", "0043d00b", "00448ef1" };

	/** The five 100Hz tick counters - reader census reveals rate consumers. */
	private static final String[] XREF_GLOBALS = {
		"004edb84", "004edbc8", "004edbcc", "004edba4", "004edba8",
	};

	/** Wait-capable imports: any caller anywhere on the game thread? */
	private static final String[] IMPORT_NAMES = {
		"Sleep", "timeGetTime", "GetTickCount", "WaitForSingleObject",
		"WaitForMultipleObjects", "MsgWaitForMultipleObjects",
		"PeekMessageA", "GetMessageA",
	};

	private static final int DECOMP_TIMEOUT_SECS = 120;
	private static final int MAX_AUTO_CALLEES = 24;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwGameMainHop: output dir = " + outDir.toAbsolutePath());

		Map<String, Function> targets = new LinkedHashMap<>();
		List<String> nameLog = new ArrayList<>();
		for (String[] entry : STATIC_TARGETS) {
			Function fn = nameFunction(nameLog, entry[0], entry[1], entry[2]);
			if (fn != null) {
				targets.put(entry[0], fn);
			}
		}

		List<Function> autoTargets = collectAutoCallees(targets);

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-gamemainhop.txt"), StandardCharsets.UTF_8))) {
			section(out, "INFO");
			out.println("program:       " + currentProgram.getName());
			out.println("imageBase:     " + currentProgram.getImageBase());
			out.println("compilerSpec:  " + currentProgram.getCompilerSpec().getCompilerSpecID());
			section(out, "NAMES");
			out.println("# columns: address<TAB>action<TAB>note");
			for (String line : nameLog) {
				out.println(line);
			}
			section(out, "DECOMP");
			DecompInterface decomp = newDecompiler();
			try {
				for (Function fn : targets.values()) {
					decompileAndPrint(out, decomp, fn);
				}
				if (!autoTargets.isEmpty()) {
					out.println("// ---- auto-discovered callees (one level deep) ----");
					for (Function fn : autoTargets) {
						decompileAndPrint(out, decomp, fn);
					}
}
			}
			finally {
				decomp.dispose();
			}
			section(out, "LISTING");
			for (String addrStr : LISTING_TARGETS) {
				writeListing(out, addrStr);
			}
			section(out, "XREFS_COUNTERS");
			for (String addrStr : XREF_GLOBALS) {
				writeXrefs(out, addrStr);
			}
			section(out, "IMPORT_CALLERS");
			writeImportCallers(out);
		}
		println("ExwGameMainHop: done.");
	}

	// ------------------------------------------------------------------
	// Naming (graceful: SKIP lines, never aborts)
	// ------------------------------------------------------------------

	private Function nameFunction(List<String> log, String addrStr, String newName, String note) {
		Address addr = parse(addrStr);
		if (addr == null) {
			log.add(addrStr + "\tSKIP bad address\t" + note);
			return null;
		}
		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		if (fn == null) {
			try {
				if (currentProgram.getListing().getInstructionAt(addr) == null) {
					disassemble(addr);
				}
				fn = createFunction(addr, newName);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP create failed: " + e + "\t" + note);
				println("ExwGameMainHop: SKIP " + addr + ": " + e);
				return null;
			}
			if (fn == null) {
				log.add(addr + "\tSKIP createFunction null\t" + note);
				return null;
			}
		}
		String oldName = fn.getName();
		if (!oldName.equals(newName)) {
			try {
				fn.setName(newName, SourceType.USER_DEFINED);
				log.add(addr + "\t" + oldName + " -> " + newName + "\t" + note);
				println("ExwGameMainHop: " + addr + " " + oldName + " -> " + newName);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP rename failed: " + e + "\t" + note);
				println("ExwGameMainHop: SKIP rename " + addr + ": " + e);
			}
		}
		else {
			log.add(addr + "\tALREADY " + newName + "\t" + note);
		}
		return fn;
	}

	// ------------------------------------------------------------------
	// Auto callees
	// ------------------------------------------------------------------

	private List<Function> collectAutoCallees(Map<String, Function> roots) {
		Map<String, Function> found = new LinkedHashMap<>();
		for (String rootAddr : AUTO_CALLEE_ROOTS) {
			Function root = roots.get(rootAddr);
			if (root == null) {
				continue;
			}
			InstructionIterator it = currentProgram.getListing().getInstructions(root.getBody(), true);
			while (it.hasNext()) {
				Instruction inst = it.next();
				for (Reference ref : inst.getReferencesFrom()) {
					if (!ref.getReferenceType().isCall()) {
						continue;
					}
					Address to = ref.getToAddress();
					if (to == null) {
						continue;
					}
					String key = to.toString();
					if (roots.containsKey(key) || found.containsKey(key)) {
						continue;
					}
					Function callee = currentProgram.getFunctionManager().getFunctionAt(to);
					if (callee != null && callee.getBody().getNumAddresses() <= 4096) {
						found.put(key, callee);
					}
				}
			}
		}
		List<Function> ordered = new ArrayList<>(found.values());
		if (ordered.size() > MAX_AUTO_CALLEES) {
			ordered = ordered.subList(0, MAX_AUTO_CALLEES);
		}
		println("ExwGameMainHop: auto callees = " + ordered.size());
		return ordered;
	}

	// ------------------------------------------------------------------
	// Import caller census
	// ------------------------------------------------------------------

	private void writeImportCallers(PrintWriter out) {
		Set<String> wanted = new LinkedHashSet<>();
		for (String n : IMPORT_NAMES) {
			wanted.add(n);
		}
		FunctionIterator fit = currentProgram.getFunctionManager().getFunctions(true);
		List<Function> imports = new ArrayList<>();
		while (fit.hasNext()) {
			Function fn = fit.next();
			if (wanted.contains(fn.getName())) {
				imports.add(fn);
			}
		}
		if (imports.isEmpty()) {
			out.println("(no functions matching: " + String.join(",", IMPORT_NAMES) + ")");
		}
		for (Function imp : imports) {
			out.println("import: " + imp.getName() + " @ " + imp.getEntryPoint() +
				(imp.isThunk() ? " [thunk]" : ""));
			ReferenceIterator it =
				currentProgram.getReferenceManager().getReferencesTo(imp.getEntryPoint());
			boolean any = false;
			while (it.hasNext()) {
				Reference ref = it.next();
				Function fromFn =
					currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
				String from = fromFn != null
						? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
				out.println("  " + ref.getFromAddress() + "\t" + from + "\t" + ref.getReferenceType());
				any = true;
			}
			if (!any) {
				out.println("  <no callers>");
			}
		}
	}

	// ------------------------------------------------------------------
	// Decompilation
	// ------------------------------------------------------------------

	private DecompInterface newDecompiler() {
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		return decomp;
	}

	private void decompileAndPrint(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		out.println("PSEUDOCALLS: " + pseudoCalls(fn));
		try {
			DecompileResults results = decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
			if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
				out.println(results.getDecompiledFunction().getC());
			}
			else {
				out.println("// DECOMP FAILED: " + results.getErrorMessage());
				println("ExwGameMainHop: decomp failed for " + fn.getName() + ": " +
					results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExwGameMainHop: decomp exception for " + fn.getName() + ": " + e);
		}
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
				targets.add(describeTarget(ref.getToAddress()));
			}
		}
		return String.join(";", targets);
	}

	private String describeTarget(Address to) {
		if (to == null) {
			return "<indirect>";
		}
		Function f = currentProgram.getFunctionManager().getFunctionAt(to);
		if (f != null) {
			return f.getName();
		}
		return to.toString();
	}

	// ------------------------------------------------------------------
	// Listing / Xrefs
	// ------------------------------------------------------------------

	private void writeListing(PrintWriter out, String addrStr) {
		Address addr = parse(addrStr);
		Function fn = addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
		if (fn == null) {
			out.println("SKIP " + addrStr + " no function at address");
			return;
		}
		out.println("function: " + fn.getEntryPoint() + " " + fn.getName() +
			" (" + fn.getBody().getNumAddresses() + " bytes)");
		InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			out.println(it.next().toString());
		}
	}

	private void writeXrefs(PrintWriter out, String addrStr) {
		Address addr = parse(addrStr);
		if (addr == null) {
			out.println("SKIP " + addrStr + " bad address");
			return;
		}
		out.println("global: " + addr);
		ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(addr);
		boolean any = false;
		while (it.hasNext()) {
			Reference ref = it.next();
			Function fromFn = currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
			String from = fromFn != null ? fromFn.getName() : "<no function>";
			out.println("  " + ref.getFromAddress() + "\t" + from + "\t" + ref.getReferenceType());
			any = true;
		}
		if (!any) {
			out.println("  <no references>");
		}
	}

	// ------------------------------------------------------------------
	// Small utilities
	// ------------------------------------------------------------------

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwGameMainHop: section " + name);
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
