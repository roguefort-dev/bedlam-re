/*-
 * ExwSurfConfirm.java - dump pass for the DD surface creation-order
 * confirmation (queue item: name the surface slots via FUN_0044a9ac /
 * FUN_0044ad18). Project BedlamWatcom, program BEDLAM.EXW, NEVER re-import.
 * READ-ONLY: decompiles + listings + current symbol names; no renames.
 * Also verifies the persisted state of CrtThreadTrampoline@00451fbc.
 * Output: <outDir>/exw-surf-confirm.txt
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
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwSurfConfirm extends GhidraScript {

	private static final String[] DUMP_FNS = {
		"0044a9ac", "0044ad18", "0044a660",
	};

	private static final String[] CHECK_FNS = { "00451fbc" };

	private static final String[] LABEL_SLOTS = {
		"004ee9b8", "004ee9bc", "004ee9c0", "004ee9c8",
		"004ee9cc", "004ee9d0", "004ee9d4",
	};

	private static final int DECOMP_TIMEOUT_SECS = 120;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwSurfConfirm: output dir = " + outDir.toAbsolutePath());

		SymbolTable symTable = currentProgram.getSymbolTable();

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-surf-confirm.txt"), StandardCharsets.UTF_8))) {
			out.println("program: " + currentProgram.getName());

			out.println();
			out.println("===== persisted-state checks =====");
			for (String addrStr : CHECK_FNS) {
				Function fn = currentProgram.getFunctionManager().getFunctionAt(parse(addrStr));
				out.println(addrStr + " function: " + (fn == null ? "MISSING" : fn.getName()));
			}
			for (String addrStr : LABEL_SLOTS) {
				Symbol s = symTable.getPrimarySymbol(parse(addrStr));
				out.println(addrStr + " label: " + (s == null ? "(none)" : s.getName()));
			}

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
		println("ExwSurfConfirm: done.");
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
