/*-
 * ExwInputReaders.java - pass B of the input/control-map unit (P4 prep).
 * Pass A (ExwInputSinks) found the sinks write:
 *   - 256-byte scan-code state array @004edc44 (byte = key released; arrows
 *     remapped to 0xc8/0xcb/0xcd/0xd0)
 *   - edge-latch dwords: 004edb50 (ESC), 004edc08 (M|Space), 004edc0c/10/14
 *     (F1/F2/F3), 004edc18..30 step4 (digits 1..7), 004edc34 (P)
 *   - mouse-button flags bits0/1 of dword 004dc6e4
 * This pass does a listing-text census over every token (Ghidra refs miss
 * scaled-index operands) + getReferencesTo, dedups the READER functions
 * (excluding the sinks 0041be05/0041bf35 and WndProc 0044dacc) and
 * decompiles each (cap 14). BedlamWatcom project, -process BEDLAM.EXW
 * -noanalysis, NEVER re-import.
 * Output: <arg0> (default ghidra-project/exw-input-readers.txt)
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwInputReaders extends GhidraScript {

	private static final String[] SCAN_TOKENS = {
		// keystore base + special-key bytes (any spelling)
		"4edc44",
		"4edc45", "4edc46", "4edc47", "4edc48", "4edc49",
		"4edc4a", "4edc4b", "4edc4c", "4edc5d", "4edc76", "4edc7d",
		"4edc7f", "4edc80", "4edc81",
		// arrow-key bytes (base + 0xc8/0xcb/0xcd/0xd0)
		"4edd0c", "4edd0f", "4edd11", "4edd14",
		// edge-latch dwords
		"4edb50",
		"4edc08", "4edc0c", "4edc10", "4edc14",
		"4edc18", "4edc1c", "4edc20", "4edc24",
		"4edc28", "4edc2c", "4edc30", "4edc34",
		// mouse flags + scroll snapshot
		"4dc6e4", "4eddcc",
	};

	private static final String[] EXCLUDE_FNS = {
		"0041be05", "0041bf35", "0044dacc",
	};

	private static final int MAX_DECOMP = 14;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-input-readers.txt");
		if (outPath.getParent() != null) {
			Files.createDirectories(outPath.getParent());
		}
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		Set<String> readerFnSet = new LinkedHashSet<>();
		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outPath, StandardCharsets.UTF_8))) {

			section(out, "INFO");
			out.println("program: " + currentProgram.getName());

			section(out, "LISTING_CENSUS");
			int hits = 0;
			InstructionIterator it = currentProgram.getListing().getInstructions(true);
			while (it.hasNext()) {
				Instruction ins = it.next();
				String text = ins.toString();
				String found = null;
				for (String tok : SCAN_TOKENS) {
					if (text.contains(tok)) {
						found = (found == null) ? tok : found + "," + tok;
					}
				}
				if (found != null) {
					Function fn = currentProgram.getFunctionManager()
						.getFunctionContaining(ins.getAddress());
					String from = fn != null
						? fn.getName() + "@" + fn.getEntryPoint() : "<no function>";
					out.println("  " + found + "\t" + ins.getAddress() + "\t" + from
						+ "\t" + text);
					hits++;
					if (fn != null && !excluded(fn)) {
						readerFnSet.add(fn.getEntryPoint().toString());
					}
				}
			}
			out.println("// listing census: " + hits + " hits, "
				+ readerFnSet.size() + " distinct reader functions");

			section(out, "REFS_TO_DATA");
			for (String tok : SCAN_TOKENS) {
				Address addr = parse("00" + tok);
				if (addr == null) {
					continue;
				}
				boolean any = false;
				ReferenceIterator ri =
					currentProgram.getReferenceManager().getReferencesTo(addr);
				while (ri.hasNext()) {
					Reference ref = ri.next();
					Function fromFn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					String from = fromFn != null
						? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
					out.println("  " + tok + "\t" + ref.getFromAddress() + "\t" + from
						+ "\t" + ref.getReferenceType());
					any = true;
				}
				if (!any) {
					out.println("  " + tok + "\t<no references>");
				}
			}

			section(out, "READER_DECOMP");
			int n = 0;
			for (String ep : readerFnSet) {
				if (n >= MAX_DECOMP) {
					out.println("// CAP " + MAX_DECOMP + " reached; remaining: "
						+ (readerFnSet.size() - n));
					break;
				}
				Function fn = fnAt(ep);
				if (fn == null) {
					continue;
				}
				n++;
				out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName()
					+ " size=" + fn.getBody().getNumAddresses() + " -----");
				try {
					ghidra.app.decompiler.DecompileResults r =
						decomp.decompileFunction(fn, 120, monitor);
					out.println(r.decompileCompleted() && r.getDecompiledFunction() != null
						? r.getDecompiledFunction().getC()
						: "// DECOMP FAILED: " + r.getErrorMessage());
				}
				catch (Exception e) {
					out.println("// DECOMP FAILED: " + e);
				}
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwInputReaders: done -> " + outPath + " ("
			+ readerFnSet.size() + " readers)");
	}

	private boolean excluded(Function fn) {
		for (String ex : EXCLUDE_FNS) {
			if (fn.getEntryPoint().toString().equalsIgnoreCase(ex)) {
				return true;
			}
		}
		return false;
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null
			? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwInputReaders: section " + name);
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
