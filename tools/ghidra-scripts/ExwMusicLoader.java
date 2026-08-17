/*-
 * ExwMusicLoader.java - .MRS/.MRW loader + CONFIG.BDL census for BEDLAM.EXW.
 *
 * Runs against the already-imported BedlamWatcom project (-process BEDLAM.EXW
 * -noanalysis). Produces exw-music.txt with sections:
 *   STRINGS   - memory scan (loaded+initialized blocks) for SOUND[MIDI, .MRS,
 *               .MRW, .MRK, CONFIG, .BDL, SAVED.BDL, OPTIONS.BDL, HISCORE
 *               byte patterns + every reference to each hit.
 *   FILEIO    - callers of CreateFileA/ReadFile/SetFilePointer/CloseHandle/
 *               GetFileSize and DirectSoundCreate/waveOutOpen/midiOutOpen/
 *               timeSetEvent imports.
 *   DECOMP    - decompile every function found in STRINGS/FILEIO plus their
 *               callees to depth 2, capped at MAX_DECOMP functions.
 *
 * Usage:
 *   analyzeHeadless <projDir> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath <thisDir> -postScript ExwMusicLoader.java <outputDir>
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayDeque;
import java.util.Deque;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.ExternalLocation;
import ghidra.program.model.symbol.ExternalLocationIterator;
import ghidra.program.model.symbol.ExternalManager;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwMusicLoader extends GhidraScript {

	private static final String BS = "\134";
	private static final String[] PATTERNS = {
		"SOUND" + BS + "MIDI" + BS, ".MRS", ".MRW", ".MRK", "CONFIG", ".BDL",
		"SAVED.BDL", "OPTIONS.BDL", "HISCORE",
	};

	private static final String[] IMPORT_LABELS = {
		"CreateFileA", "ReadFile", "SetFilePointer", "CloseHandle", "GetFileSize",
		"DirectSoundCreate", "waveOutOpen", "midiOutOpen", "timeSetEvent",
	};

	private static final int MAX_DECOMP = 45;
	private static final int DECOMP_TIMEOUT_SECS = 90;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		Path outDir = Paths.get(args.length > 0 ? args[0] : ".");
		Files.createDirectories(outDir);
		println("ExwMusicLoader: output dir = " + outDir.toAbsolutePath());

		Set<Function> musicFns = new LinkedHashSet<>();

		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(outDir.resolve("exw-music.txt"),
				StandardCharsets.UTF_8))) {

			section(out, "STRINGS");
			Memory mem = currentProgram.getMemory();
			for (String pat : PATTERNS) {
				byte[] bytes = pat.getBytes(StandardCharsets.US_ASCII);
				for (MemoryBlock block : mem.getBlocks()) {
					if (!block.isInitialized() || !block.isLoaded()) {
						continue;
					}
					Address hit = block.getStart();
					while (hit != null && hit.compareTo(block.getEnd()) <= 0) {
						hit = mem.findBytes(hit, block.getEnd(), bytes, null, true,
							monitor);
						if (hit == null) {
							break;
						}
						String ctx = readCString(mem, hit, 48);
						StringBuilder refs = new StringBuilder();
						ReferenceIterator ri =
							currentProgram.getReferenceManager().getReferencesTo(hit);
						int n = 0;
						while (ri.hasNext()) {
							Reference ref = ri.next();
							Function fn = currentProgram.getFunctionManager()
								.getFunctionContaining(ref.getFromAddress());
							refs.append("\n    use@").append(ref.getFromAddress())
								.append(" fn=")
								.append(fn != null ? fn.getEntryPoint() + " " + fn.getName()
									: "<no function>");
							if (fn != null) {
								musicFns.add(fn);
							}
							n++;
						}
						out.println("HIT " + hit + " \"" + ctx + "\"" +
							(n == 0 ? "  (no direct refs)" : refs));
						hit = hit.next();
					}
					monitor.checkCanceled();
				}
			}

			section(out, "FILEIO");
			ExternalManager extMgr = currentProgram.getExternalManager();
			for (String lib : extMgr.getExternalLibraryNames()) {
				ExternalLocationIterator it = extMgr.getExternalLocations(lib);
				while (it.hasNext()) {
					ExternalLocation loc = it.next();
					if (!isWanted(loc.getLabel())) {
						continue;
					}
					Address a = loc.getAddress();
					if (a == null) {
						continue;
					}
					ReferenceIterator ri =
						currentProgram.getReferenceManager().getReferencesTo(a);
					while (ri.hasNext()) {
						Reference ref = ri.next();
						Function fn = currentProgram.getFunctionManager()
							.getFunctionContaining(ref.getFromAddress());
						out.println(loc.getLabel() + "\tcallsite=" + ref.getFromAddress() +
							"\tfn=" + (fn != null ? fn.getEntryPoint() + " " + fn.getName()
								: "<no function>"));
						if (fn != null && ("CreateFileA".equals(loc.getLabel())
							|| "DirectSoundCreate".equals(loc.getLabel()))) {
							musicFns.add(fn);
						}
					}
				}
			}

			section(out, "DECOMP-SET");
			for (Function fn : musicFns) {
				out.println("queued " + fn.getEntryPoint() + " " + fn.getName());
			}

			Set<Function> all = new LinkedHashSet<>(musicFns);
			Deque<Function> frontier = new ArrayDeque<>(musicFns);
			int depthBudget = 2;
			for (int d = 0; d < depthBudget && !frontier.isEmpty(); d++) {
				Deque<Function> next = new ArrayDeque<>();
				while (!frontier.isEmpty()) {
					Function fn = frontier.poll();
					for (Function callee : fn.getCalledFunctions(monitor)) {
						if (all.add(callee)) {
							next.add(callee);
						}
					}
					if (all.size() > 4 * MAX_DECOMP) {
						break;
					}
				}
				frontier = next;
			}

			section(out, "DECOMP");
			DecompInterface decomp = newDecompiler();
			int count = 0;
			try {
				for (Function fn : all) {
					if (count >= MAX_DECOMP) {
						out.println("SKIP (cap) " + fn.getEntryPoint() + " " + fn.getName());
						continue;
					}
					decompileAndPrint(out, decomp, fn);
					count++;
				}
			}
			finally {
				decomp.dispose();
			}
			out.println();
			out.println("===== TOTAL DECOMPILED: " + count + " =====");
		}
		println("ExwMusicLoader: done.");
	}

	private String readCString(Memory mem, Address start, int max) {
		StringBuilder sb = new StringBuilder();
		try {
			for (int i = 0; i < max; i++) {
				byte b = mem.getByte(start.add(i));
				if (b == 0) {
					break;
				}
				sb.append((char) (b & 0xff));
			}
		}
		catch (Exception e) {
			/* truncate */
		}
		return sb.toString();
	}

	private DecompInterface newDecompiler() {
		DecompInterface decomp = new DecompInterface();
		decomp.setOptions(new DecompileOptions());
		decomp.toggleCCode(true);
		decomp.openProgram(currentProgram);
		return decomp;
	}

	private void decompileAndPrint(PrintWriter out, DecompInterface decomp, Function fn) {
		out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
		try {
			DecompileResults results =
				decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
			if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
				out.println(results.getDecompiledFunction().getC());
			}
			else {
				out.println("// DECOMP FAILED: " + results.getErrorMessage());
			}
		}
		catch (Exception e) {
			out.println("// DECOMP FAILED: " + e);
		}
	}

	private boolean isWanted(String label) {
		for (String wanted : IMPORT_LABELS) {
			if (wanted.equals(label)) {
				return true;
			}
		}
		return false;
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwMusicLoader: section " + name);
	}
}
