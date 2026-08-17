/*-
 * ExwPacerFollowup.java - sim/render RATE follow-up for BEDLAM.EXW in the
 * already-imported Ghidra project "BedlamWatcom". NEVER re-import (AGENTS.md).
 *
 * Context (.state/NEXT.md task 1 / D15): FUN_0043d00b mission loop found:
 *   poll(FUN_00425ab9) -> sim/render -> FUN_00425a1e -> frame++(0046ae68)
 *   -> FUN_00425a03 -> _SmackWait(cinematics only). NO sleep in loop body.
 * FUN_00425a1e calls FUN_00402aaa 0x1e0(=480) times per frame - per-row
 * primitive, prime pacer suspect. This pass dumps the loop callees:
 *   00402aaa (14b)  per-row primitive (called 480x/frame + 1x w/ 0x4b000)
 *   00425a8b (21b)  begin-present
 *   00425aa0 (25b)  end-present
 *   0044acf4 (34b)  pre-flip
 *   0044ad18 (355b) surface flip (DD surface roles, tick2 open item)
 *   0043f5b1 (220b) sim step candidate 1
 *   0043f68d (144b) sim step candidate 2
 *   0043fb80 (778b) render candidate
 *   0044e1ca (11b)  the ONLY Sleep caller
 *   00451b62 (93b)  the ONLY WaitForSingleObject caller
 * Plus: xref census (0046ae68 frame ctr, 004edb50 attract flag, 004edb3c
 * pre-flip store, 004edb64 clock-run), import census (timeGetTime,
 * GetTickCount, QueryPerformanceCounter, Sleep, WaitForSingleObject), and
 * listings for named DDCreate/DDInitSurfaces (find SetDisplayMode refresh).
 *
 * Output: <outDir>/exw-pacer.txt (analysis artifact, gitignored).
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

public class ExwPacerFollowup extends GhidraScript {

	private static final String[] TARGETS = {
		"00402aaa", "00425a8b", "00425aa0", "0044acf4", "0044ad18",
		"0043f5b1", "0043f68d", "0043fb80", "0044e1ca", "00451b62",
	};

	private static final String[] XREF_GLOBALS = {
		"0046ae68", "004edb50", "004edb3c", "004edb64", "004edb88",
	};

	private static final String[] IMPORT_NAMES = {
		"Sleep", "timeGetTime", "GetTickCount", "QueryPerformanceCounter",
		"WaitForSingleObject",
	};

	private static final String[] LISTING_BY_NAME = { "DDCreate", "DDInitSurfaces" };

	private static final int DECOMP_TIMEOUT_SECS = 120;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwPacerFollowup: output dir = " + outDir.toAbsolutePath());

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-pacer.txt"), StandardCharsets.UTF_8))) {
			section(out, "INFO");
			out.println("program:       " + currentProgram.getName());
			out.println("imageBase:     " + currentProgram.getImageBase());
			out.println("compilerSpec:  " + currentProgram.getCompilerSpec().getCompilerSpecID());
			section(out, "DECOMP");
			DecompInterface decomp = newDecompiler();
			try {
				for (String addrStr : TARGETS) {
					Function fn = fnAt(addrStr);
					if (fn == null) {
						out.println("SKIP " + addrStr + " no function");
						continue;
					}
					decompileAndPrint(out, decomp, fn);
				}
			}
			finally {
				decomp.dispose();
			}
			section(out, "LISTING");
			for (String addrStr : TARGETS) {
				writeListing(out, addrStr);
			}
			for (String name : LISTING_BY_NAME) {
				writeListingByName(out, name);
			}
			section(out, "XREFS");
			for (String addrStr : XREF_GLOBALS) {
				writeXrefs(out, addrStr);
			}
			section(out, "IMPORT_CALLERS");
			writeImportCallers(out);
		}
		println("ExwPacerFollowup: done.");
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
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
		out.println("PSEUDOCALLS: " + pseudoCalls(fn));
		try {
			DecompileResults results = decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
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
				if (to == null) {
					targets.add("<indirect>");
					continue;
				}
				Function f = currentProgram.getFunctionManager().getFunctionAt(to);
				targets.add(f != null ? f.getName() : to.toString());
			}
		}
		return String.join(";", targets);
	}

	private void writeListing(PrintWriter out, String addrStr) {
		Address addr = parse(addrStr);
		Function fn = addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
		if (fn == null) {
			out.println("SKIP " + addrStr + " no function at address");
			return;
		}
		writeListingFn(out, fn);
	}

	private void writeListingByName(PrintWriter out, String name) {
		FunctionIterator fit = currentProgram.getFunctionManager().getFunctions(true);
		while (fit.hasNext()) {
			Function fn = fit.next();
			if (name.equals(fn.getName())) {
				writeListingFn(out, fn);
				return;
			}
		}
		out.println("SKIP by-name " + name + " not found");
	}

	private void writeListingFn(PrintWriter out, Function fn) {
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
			Function fromFn =
				currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
			String from = fromFn != null
					? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
			out.println("  " + ref.getFromAddress() + "\t" + from + "\t" + ref.getReferenceType());
			any = true;
		}
		if (!any) {
			out.println("  <no references>");
		}
	}

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

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwPacerFollowup: section " + name);
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
