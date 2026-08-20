/*-
 * ExwBootAttract.java - one-off RE for the boot attract arm (queue item 1):
 * 1) locate the GAMEGFX\*.SMK strings, resolve code xrefs, decompile each
 *    referencing function;
 * 2) decompile the FUN_0044567c satellites (open/pacing helpers);
 * 3) list all code refs to the skip gate 004edbc4 and the movies-enabled
 *    flag 0046cca4 (instruction-level, store/load visible in listing text).
 * Ghidra discipline: -process BEDLAM.EXW -noanalysis only, never re-import.
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
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.data.StringDataInstance;

public class ExwBootAttract extends GhidraScript {

	private PrintWriter out;
	private DecompInterface decomp;

	private void dumpFunc(Address a) throws Exception {
		Function fn = currentProgram.getFunctionManager().getFunctionContaining(a);
		if (fn == null) {
			out.println("----- NO FUNCTION CONTAINING " + a + " -----");
			return;
		}
		if (fn.getEntryPoint().equals(a)) {
			out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		} else {
			out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName()
				+ " (via ref from " + a + ") -----");
		}
		DecompileResults r = decomp.decompileFunction(fn, 90, monitor);
		if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
			out.println(r.getDecompiledFunction().getC());
		} else {
			out.println("// DECOMP FAILED: " + r.getErrorMessage());
		}
	}

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		try (PrintWriter w = new PrintWriter(Files.newBufferedWriter(
				Paths.get(args[0]), StandardCharsets.UTF_8))) {
			out = w;
			decomp = new DecompInterface();
			decomp.setOptions(new DecompileOptions());
			decomp.toggleCCode(true);
			decomp.openProgram(currentProgram);
			Listing listing = currentProgram.getListing();
			Memory mem = currentProgram.getMemory();

			// 1) SMK string xrefs
			for (String want : new String[] {
					"GAMEGFX\\TITLE.SMK", "GAMEGFX\\GTLOG_US.SMK", "GAMEGFX\\LOGO_US.SMK",
					"GAMEGFX\\GAMEOVER.SMK" }) {
				out.println("===== STRING " + want + " =====");
				boolean found = false;
				var dataIt = listing.getDefinedData(true);
				while (dataIt.hasNext()) {
					var d = dataIt.next();
					StringDataInstance s = StringDataInstance.getStringDataInstance(d);
					if (s != null) {
						String v = s.getStringValue();
						if (v != null && v.contains(want)) {
							found = true;
							out.println("  data " + d.getAddress() + " = \"" + v + "\"");
							for (Reference ref : currentProgram.getReferenceManager().getReferencesTo(d.getAddress())) {
								out.println("    ref-from " + ref.getFromAddress()
									+ " type=" + ref.getReferenceType());
								dumpFunc(ref.getFromAddress());
							}
						}
					}
				}
				if (!found) out.println("  (no defined data matched)");
			}

			// 2) satellites of FUN_0044567c
			for (String a : new String[] { "0041ce69", "0042597c", "0044bc08", "0044bc6c", "0044b340" }) {
				dumpFunc(currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(a));
			}

			// 3) refs to the skip gate + movies-enabled flag
			for (String sym : new String[] { "004edbc4", "0046cca4" }) {
				out.println("===== REFS TO " + sym + " =====");
				Address sa = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(sym);
				ReferenceIterator ri = currentProgram.getReferenceManager().getReferencesTo(sa);
				while (ri.hasNext()) {
					Reference ref = ri.next();
					Address from = ref.getFromAddress();
					var cu = listing.getCodeUnitContaining(from);
					String txt = cu == null ? "?" : cu.toString();
					out.println("  " + from + " " + ref.getReferenceType() + " | " + txt);
				}
			}
			out.println("===== DONE =====");
		} finally {
			if (decomp != null) decomp.dispose();
		}
		println("ExwBootAttract: done.");
	}
}
