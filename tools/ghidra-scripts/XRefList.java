/*-
 * XRefList.java - print all references TO each given address in the
 * already-imported BEDLAM.EXW program. Usage: <outputFile> <addr>...
 * Ghidra discipline: run with -process BEDLAM.EXW -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class XRefList extends GhidraScript {

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 2) {
			throw new IllegalArgumentException("usage: XRefList <outFile> <addr>...");
		}
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			for (int i = 1; i < args.length; i++) {
				Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
					.getAddress(args[i]);
				out.println("----- XREF TO " + args[i] + " -----");
				ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(a);
				while (it.hasNext()) {
					Reference r = it.next();
					Function f = currentProgram.getFunctionManager()
						.getFunctionContaining(r.getFromAddress());
					out.println(r.getFromAddress() + " " + r.getReferenceType() + " in " +
						(f != null ? f.getName() : "<nofunc>"));
				}
			}
			out.println("===== DONE =====");
		}
		println("XRefList: done.");
	}
}
