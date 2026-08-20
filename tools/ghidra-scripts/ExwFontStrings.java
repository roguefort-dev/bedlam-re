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

public class ExwFontStrings extends GhidraScript {

	private static final String LANG = "004eba1c";
	private static final int MAX_AUTO = 10;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-font-strings.txt");
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

			Set<String> hitFns = new LinkedHashSet<>();
			Set<String> langFns = new LinkedHashSet<>();

			section(out, "LISTING_HITS_46af5c");
			int n = 0;
			InstructionIterator ii = currentProgram.getListing().getInstructions(true);
			while (ii.hasNext()) {
				Instruction ins = ii.next();
				String txt = ins.toString();
				if (txt.toLowerCase().contains("46af5c")) {
					Address t = ins.getAddress();
					Function f = currentProgram.getFunctionManager().getFunctionContaining(t);
					if (f != null) {
						hitFns.add(f.getEntryPoint().toString());
						out.println(t + "\t" + f.getName() + "@" + f.getEntryPoint() + "\t" + txt);
					}
					else {
						out.println(t + "\t<no fn>\t" + txt);
					}
					n++;
				}
			}
			out.println("// " + n + " listing hits for 46af5c");

			section(out, "XREFS_LANG_004eba1c");
			Address la = parse(LANG);
			ReferenceIterator rit = currentProgram.getReferenceManager().getReferencesTo(la);
			while (rit.hasNext()) {
				Reference ref = rit.next();
				Function f = currentProgram.getFunctionManager()
					.getFunctionContaining(ref.getFromAddress());
				out.println(ref.getFromAddress() + "\t"
					+ (f != null ? f.getName() + "@" + f.getEntryPoint() : "<no fn>")
					+ "\t" + ref.getReferenceType());
				if (f != null) {
					langFns.add(f.getEntryPoint().toString());
				}
			}

			section(out, "DECOMP_TABLE_REFS");
			int k = 0;
			for (String ep : hitFns) {
				if (k >= MAX_AUTO) {
					break;
				}
				Function fn = fnAt(ep);
				if (fn == null) {
					continue;
				}
				dumpFn(out, decomp, fn);
				k++;
			}
			out.println("// decompiled " + k + " of " + hitFns.size());

			section(out, "DECOMP_LANG_REFS");
			for (String ep : langFns) {
				if (hitFns.contains(ep)) {
					continue;
				}
				Function fn = fnAt(ep);
				if (fn == null) {
					continue;
				}
				dumpFn(out, decomp, fn);
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwFontStrings: done -> " + outPath);
	}

	private void dumpFn(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		try {
			var r = decomp.decompileFunction(fn, 120, monitor);
			out.println(r.decompileCompleted() && r.getDecompiledFunction() != null
				? r.getDecompiledFunction().getC()
				: "// DECOMP FAILED: " + r.getErrorMessage());
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
		}
	}

	private void section(PrintWriter out, String s) {
		out.println();
		out.println("===== " + s + " =====");
	}

	private Function fnAt(String addrStr) {
		Address a = parse(addrStr);
		return a == null ? null : currentProgram.getFunctionManager().getFunctionContaining(a);
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
