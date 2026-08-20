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

public class ExwMenuParse extends GhidraScript {
	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-menu-parse.txt");
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outPath, StandardCharsets.UTF_8))) {
			String[] targets = { "00424679", "0042463d", "004245e6" };
			for (String t : targets) {
				Address a = currentProgram.getAddressFactory().getAddress(t);
				Function fn = currentProgram.getFunctionManager().getFunctionContaining(a);
				if (fn == null) {
					out.println("SKIP " + t);
					continue;
				}
				out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
				var r = decomp.decompileFunction(fn, 120, monitor);
				out.println(r.decompileCompleted() && r.getDecompiledFunction() != null
					? r.getDecompiledFunction().getC()
					: "// DECOMP FAILED: " + r.getErrorMessage());
				var it = currentProgram.getListing().getInstructions(fn.getBody(), true);
				while (it.hasNext()) {
					out.println(it.next().toString());
				}
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwMenuParse: done");
	}
}
