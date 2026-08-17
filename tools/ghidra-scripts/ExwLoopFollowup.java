/*-
 * ExwLoopFollowup.java — third-pass targeted dump for BEDLAM.EXW that ties the
 * startup chain to the real game loop. Runs against the already-imported
 * BedlamWatcom project (never re-import) and produces one output file with:
 *
 *   PROMOTE        — force disassembly + function creation at the two never-
 *                    promoted code addresses 0044d6e8 (exit/loop handler
 *                    installed via global 004ef8fc) and 0044dacc (WndProc).
 *   REFSTO         — every reference TO the eight hardcoded anchors (pump
 *                    0044d93c, WinMain-equiv 0044d320, 0044d6e8, 0044dacc,
 *                    globals 004ef8fc / 004ef690, CRT startup 004520ed, PE
 *                    entry 004502ee) with callsite, refType (READ/WRITE for
 *                    the data globals) and containing function.
 *   DECOMP         — ordered decompilation: function containing 0044d6e8,
 *                    function containing 0044dacc (WndProc), FUN_004520ed,
 *                    then every DISTINCT caller of the pump (0044d93c) and
 *                    WinMain-equiv (0044d320) found in REFSTO (cap 6).
 *                    Each entry carries a PSEUDOCALLS summary line.
 *   LISTING_WNDPROC — raw listing (addr TAB bytes TAB disassembly) of the
 *                    WndProc function after promotion.
 *
 * Usage:
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExwLoopFollowup.java <outputFile>
 *
 *   <outputFile> is optional and defaults to "exw-followup.txt" in the cwd.
 *
 * Every failure degrades gracefully: a SKIP line is logged instead of
 * aborting the run. Outputs are analysis artifacts in gitignored locations.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwLoopFollowup extends GhidraScript {

	private static final String DEFAULT_OUTPUT = "exw-followup.txt";

	/** Code addresses never promoted to functions by the importer. */
	private static final String[] PROMOTE_ADDRS = { "0044d6e8", "0044dacc" };

	/** REFSTO targets (hardcoded) paired with REF_LABELS by index. */
	private static final String[] REF_TARGETS = {
		"0044d93c", // message pump
		"0044d320", // WinMain-equivalent
		"0044d6e8", // loop/exit-handler candidate (& stored into global 004ef8fc)
		"0044dacc", // WndProc candidate
		"004ef8fc", // global holding pointer to 0044d6e8
		"004ef690", // quit-flag global
		"004520ed", // Watcom CRT startup
		"004502ee", // PE entry point
	};

	private static final String[] REF_LABELS = {
		"pump", "winmain_equiv", "loop_candidate", "wndproc", "global_loop_ptr",
		"global_quit_flag", "crt_startup", "pe_entry",
	};

	/** Containing-functions of refs to these targets are decompile candidates. */
	private static final Set<String> CALLER_DECOMP_TARGETS =
		Set.of("0044d93c", "0044d320");

	private static final String CRT_STARTUP_ADDR = "004520ed";

	private static final int MAX_CALLER_DECOMPS = 6;
	private static final int DECOMP_TIMEOUT_SECS = 90;

	private SymbolTable symTable;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outFile = Paths.get(args.length > 0 ? args[0] : DEFAULT_OUTPUT);
		if (outFile.getParent() != null) {
			Files.createDirectories(outFile.getParent());
		}
		println("ExwLoopFollowup: output file = " + outFile.toAbsolutePath());

		symTable = currentProgram.getSymbolTable();

		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(outFile, StandardCharsets.UTF_8))) {

			writeInfo(out);

			section(out, "PROMOTE");
			Function loopFn = promote(out, "0044d6e8");
			Function wndprocFn = promote(out, "0044dacc");

			section(out, "REFSTO");
			LinkedHashSet<Function> callers = writeRefsTo(out);

			section(out, "DECOMP");
			DecompInterface decomp = newDecompiler();
			try {
				writeDecomps(out, decomp, loopFn, wndprocFn, callers);
			}
			finally {
				decomp.dispose();
			}

			section(out, "LISTING_WNDPROC");
			writeListingWndproc(out, wndprocFn);
		}
		println("ExwLoopFollowup: done.");
	}

	// ------------------------------------------------------------------
	// INFO
	// ------------------------------------------------------------------

	private void writeInfo(PrintWriter out) {
		section(out, "INFO");
		out.println("program:      " + currentProgram.getName());
		out.println("imageBase:    " + currentProgram.getImageBase());
		out.println("language:     " + currentProgram.getLanguageID());
		out.println("compilerSpec: " + currentProgram.getCompilerSpec().getCompilerSpecID());
		out.println("promoteAddrs: " + String.join(",", PROMOTE_ADDRS));
	}

	// ------------------------------------------------------------------
	// PROMOTE
	// ------------------------------------------------------------------

	/**
	 * Ensures an instruction exists at addr (disassemble if needed) and that a
	 * function contains it (createFunction(addr, null) if needed). Never
	 * aborts: every failure is reported inline. Returns the function now
	 * containing addr, or null.
	 */
	private Function promote(PrintWriter out, String addrStr) {
		Address addr = parse(addrStr);
		if (addr == null) {
			out.println(addrStr + "\tSKIP bad address");
			println("ExwLoopFollowup: SKIP promote " + addrStr + " (bad address)");
			return null;
		}

		String disasmStatus;
		try {
			if (currentProgram.getListing().getInstructionAt(addr) == null) {
				disassemble(addr); // FlatProgramAPI; return value unneeded
				disasmStatus = currentProgram.getListing().getInstructionAt(addr) != null
					? "disassembled" : "disassemble produced no instruction";
			}
			else {
				disasmStatus = "instruction already present";
			}
		}
		catch (Exception e) {
			disasmStatus = "disassemble failed: " + e;
		}

		String creationStatus;
		Function fn = functionAtOrContaining(addr);
		if (fn != null) {
			creationStatus = "already in function " + fn.getEntryPoint() + " " + fn.getName();
		}
		else {
			try {
				fn = createFunction(addr, null); // null name -> reuse/generate symbol name
				creationStatus = fn != null
					? "created " + fn.getEntryPoint() + " " + fn.getName()
					: "createFunction returned null";
			}
			catch (Exception e) {
				creationStatus = "createFunction failed: " + e;
			}
		}

		Function now = functionAtOrContaining(addr);
		out.println(addr + "\t" + disasmStatus + "\t" + creationStatus + "\tnowIn=" +
			(now != null ? now.getEntryPoint() + " " + now.getName() : "<none>"));
		println("ExwLoopFollowup: promote " + addr + " -> " +
			(now != null ? now.getName() : "FAILED"));
		return now;
	}

	/** Function starting at addr, else any function containing it, else null. */
	private Function functionAtOrContaining(Address addr) {
		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		return fn != null ? fn : currentProgram.getFunctionManager().getFunctionContaining(addr);
	}

	// ------------------------------------------------------------------
	// REFSTO
	// ------------------------------------------------------------------

	/**
	 * Every reference to each hardcoded target with caller context. Collects
	 * the containing-functions of refs to the pump / WinMain-equiv targets as
	 * decompile candidates (encounter order, deduped).
	 */
	private LinkedHashSet<Function> writeRefsTo(PrintWriter out) {
		LinkedHashSet<Function> callers = new LinkedHashSet<>();
		for (int i = 0; i < REF_TARGETS.length; i++) {
			String label = REF_LABELS[i];
			Address target = parse(REF_TARGETS[i]);
			if (target == null) {
				out.println(label + "\t" + REF_TARGETS[i] + "\tSKIP bad address");
				continue;
			}
			int n = 0;
			try {
				ReferenceIterator it =
					currentProgram.getReferenceManager().getReferencesTo(target);
				while (it.hasNext()) {
					Reference ref = it.next();
					Function fn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					// refType alone identifies READ/WRITE for the data globals.
					out.println(label + "\t" + target + "\tfrom=" + ref.getFromAddress() +
						"\trefType=" + ref.getReferenceType() + "\tisCall=" +
						ref.getReferenceType().isCall() + "\tfn=" +
						(fn != null ? fn.getEntryPoint() + " " + fn.getName()
							: "<no function>"));
					if (fn != null && CALLER_DECOMP_TARGETS.contains(target.toString())) {
						callers.add(fn);
					}
					n++;
				}
			}
			catch (Exception e) {
				out.println(label + "\t" + target + "\tSKIP reference scan failed: " + e);
				println("ExwLoopFollowup: SKIP refs to " + target + ": " + e);
			}
			if (n == 0) {
				out.println(label + "\t" + target + "\tNO REFERENCES");
			}
			println("ExwLoopFollowup: refs to " + target + " (" + label + ") = " + n);
		}
		return callers;
	}

	// ------------------------------------------------------------------
	// DECOMP
	// ------------------------------------------------------------------

	/** Ordered decompile: loop-candidate, WndProc, CRT startup, then callers. */
	private void writeDecomps(PrintWriter out, DecompInterface decomp, Function loopFn,
			Function wndprocFn, LinkedHashSet<Function> callers) {
		List<Function> ordered = new ArrayList<>();
		Set<String> seenEntries = new LinkedHashSet<>();

		if (loopFn != null) {
			addIfNew(ordered, seenEntries, loopFn);
		}
		else {
			out.println("SKIP no function at/containing 0044d6e8 after promotion");
		}
		if (wndprocFn != null) {
			addIfNew(ordered, seenEntries, wndprocFn);
		}
		else {
			out.println("SKIP no function at/containing 0044dacc after promotion");
		}
		Function crtFn = functionAtOrContaining(parse(CRT_STARTUP_ADDR));
		if (crtFn != null) {
			addIfNew(ordered, seenEntries, crtFn);
		}
		else {
			out.println("SKIP no function at " + CRT_STARTUP_ADDR);
		}

		List<Function> capped = new ArrayList<>(callers);
		if (capped.size() > MAX_CALLER_DECOMPS) {
			out.println("NOTE truncating " + capped.size() + " caller decomps to " +
				MAX_CALLER_DECOMPS);
			capped = capped.subList(0, MAX_CALLER_DECOMPS);
		}
		for (Function fn : capped) {
			addIfNew(ordered, seenEntries, fn);
		}

		if (ordered.isEmpty()) {
			out.println("SKIP nothing to decompile");
			println("ExwLoopFollowup: SKIP decomp (no functions)");
			return;
		}
		for (Function fn : ordered) {
			decompileAndPrint(out, decomp, fn);
		}
		println("ExwLoopFollowup: decompiled " + ordered.size() + " functions");
	}

	/** Adds fn unless its entry point was already selected. */
	private boolean addIfNew(List<Function> ordered, Set<String> seenEntries, Function fn) {
		if (fn == null || !seenEntries.add(fn.getEntryPoint().toString())) {
			return false;
		}
		ordered.add(fn);
		return true;
	}

	// ------------------------------------------------------------------
	// LISTING_WNDPROC
	// ------------------------------------------------------------------

	private void writeListingWndproc(PrintWriter out, Function wndprocFn) {
		if (wndprocFn == null) {
			out.println("SKIP no WndProc function (promotion of 0044dacc failed)");
			println("ExwLoopFollowup: SKIP LISTING_WNDPROC (no function)");
			return;
		}
		out.println("function: " + wndprocFn.getEntryPoint() + " " + wndprocFn.getName());
		try {
			InstructionIterator it =
				currentProgram.getListing().getInstructions(wndprocFn.getBody(), true);
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
			println("ExwLoopFollowup: listed WndProc " + wndprocFn.getName());
		}
		catch (Exception e) {
			out.println("SKIP listing failed: " + e);
			println("ExwLoopFollowup: SKIP LISTING_WNDPROC: " + e);
		}
	}

	// ------------------------------------------------------------------
	// Decompilation helpers
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
				println("ExwLoopFollowup: decomp failed for " + fn.getName() + ": " +
					results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
			println("ExwLoopFollowup: decomp exception for " + fn.getName() + ": " + e);
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
		println("ExwLoopFollowup: section " + name);
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
