/*-
 * ExwMusicTails3.java - pass 3 (final) of the music-tails unit. Pass 2
 * proved Ghidra reference census MISSES scaled-index operands
 * ([EAX*2+0x45cdbe] creates no reference), so this pass does a full
 * LISTING-TEXT census over every instruction for the music globals
 * (any base spelling: 45cdc0/45cdbe loop flag, 4543d2/4543d4 pending
 * restart, 45cda8/45cda6 table C ptrs, 4ef5e0 inst count, 45b010/45b00e
 * play flag, 4ee9b4/4ee9b6 master-vol / palette-reflag words), plus a
 * normal caller census for SubVoiceFind 0044c3a4 / SubVoiceStart 0044c4a8 /
 * master-vol setter FUN_0044c630, plus MrsTriggerNote listing (resolve the
 * shim register shuffle: ECX=ratio?, stack=vol<<8?).
 * BedlamWatcom project, -process BEDLAM.EXW -noanalysis, NEVER re-import.
 * Output: <arg0> (default ghidra-project/exw-music-tails3.txt)
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

public class ExwMusicTails3 extends GhidraScript {

	private static final String[] SCAN_TOKENS = {
		"45cdc0", "45cdbe", "4543d2", "4543d4", "45cda8", "45cda6",
		"4ef5e0", "45b010", "45b00e", "4ee9b4", "4ee9b6", "4ef4de", "4ef4e0",
	};

	private static final String[] DUMP_FNS = {
		"00402e46", // MrsTriggerNote shim listing
	};

	private static final String[] CALLER_TARGETS = {
		"0044c3a4", "0044c4a8", "0044c630",
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-music-tails3.txt");
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

			section(out, "LISTING_CENSUS");
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

			section(out, "DUMP");
			for (String addrStr : DUMP_FNS) {
				Function fn = fnAt(addrStr);
				if (fn == null) {
					out.println("SKIP " + addrStr + " no function");
					continue;
				}
				out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
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

			section(out, "CALLERS");
			for (String addrStr : CALLER_TARGETS) {
				Address addr = parse(addrStr);
				if (addr == null) {
					out.println("SKIP " + addrStr + " bad address");
					continue;
				}
				out.println("refs-to: " + addrStr);
				boolean any = false;
				ReferenceIterator ri = currentProgram.getReferenceManager().getReferencesTo(addr);
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
		}
		finally {
			decomp.dispose();
		}
		println("ExwMusicTails3: done -> " + outPath);
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwMusicTails3: section " + name);
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
