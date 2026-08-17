/*-
 * ExwMusicFollowup.java - focused decompile of the .MRS/.MRW music chain in
 * BEDLAM.EXW + name application. Runs against the already-imported
 * BedlamWatcom project (-process BEDLAM.EXW -noanalysis).
 *
 * Usage: analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *   -scriptPath <thisDir> -postScript ExwMusicFollowup.java <outputFile>
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.LinkedHashMap;
import java.util.Map;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;

public class ExwMusicFollowup extends GhidraScript {

	/* entry -> new name (empty = decompile only, do not rename) */
	private static final Map<String, String> FNS = new LinkedHashMap<>();
	static {
		FNS.put("0044c2cc", "mrw_load");
		FNS.put("0044c64c", "DSCreateVoice");
		FNS.put("0044c828", "DSPrimeSubVoice");
		FNS.put("004033d4", "MusicStart");
		FNS.put("00402bac", "MusicPump");
		FNS.put("004032a5", "MrsChunkStart");
		FNS.put("00402e74", "MrsNextEvent");
		FNS.put("00402e46", "MrsTriggerNote");
		FNS.put("00402db9", "VoiceAlloc");
		FNS.put("004034ef", "MusicStop");
		FNS.put("004035f5", "VoiceTableWipe");
		FNS.put("0043a48d", "VoicesFree");
		FNS.put("00402965", "Rand16");
		FNS.put("0043a39c", "SfxLoad");
		FNS.put("00403642", "load_midi");
		FNS.put("00403827", "load_mrs");
		FNS.put("004038c6", "load_mrw");
		FNS.put("0041cc7f", "LoadFile");
		FNS.put("0041db89", "ArenaAlloc");
		FNS.put("0044c480", "DSReleaseVoice");
	}

	private static final int TIMEOUT_SECS = 120;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outPath = args.length > 0 ? Paths.get(args[0])
			: Paths.get("ghidra-project", "exw-music-followup.txt");
		if (outPath.getParent() != null) {
			Files.createDirectories(outPath.getParent());
		}
		FunctionManager fm = currentProgram.getFunctionManager();
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outPath, StandardCharsets.UTF_8))) {
			for (Map.Entry<String, String> e : FNS.entrySet()) {
				Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
					.getAddress(e.getKey());
				Function fn = fm.getFunctionAt(a);
				out.println("----- DECOMP " + e.getKey() + " "
					+ (fn != null ? fn.getName() : "<none>") + " -----");
				if (fn == null) {
					out.println("// NO FUNCTION AT " + e.getKey());
					continue;
				}
				DecompileResults r = decomp.decompileFunction(fn, TIMEOUT_SECS, monitor);
				if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
					out.println(r.getDecompiledFunction().getC());
				}
				else {
					out.println("// DECOMP FAILED: " + r.getErrorMessage());
				}
				if (!e.getValue().isEmpty() && fn != null) {
					try {
						fn.setName(e.getValue(), ghidra.program.model.symbol.SourceType.USER_DEFINED);
						fn.setComment("named by ExwMusicFollowup (see docs/RE-EXW-MUSIC.md)");
						out.println("// RENAMED -> " + e.getValue());
					}
					catch (Exception ex) {
						out.println("// RENAME FAILED: " + ex);
					}
				}
				out.flush();
				monitor.checkCanceled();
			}
		}
		finally {
			decomp.dispose();
		}
		println("ExwMusicFollowup: done -> " + outPath);
	}
}
