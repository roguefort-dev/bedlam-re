/*-
 * ExwMusicTails.java - close the small open tails of the .MRS chain
 * (docs/RE-EXW-MUSIC.md sec 6): (a) FUN_0044c4a8 sub-voice start applier
 * (presumed SetFrequency 16.16 ratio + volume, consuming 0045b03e/0045b042),
 * its caller FUN_0044c3a4 and sibling DSPrimeSubVoice 0044c828 for context;
 * (b) xref census of header table C pointer array 0045cda8 (written by
 * load_midi, reader unknown); (c) xref census of loop-flag array 0045cdc0
 * (writer of =1 unknown) and pending-restart word base 004543d4; plus
 * auto-decompile of every function referencing those addresses (capped).
 * Also applies plate comments to the already-verified music chain fns.
 * Runs against the already-imported BedlamWatcom project, NEVER re-import:
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExwMusicTails.java <outputFile>
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwMusicTails extends GhidraScript {

	/* decompile + listing */
	private static final String[] DUMP_FNS = {
		"0044c4a8", // sub-voice start applier (MAIN TARGET)
		"0044c3a4", // caller: find free sub-voice
		"0044c828", // DSPrimeSubVoice (context)
	};

	/* xref census targets */
	private static final String[] XREF_TARGETS = {
		"0045cda8", // header table C pointer array (reader unknown)
		"0045cdc0", // loop-flag array u16[song] (writer of 1 unknown)
		"004543d4", // pending-restart word base (writer unknown)
	};

	/* auto-decompile cap for census-referencing functions */
	private static final int MAX_AUTO = 16;

	/* plate comments for already-verified functions (docs/RE-EXW-MUSIC.md) */
	private static final String[][] PLATE_COMMENTS = {
		{ "00402bac",
			"MusicPump: 100Hz tick sequencer, song slot 3 ONLY. Per tick per chunk (0x26-stride state @0045b020): while delta 0045b038==0 dispatch pending event 0045b03a, then MrsNextEvent reads the next event. See docs/RE-EXW-MUSIC.md sec 1/2b." },
		{ "004032a5",
			"MrsChunkStart(song, chan): PATTERN RESTART - re-init ALL chunks of channel chan from the header tables (variant word, start-offset, initial tick delay). Triggered by 0xFF events, 0xFE in loop mode, and the pending-restart word 004543d4." },
		{ "00402e74",
			"MrsNextEvent: read u16 delta then opcode byte. 0x00..0x7E note-on (variant 0: inst=b; variant 1: inst=variant+7, ratio=table@00454174[b] 16.16, tag=b-0x54; +1 volume byte, 0xFF=note-off), 0x7F song-end shadow copy, 0x80..0xFD rest, 0xFE/0xFF restart on chan byte. Signed delta <0 = freeze (natural stop); 30001..32767 = backward reposition (unused by data)." },
		{ "004033d4",
			"MusicStart(song): zero per-chunk position counters 0045ca60, reset per-chunk state (MrsChunkStart path), set play flag 0045b010[song]=1, zero loop flag 0045cdc0[song]." },
		{ "00403642",
			"load_midi(base, song): MusicStop + VoicesFree + VoiceTableWipe, then load_mrs (<base>.MRS, arena) and build per-chunk runtime pointers (0045c7e0/0045ca60/0045cce8) + header table pointers: start-offset 0045cd88, tick-delay 0045cd98, table C 0045cda8." },
		{ "0044c2cc",
			"mrw_load(base, song): load_mrw (<base>.MRW) then per instrument create a DS voice (DSCreateVoice 11025Hz 8-bit mono) and assign sub-voice slot table 004ef4e0 + song*0x40." },
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArguments();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-music-tails.txt");
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
			Set<String> dumped = new LinkedHashSet<>();
			for (String addrStr : DUMP_FNS) {
				Function fn = fnAt(addrStr);
				dumped.add(addrStr);
				if (fn == null) {
					out.println("SKIP " + addrStr + " no function");
					continue;
				}
				dumpFn(out, decomp, fn);
			}

			section(out, "XREFS");
			Set<String> censusFns = new LinkedHashSet<>();
			for (String addrStr : XREF_TARGETS) {
				Address addr = parse(addrStr);
				if (addr == null) {
					out.println("SKIP " + addrStr + " bad address");
					continue;
				}
				out.println("refs-to: " + addrStr);
				boolean any = false;
				ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(addr);
				while (it.hasNext()) {
					Reference ref = it.next();
					Function fromFn = currentProgram.getFunctionManager()
						.getFunctionContaining(ref.getFromAddress());
					String from = fromFn != null
						? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
					out.println("  " + ref.getFromAddress() + "\t" + from + "\t"
						+ ref.getReferenceType());
					if (fromFn != null) {
						censusFns.add(fromFn.getEntryPoint().toString());
					}
					any = true;
				}
				if (!any) {
					out.println("  <no references>");
				}
			}

			section(out, "AUTO_DECOMP");
			int n = 0;
			for (String ep : censusFns) {
				if (dumped.contains(ep) || n >= MAX_AUTO) {
					continue;
				}
				Function fn = fnAt(ep);
				if (fn == null) {
					continue;
				}
				dumpFn(out, decomp, fn);
				n++;
			}
			out.println("// auto-decompiled " + n + " of " + censusFns.size()
				+ " census functions (cap " + MAX_AUTO + ")");

			section(out, "PLATE_COMMENTS");
			for (String[] e : PLATE_COMMENTS) {
				Function fn = fnAt(e[0]);
				if (fn == null) {
					out.println(e[0] + "\tSKIP no function");
					continue;
				}
				try {
					fn.setComment(e[1]);
					out.println(e[0] + "\t" + fn.getName() + "\tCOMMENT SET");
				}
				catch (Exception ex) {
					out.println(e[0] + "\tCOMMENT FAILED: " + ex);
				}
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwMusicTails: done -> " + outPath);
	}

	private void dumpFn(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		try {
			DecompileResults r = decomp.decompileFunction(fn, 120, monitor);
			out.println(r.decompileCompleted() && r.getDecompiledFunction() != null
				? r.getDecompiledFunction().getC()
				: "// DECOMP FAILED: " + r.getErrorMessage());
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
		}
		out.println("listing:");
		InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			out.println(it.next().toString());
		}
		out.flush();
		monitor.checkCanceled();
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwMusicTails: section " + name);
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
