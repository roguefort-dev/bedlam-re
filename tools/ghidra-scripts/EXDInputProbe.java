/*-
 * EXDInputProbe.java - W5 injector support: pin the EXD keystore twin.
 * Disassembles the key-latch helper (FUN_0002ec12, the pause-spin key check
 * per exd-probe7) and every function referencing the latch cell it touches,
 * so the 256-byte keystore base DAT can be read off the indexed reads.
 * Usage: EXDInputProbe <outFile>
 * Ghidra discipline: -process BEDLAM.EXD -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.RefType;

public class EXDInputProbe extends GhidraScript {

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 1) {
			throw new IllegalArgumentException("usage: EXDInputProbe <outFile>");
		}
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			Address latch = currentProgram.getAddressFactory().getDefaultAddressSpace()
				.getAddress("001075b4");
			out.println("===== refs to latch 0x1075b4 =====");
			ReferenceIterator it = currentProgram.getReferenceManager()
				.getReferenceIterator(latch);
			while (it.hasNext()) {
				Reference r = it.next();
				if (!r.getToAddress().equals(latch)) {
					continue;
				}
				Function fn = currentProgram.getFunctionManager()
					.getFunctionContaining(r.getFromAddress());
				out.println(r.getFromAddress() + " " + r.getReferenceType() +
					" in " + (fn != null ? fn.getName() + "@" + fn.getEntryPoint() : "<none>"));
			}
			// disassemble FUN_0002ec12 raw
			Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
				.getAddress("0002ec12");
			Function fn = currentProgram.getFunctionManager().getFunctionAt(a);
			out.println("===== disasm FUN_0002ec12 =====");
			if (fn != null) {
				for (Address p = fn.getEntryPoint(); p.compareTo(fn.getBody().getMaxAddress()) <= 0;) {
					Instruction ins = currentProgram.getListing().getInstructionAt(p);
					if (ins == null) {
						p = p.add(1);
						continue;
					}
					out.println(p + " " + ins.toString());
					p = p.add(ins.getLength());
				}
			}
			else {
				out.println("// no function at 0002ec12");
			}
			out.println("===== DONE =====");
		}
		println("EXDInputProbe: done.");
	}
}
