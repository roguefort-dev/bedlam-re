/*-
 * DumpRange.java - print the instruction listing for [start,end) address
 * ranges in the already-imported BEDLAM.EXW program.
 * Usage: DumpRange <outFile> <startAddr> <endAddr> [<startAddr> <endAddr> ...]
 * Ghidra discipline: run with -process BEDLAM.EXW -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class DumpRange extends GhidraScript {

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 3 || (args.length - 1) % 2 != 0) {
			throw new IllegalArgumentException("usage: DumpRange <outFile> <start> <end> ...");
		}
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			for (int i = 1; i < args.length; i += 2) {
				Address s = currentProgram.getAddressFactory().getDefaultAddressSpace()
					.getAddress(args[i]);
				Address e = currentProgram.getAddressFactory().getDefaultAddressSpace()
					.getAddress(args[i + 1]);
				out.println("----- RANGE " + args[i] + ".." + args[i + 1] + " -----");
				InstructionIterator it =
					currentProgram.getListing().getInstructions(s, true);
				while (it.hasNext()) {
					Instruction ins = it.next();
					if (ins.getAddress().compareTo(e) >= 0) {
						break;
					}
					out.println(ins.getAddress() + " " + ins);
				}
			}
			out.println("===== DONE =====");
		}
		println("DumpRange: done.");
	}
}
