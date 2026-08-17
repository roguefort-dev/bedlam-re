/*-
 * ExwGameThread.java - game worker-thread body dump for BEDLAM.EXW in the
 * already imported Ghidra project "BedlamWatcom". NEVER re-import (AGENTS.md).
 *
 * Usage (postScript, -process mode only):
 *   analyzeHeadless ghidra-project BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath tools/ghidra-scripts -postScript ExwGameThread.java ghidra-project
 *
 * What it does:
 *   1. Creates (disassemble-if-needed) + names the game worker-thread body at
 *      0044dea0 (start address passed to CreateThread by GameThreadStart) and
 *      the tiny go-flag setter FUN_0044d9b4.
 *   2. Decompiles both static targets.
 *   3. Auto-decompiles direct callees of 0044dea0 (one level deep, capped at
 *      24) - the sim/render loop's first hop. PSEUDOCALLS lines give the
 *      second-hop map; nothing deeper is decompiled.
 *   4. Dumps instruction listings for the targets (Watcom register args that
 *      the decompiler hides).
 *   5. Dumps references-to for the loop-relevant globals.
 *
 * Output: <outDir>/exw-gamethread.txt (analysis artifact in gitignored dir).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwGameThread extends GhidraScript {

	/** { address, name, note } - named then decompiled, in this order. */
	private static final String[][] STATIC_TARGETS = {
		{ "0044dea0", "GameThread", "worker thread start address from GameThreadStart CreateThread args (00457874 slot)" },
		{ "0044d9b4", "GoFlagSet", "10-byte writer of go flag 004ef674" },
	};

	/** Direct callees of these roots are auto-decompiled (one level). */
	private static final String[] AUTO_CALLEE_ROOTS = { "0044dea0" };

	/** Tiny functions: listing shows Watcom register args the decompiler hides. */
	private static final String[] LISTING_TARGETS = { "0044dea0", "0044d9b4" };

	/** Globals whose writers/readers matter for the game-thread doc. */
	private static final String[] XREF_GLOBALS = {
		"004ef674", "00457874", "004edbc8", "004ede10", "004ee9bc",
	};

	private static final int DECOMP_TIMEOUT_SECS = 90;
	private static final int MAX_AUTO_CALLEES = 24;

	private SymbolTable symTable;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwGameThread: output dir = " + outDir.toAbsolutePath());

		symTable = currentProgram.getSymbolTable();

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
			Files.newBufferedWriter(outDir.resolve("exw-gamethread.txt"), StandardCharsets.UTF_8))) {
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
			section(out, "XREFS");
			for (String addrStr : XREF_GLOBALS) {
				writeXrefs(out, addrStr);
			}
		}
		println("ExwGameThread: done.");
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
				println("ExwGameThread: SKIP " + addr + ": " + e);
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
				println("ExwGameThread: " + addr + " " + oldName + " -> " + newName);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP rename failed: " + e + "\t" + note);
				println("ExwGameThread: SKIP rename " + addr + ": " + e);
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
		println("ExwGameThread: auto callees = " + ordered.size());
		return ordered;
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
				println("ExwGameThread: decomp failed for " + fn.getName() + ": " + results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExwGameThread: decomp exception for " + fn.getName() + ": " + e);
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
				String desc = describeTarget(ref.getToAddress());
				if (!targets.contains(desc)) {
					targets.add(desc);
				}
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
		Symbol s = symTable.getPrimarySymbol(to);
		if (s != null) {
			return s.getName();
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
		println("ExwGameThread: section " + name);
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
