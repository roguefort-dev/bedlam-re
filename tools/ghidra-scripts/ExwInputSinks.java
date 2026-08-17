/*-
 * ExwInputSinks.java - pass A of the input/control-map unit (P4 prep).
 * Decompile + full listing of the two game-side input sinks:
 *   FUN_0041be05 (vkey, down)  keyboard sink called from WndProc
 *   FUN_0041bf35 (button, state) mouse-button sink called from WndProc
 * plus caller census (refs-to AND listing token scan, since scaled-index
 * operands create no reference). The globals these sinks write feed pass B
 * (listing census over those addresses to find the game-side readers).
 * BedlamWatcom project, -process BEDLAM.EXW -noanalysis, NEVER re-import.
 * Output: <arg0> (default ghidra-project/exw-input-sinks.txt)
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwInputSinks extends GhidraScript {

	private static final String[] DUMP_FNS = {
		"0041be05", // keyboard sink (vkey, down)
		"0041bf35", // mouse-button sink (button, state)
	};

	private static final String[] SCAN_TOKENS = {
		"41be05", "41bf35",
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-input-sinks.txt");
		if (outPath.getParent() != null) {
			Files.createDirectories(outPath.getParent());
		}
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outPath, StandardCharsets.UTF_8))) {

			section(out, "INFO");
			out.println("program: " + currentProgram.getName());

			section(out, "DUMP");
			for (String addrStr : DUMP_FNS) {
				Function fn = fnAt(addrStr);
				if (fn == null) {
					out.println("SKIP " + addrStr + " no function");
					continue;
				}
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
				out.println("listing:");
				InstructionIterator li =
					currentProgram.getListing().getInstructions(fn.getBody(), true);
				while (li.hasNext()) {
					out.println(li.next().toString());
				}
			}

			section(out, "CALLERS_REFS");
			for (String addrStr : DUMP_FNS) {
				Address addr = parse(addrStr);
				if (addr == null) {
					out.println("SKIP " + addrStr + " bad address");
					continue;
				}
				out.println("refs-to: " + addrStr);
				boolean any = false;
				ReferenceIterator ri =
					currentProgram.getReferenceManager().getReferencesTo(addr);
				while (ri.hasNext()) {
					Reference ref = ri.next();
					Function fromFn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					String from = fromFn != null
						? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
					out.println("  " + ref.getFromAddress() + "\t" + from + "\t"
						+ ref.getReferenceType());
					any = true;
				}
				if (!any) {
					out.println("  <no references>");
				}
			}

			section(out, "CALLERS_LISTING");
			int hits = 0;
			InstructionIterator it = currentProgram.getListing().getInstructions(true);
			while (it.hasNext()) {
				Instruction ins = it.next();
				String text = ins.toString();
				for (String tok : SCAN_TOKENS) {
					if (text.contains(tok)) {
						Function fn = currentProgram.getFunctionManager()
							.getFunctionContaining(ins.getAddress());
						String from = fn != null
							? fn.getName() + "@" + fn.getEntryPoint() : "<no function>";
						out.println("  " + tok + "\t" + ins.getAddress() + "\t" + from
							+ "\t" + text);
						hits++;
						break;
					}
				}
			}
			out.println("// listing census: " + hits + " hits");
		}
		finally {
			decomp.dispose();
		}
		println("ExwInputSinks: done -> " + outPath);
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null
			? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwInputSinks: section " + name);
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
