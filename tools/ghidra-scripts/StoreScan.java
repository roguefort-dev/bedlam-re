/*-
 * StoreScan.java - scan ALL instructions in the already-imported BEDLAM.EXW
 * program and print those whose formatted operands contain any of the given
 * substrings (e.g. "0x4c7226", "0x4e66b8"). Finds computed/EBP-based refs the
 * reference manager misses.
 * Usage: <outputFile> <substring> [<substring>...]
 * Ghidra discipline: run with -process BEDLAM.EXW -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class StoreScan extends GhidraScript {

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 2) {
			throw new IllegalArgumentException("usage: StoreScan <outFile> <substr>...");
		}
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			InstructionIterator it =
				currentProgram.getListing().getInstructions(true);
			while (it.hasNext()) {
				if (monitor.isCancelled()) {
					break;
				}
				Instruction ins = it.next();
				String text = ins.toString();
				boolean hit = false;
				for (int i = 1; i < args.length && !hit; i++) {
					if (text.indexOf(args[i]) >= 0) {
						hit = true;
					}
				}
				if (hit) {
					Address a = ins.getAddress();
					Function f = currentProgram.getFunctionManager()
						.getFunctionContaining(a);
					String fname = (f == null) ? "?" : f.getName();
					out.println(a + "  " + text + "   <" + fname + ">");
				}
			}
			out.println("===== DONE =====");
		}
	}
}
