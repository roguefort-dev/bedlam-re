/*-
 * DecompList.java - decompile an explicit address list from the already-imported
 * BEDLAM.EXW program. Usage (postScript args): <outputFile> <addr> [<addr>...]
 * Ghidra discipline: run with -process BEDLAM.EXW -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

public class DecompList extends GhidraScript {

	private static final int TIMEOUT_SECS = 90;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 2) {
			throw new IllegalArgumentException("usage: DecompList <outFile> <addr>...");
		}
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			DecompInterface decomp = new DecompInterface();
			decomp.setOptions(new DecompileOptions());
			decomp.toggleCCode(true);
			decomp.openProgram(currentProgram);
			try {
				for (int i = 1; i < args.length; i++) {
					Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
						.getAddress(args[i]);
					Function fn = currentProgram.getFunctionManager().getFunctionAt(a);
					out.println("----- DECOMP " + args[i] + " " +
						(fn != null ? fn.getName() : "<nofunc>") + " -----");
					if (fn == null) {
						out.println("// NO FUNCTION AT ADDRESS");
						continue;
					}
					DecompileResults r = decomp.decompileFunction(fn, TIMEOUT_SECS, monitor);
					if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
						out.println(r.getDecompiledFunction().getC());
					}
					else {
						out.println("// DECOMP FAILED: " + r.getErrorMessage());
					}
				}
			}
			finally {
				decomp.dispose();
			}
			out.println("===== DONE =====");
		}
		println("DecompList: done.");
	}
}
