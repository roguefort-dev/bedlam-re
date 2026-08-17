/*-
 * ExwTickFollowup2.java - second tick followup for BEDLAM.EXW in the already
 * imported Ghidra project "BedlamWatcom". NEVER re-import (AGENTS.md rule).
 *
 * Targets (docs/RE-EXW-TICK.md open list):
 *   1. FUN_00425901 - the 50Hz-gated update called by FUN_00402b0c.
 *   2. FUN_0044b428 - scroll delta source consumed by FUN_00425ab9.
 *      (both + one level of direct callees; listing dump for both since
 *       Watcom register args hide in decompiler output)
 *   3. .data slot 00457874 - thread spawn function slot: dump initial value,
 *      all xrefs, and decompile the pointed-to function if it is code.
 *   4. DDRAW object array 004ee9b0..004ee9d0: dump initial dwords, xrefs
 *      (esp. WRITES = creation sites), decompile writer functions, and
 *      callers of the DirectDrawCreate import. Plus full listing of
 *      AppActivate so the +0x18 call on 004ee9d0 shows its register/stack
 *      argument setup.
 *   5. Gate globals 004ede10 (50Hz) / 004edbe0 (MusicPump): xref writers.
 *
 * Output: <outDir>/exw-tick2.txt (analysis artifact, gitignored location).
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
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwTickFollowup2 extends GhidraScript {

	/** { address, name, note } - named then decompiled, in this order. */
	private static final String[][] STATIC_TARGETS = {
		{ "00425901", "FUN_00425901", "50Hz-gated update called from FUN_00402b0c" },
		{ "0044b428", "FUN_0044b428", "scroll delta source consumed by FUN_00425ab9" },
	};

	/** Direct callees of these roots are auto-decompiled (one level). */
	private static final String[] AUTO_CALLEE_ROOTS = { "00425901", "0044b428" };

	/** Listing dumps (Watcom register args + AppActivate vtable call sites). */
	private static final String[] LISTING_TARGETS = { "00425901", "0044b428", "0044b1c0" };

	/** Globals whose writers/readers matter for the open questions. */
	private static final String[] XREF_GLOBALS = {
		"00457874",                                        // thread spawn slot
		"004ee9bc", "004ee9c0", "004ee9c8", "004ee9cc", "004ee9d0", // ddraw object slots
		"004ede10",                                        // 50Hz gate
		"004edbe0",                                        // MusicPump gate
	};

	/** Import whose callers reveal the DDRAW init site. */
	private static final String[] CALLER_HINTS = { "00453020" }; // DirectDrawCreate

	private static final int DECOMP_TIMEOUT_SECS = 90;
	private static final int MAX_AUTO_CALLEES = 20;
	private static final int MAX_DERIVED = 8;

	private SymbolTable symTable;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwTickFollowup2: output dir = " + outDir.toAbsolutePath());

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

		// Derived targets: writers of tracked globals + callers of imports.
		Set<String> derivedAddrs = new LinkedHashSet<>();
		for (String addrStr : CALLER_HINTS) {
			collectCallers(addrStr, derivedAddrs);
		}
		List<String> writerLogs = new ArrayList<>();
		for (String addrStr : XREF_GLOBALS) {
			collectWriters(addrStr, derivedAddrs, writerLogs);
		}
		// Thread-spawn slot target: initial value -> decompile if it is code.
		String slotReport = resolveSlotTarget("00457874", derivedAddrs);

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-tick2.txt"), StandardCharsets.UTF_8))) {
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
				List<Function> derived = new ArrayList<>();
				int n = 0;
				for (String a : derivedAddrs) {
					if (n >= MAX_DERIVED) {
						break;
					}
					Function fn = getOrCreateFunction(a);
					if (fn != null && !containsFn(targets, fn) && !containsFn(autoTargets, fn)) {
						derived.add(fn);
						n++;
					}
				}
				if (!derived.isEmpty()) {
					out.println("// ---- derived: ddraw slot writers + DirectDrawCreate callers ----");
					for (Function fn : derived) {
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
			for (String line : writerLogs) {
				out.println(line);
			}
			for (String addrStr : XREF_GLOBALS) {
				writeXrefs(out, addrStr);
			}
			section(out, "SLOT_00457874");
			out.println(slotReport);
			section(out, "DATA_004EE9B0");
			writeDwords(out, "004ee9b0", 12);
		}
		println("ExwTickFollowup2: done.");
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
				println("ExwTickFollowup2: SKIP " + addr + ": " + e);
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
				println("ExwTickFollowup2: " + addr + " " + oldName + " -> " + newName);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP rename failed: " + e + "\t" + note);
				println("ExwTickFollowup2: SKIP rename " + addr + ": " + e);
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
		println("ExwTickFollowup2: auto callees = " + ordered.size());
		return ordered;
	}

	// ------------------------------------------------------------------
	// Derived target collection (callers of import, writers of globals)
	// ------------------------------------------------------------------

	private void collectCallers(String addrStr, Set<String> out) {
		Address addr = parse(addrStr);
		if (addr == null) {
			return;
		}
		ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(addr);
		while (it.hasNext()) {
			Reference ref = it.next();
			if (ref.getReferenceType().isCall()) {
				Function fromFn =
					currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
				if (fromFn != null) {
					out.add(fromFn.getEntryPoint().toString());
				}
			}
		}
	}

	private void collectWriters(String addrStr, Set<String> out, List<String> log) {
		Address addr = parse(addrStr);
		if (addr == null) {
			return;
		}
		ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(addr);
		while (it.hasNext()) {
			Reference ref = it.next();
			if (ref.getReferenceType().isWrite()) {
				Function fromFn =
					currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
				String from = fromFn != null ? fromFn.getName() + "@" + fromFn.getEntryPoint()
						: "<no function>@" + ref.getFromAddress();
				log.add("WRITE " + addrStr + " from " + from);
				if (fromFn != null) {
					out.add(fromFn.getEntryPoint().toString());
				}
			}
		}
	}

	private String resolveSlotTarget(String addrStr, Set<String> out) {
		StringBuilder sb = new StringBuilder();
		Address addr = parse(addrStr);
		if (addr == null) {
			return "SKIP bad address " + addrStr;
		}
		try {
			int val = currentProgram.getMemory().getInt(addr);
			sb.append("initial dword @").append(addrStr).append(" = 0x")
				.append(String.format("%08x", val)).append("\n");
			Address tgt = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(
				val & 0xffffffffL);
			sb.append("interpreted as address: ").append(tgt).append("\n");
			if (val != 0 && currentProgram.getMemory().getLoadedAndInitializedAddressSet()
					.contains(tgt)) {
				Function fn = getOrCreateFunction(tgt.toString());
				if (fn != null) {
					sb.append("target is code: function ").append(fn.getName()).append(" @")
						.append(fn.getEntryPoint()).append("\n");
					out.add(tgt.toString());
				}
				else {
					sb.append("target in image but not resolvable as function\n");
				}
			}
		}
		catch (Exception e) {
			sb.append("read failed: ").append(e).append("\n");
		}
		return sb.toString();
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
				println("ExwTickFollowup2: decomp failed for " + fn.getName() + ": " +
					results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExwTickFollowup2: decomp exception for " + fn.getName() + ": " + e);
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
	// Listing / Xrefs / Data
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

	private void writeDwords(PrintWriter out, String addrStr, int count) {
		Address addr = parse(addrStr);
		if (addr == null) {
			out.println("SKIP " + addrStr + " bad address");
			return;
		}
		for (int i = 0; i < count; i++) {
			Address a = addr.add(i * 4L);
			try {
				int val = currentProgram.getMemory().getInt(a);
				out.println(a + "\t0x" + String.format("%08x", val));
			}
			catch (Exception e) {
				out.println(a + "\t<unreadable>");
			}
		}
	}

	// ------------------------------------------------------------------
	// Small utilities
	// ------------------------------------------------------------------

	private Function getOrCreateFunction(String addrStr) {
		Address addr = parse(addrStr);
		if (addr == null) {
			return null;
		}
		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		if (fn != null) {
			return fn;
		}
		try {
			if (currentProgram.getListing().getInstructionAt(addr) == null) {
				disassemble(addr);
			}
			return createFunction(addr, "FUN_" + addrStr);
		}
		catch (Exception e) {
			return null;
		}
	}

	private boolean containsFn(List<Function> list, Function fn) {
		for (Function f : list) {
			if (f.getEntryPoint().equals(fn.getEntryPoint())) {
				return true;
			}
		}
		return false;
	}

	private boolean containsFn(Map<String, Function> map, Function fn) {
		return map.containsValue(fn);
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwTickFollowup2: section " + name);
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
