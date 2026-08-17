/*-
 * ExwTickNames.java - name+dump pass after the tick2 followup. NEVER re-import.
 *   1. Decompile + list FUN_0045204b (the .data slot 00457874 thread-spawn target).
 *   2. Persist semantic names discovered by exw-tick2.txt analysis.
 * Output: <outDir>/exw-tick2-names.txt
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwTickNames extends GhidraScript {

	private static final String[][] FN_NAMES = {
		{ "00425901", "FadeStep", "50Hz palette fade stepper; decrements g_fade_ticks_left" },
		{ "0044b428", "CursorToGame", "GetCursorPos -> window-scaled 640x480 game coords (out EAX/EDX ptrs)" },
		{ "0044aed4", "SetPaletteRGB", "upload 6-bit RGB bytes<<2 to ddraw palette + SetEntries + SetPalette" },
		{ "0041cbf0", "FadeSetup", "compute 16.16 fade from current palette to target over N 50Hz steps" },
		{ "0044a5f0", "DDCreate", "DirectDrawCreate wrapper (+SetCooperativeLevel check)" },
		{ "0044a660", "DDInitSurfaces", "SetCooperativeLevel/SetDisplayMode/CreateSurface/CreatePalette (+clipper)" },
		{ "0044ab54", "DDShutdown", "RestoreDisplayMode, FlipToGDISurface, release all dd objects" },
		{ "0045204b", "ThreadSpawnImpl", ".data slot 00457874 target: CreateThread-like impl" },
	};

	/** { address, label } data labels. */
	private static final String[][] DATA_NAMES = {
		{ "004ede10", "g_fade_ticks_left" },
		{ "004edc38", "g_fade_state_16_16" },
		{ "004edc3c", "g_fade_palette_6bit" },
		{ "004ee9b8", "g_dd_obj" },
		{ "004ee9d0", "g_dd_palette" },
		{ "004ee9d4", "g_dd_clipper" },
		{ "00457874", "g_thread_spawn_slot" },
	};

	private static final int DECOMP_TIMEOUT_SECS = 90;

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwTickNames: output dir = " + outDir.toAbsolutePath());

		SymbolTable symTable = currentProgram.getSymbolTable();
		List<String> log = new ArrayList<>();

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-tick2-names.txt"), StandardCharsets.UTF_8))) {
			out.println("===== SECTION: NAMES =====");
			for (String[] entry : FN_NAMES) {
				Address addr = parse(entry[0]);
				if (addr == null) {
					log.add(entry[0] + "\tSKIP bad address");
					continue;
				}
				Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
				if (fn == null) {
					try {
						if (currentProgram.getListing().getInstructionAt(addr) == null) {
							disassemble(addr);
						}
						fn = createFunction(addr, entry[1]);
					}
					catch (Exception e) {
						log.add(addr + "\tSKIP create failed: " + e);
						continue;
					}
				}
				if (fn == null) {
					log.add(addr + "\tSKIP createFunction null");
					continue;
				}
				String old = fn.getName();
				if (!old.equals(entry[1])) {
					try {
						fn.setName(entry[1], SourceType.USER_DEFINED);
						log.add(addr + "\t" + old + " -> " + entry[1] + "\t" + entry[2]);
					}
					catch (Exception e) {
						log.add(addr + "\tSKIP rename: " + e);
					}
				}
				else {
					log.add(addr + "\tALREADY " + entry[1]);
				}
			}
			for (String[] entry : DATA_NAMES) {
				Address addr = parse(entry[0]);
				if (addr == null) {
					continue;
				}
				try {
					Symbol s = symTable.getPrimarySymbol(addr);
					if (s != null && s.getName().equals(entry[1])) {
						log.add(addr + "\tALREADY " + entry[1]);
					}
					else {
						createLabel(addr, entry[1], true, SourceType.USER_DEFINED);
						log.add(addr + "\tLABEL -> " + entry[1]);
					}
				}
				catch (Exception e) {
					log.add(addr + "\tSKIP label: " + e);
				}
			}
			for (String line : log) {
				out.println(line);
				println(line);
			}

			out.println();
			out.println("===== SECTION: DECOMP 0045204b =====");
			Function fn = currentProgram.getFunctionManager().getFunctionAt(parse("0045204b"));
			if (fn != null) {
				DecompInterface decomp = new DecompInterface();
				decomp.setOptions(new DecompileOptions());
				decomp.toggleCCode(true);
				decomp.openProgram(currentProgram);
				try {
					out.println("PSEUDOCALLS: " + pseudoCalls(fn));
					DecompileResults results = decomp.decompileFunction(fn, DECOMP_TIMEOUT_SECS, monitor);
					if (results.decompileCompleted() && results.getDecompiledFunction() != null) {
						out.println(results.getDecompiledFunction().getC());
					}
					else {
						out.println("// DECOMP FAILED: " + results.getErrorMessage());
					}
				}
				finally {
					decomp.dispose();
				}
				out.println();
				out.println("===== SECTION: LISTING 0045204b =====");
				out.println("function: " + fn.getEntryPoint() + " " + fn.getName() +
					" (" + fn.getBody().getNumAddresses() + " bytes)");
				InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
				while (it.hasNext()) {
					Instruction inst = it.next();
					out.println(inst.toString());
					for (Reference ref : inst.getReferencesFrom()) {
						if (ref.getReferenceType().isCall()) {
							out.println("    ; calls -> " + ref.getToAddress());
						}
					}
				}
			}
			else {
				out.println("SKIP no function at 0045204b");
			}
		}
		println("ExwTickNames: done.");
	}

	private String pseudoCalls(Function fn) {
		List<String> targets = new ArrayList<>();
		InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
		while (it.hasNext()) {
			Instruction inst = it.next();
			for (Reference ref : inst.getReferencesFrom()) {
				if (!ref.getReferenceType().isCall()) {
					continue;
				}
				Address to = ref.getToAddress();
				Function f = to != null ? currentProgram.getFunctionManager().getFunctionAt(to) : null;
				String desc = f != null ? f.getName() : String.valueOf(to);
				if (!targets.contains(desc)) {
					targets.add(desc);
				}
			}
		}
		return String.join(";", targets);
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
