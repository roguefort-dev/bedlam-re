/*-
 * ExwMusicTails2.java - pass 2 of the music-tails unit (BedlamWatcom,
 * -process BEDLAM.EXW -noanalysis, NEVER re-import). Pass 1 found the
 * applier 0044c4a8 (SubVoiceStart: stock IDirectSoundBuffer vtable
 * SetCurrentPosition/SetFrequency/SetVolume/SetPan/Play) and that MusicPump
 * reads some music globals through ALIASED dword bases (hiword extraction),
 * so refs-to the exact base can miss writers. This pass censuses the alias
 * bases, dumps the probe FUN_0044c5ac, auto-decompiles referencing
 * functions, and applies verified names.
 *   alias bases: 0045cdbe (dword base whose hiword = loop flag 0045cdc0+2s),
 *     004543d2 (hiword = pending restart 004543d4+2s), 0045b00e (hiword =
 *     play flag 0045b010+2s), 0045cda6 (hiword = table C ptr 0045cda8+4s).
 *   extra census: 004ef5e0 (per-song instrument count table), 004ee9b4
 *     (master word consumed by the SubVoiceStart volume formula).
 * Names applied (call-site-verified): 0044c4a8 SubVoiceStart,
 *   0044c3a4 SubVoiceFind, 0044c5ac SubVoiceProbe; globals 0045cdc0
 *   g_music_loopflag, 004543d4 g_music_pending_restart, 0045cda8
 *   g_tableC_ptrs, 004ef5e0 g_song_inst_count, 004ee9b4 g_music_master_vol.
 * Output: <arg0> (default ghidra-project/exw-music-tails2.txt)
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
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwMusicTails2 extends GhidraScript {

	private static final String[] DUMP_FNS = {
		"0044c5ac", // sub-voice availability probe
	};

	private static final String[] XREF_TARGETS = {
		"0045cdbe", "004543d2", "0045b00e", "0045cda6",
		"004ef5e0", "004ee9b4",
	};

	private static final int MAX_AUTO = 16;

	private static final String[][] FN_NAMES = {
		{ "0044c4a8", "SubVoiceStart" },
		{ "0044c3a4", "SubVoiceFind" },
		{ "0044c5ac", "SubVoiceProbe" },
	};

	private static final String[][] GLOBAL_NAMES = {
		{ "0045cdc0", "g_music_loopflag" },
		{ "004543d4", "g_music_pending_restart" },
		{ "0045cda8", "g_tableC_ptrs" },
		{ "004ef5e0", "g_song_inst_count" },
		{ "004ee9b4", "g_music_master_vol" },
	};

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-music-tails2.txt");
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

			section(out, "NAMES_APPLIED");
			for (String[] e : FN_NAMES) {
				Function fn = fnAt(e[0]);
				if (fn == null) {
					out.println(e[0] + "\tSKIP no function\t" + e[1]);
					continue;
				}
				String oldName = fn.getName();
				if (!oldName.equals(e[1])) {
					try {
						fn.setName(e[1], SourceType.USER_DEFINED);
						fn.setComment("named by ExwMusicTails2 (see docs/RE-EXW-MUSIC.md sec 6)");
						out.println(e[0] + "\t" + oldName + " -> " + e[1]);
					}
					catch (Exception ex) {
						out.println(e[0] + "\tSKIP rename failed " + ex + "\t" + e[1]);
					}
				}
				else {
					out.println(e[0] + "\tALREADY " + e[1]);
				}
			}
			SymbolTable st = currentProgram.getSymbolTable();
			for (String[] e : GLOBAL_NAMES) {
				Address addr = parse(e[0]);
				if (addr == null) {
					out.println(e[0] + "\tSKIP bad address\t" + e[1]);
					continue;
				}
				Symbol sym = st.getPrimarySymbol(addr);
				String oldName = sym != null ? sym.getName() : "<none>";
				if (!e[1].equals(oldName)) {
					try {
						st.createLabel(addr, e[1], SourceType.USER_DEFINED);
						out.println(e[0] + "\t" + oldName + " -> " + e[1]);
					}
					catch (Exception ex) {
						try {
							sym.setName(e[1], SourceType.USER_DEFINED);
							out.println(e[0] + "\t" + oldName + " => " + e[1] + " (rename)");
						}
						catch (Exception e2) {
							out.println(e[0] + "\tSKIP " + e2 + "\t" + e[1]);
						}
					}
				}
				else {
					out.println(e[0] + "\tALREADY " + e[1]);
				}
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwMusicTails2: done -> " + outPath);
	}

	private void dumpFn(PrintWriter out, DecompInterface decomp, Function fn) throws Exception {
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
		println("ExwMusicTails2: section " + name);
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
