/*-
 * ExwPacerNames.java - name the present/pacer chain for BEDLAM.EXW
 * (project BedlamWatcom, NEVER re-import). Evidence: ghidra-project/exw-pacer.txt
 * + exw-gamemainhop.txt (see run 2026-08-18). Dumps-then-names:
 *   00402aaa MemCopy       rep movsb memcpy (Watcom inline)
 *   00425a8b SurfaceLock   Lock-retry until success, sets g_surface_locked
 *   00425aa0 SurfaceUnlock Unlock if locked, clears g_surface_locked
 *   00425a1e PresentCopy   Lock + 480 row-copies (640x480) + Unlock
 *   00425a03 PresentEnd    Unlock + DDFlipOrBlt
 *   0044ad18 DDFlipOrBlt   fullscreen Flip(DDFLIP_WAIT) / windowed Blt(DDBLT_WAIT)
 *                          + hw-cursor handshake + SetPalette
 *   0044acf4 DDSurfaceUnlock vtable+0x80 Unlock on staging surface
 *   0043f5b1 AnimSprites   24-sprite anim step (frame-rate bound)
 *   0043f68d AnimEntities  300-entity anim step (frame-rate bound)
 *   0043fb80 DrawOverlays  15+15 text overlays, 7-frame lifetime
 *   00402b48 PlayClockTick hh:mm:ss divider from 100Hz ticks
 *   0041e19d GameGoRelease reset counters + GoFlagSet
 * Globals: 0046ae68 g_frame_count, 004edb3c g_surface_locked,
 *   004edb50 g_input_seen, 004eee5c g_presenting.
 * Verify-then-name extras: 0044ac5c (hyp DDSurfaceLock - DUMPED, named only
 * if listing shows vtable call), plus xrefs 004eedf8/004eedfa/004ee9b6 and
 * callers of 0044e1ca (Sleep wrapper) and 0043e7d4 (frame-counter consumer).
 * Output: <outDir>/exw-pacer-names.txt
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
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.RefType;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

public class ExwPacerNames extends GhidraScript {

	private static final String[][] FN_NAMES = {
		{ "00402aaa", "MemCopy" },
		{ "00425a8b", "SurfaceLock" },
		{ "00425aa0", "SurfaceUnlock" },
		{ "00425a1e", "PresentCopy" },
		{ "00425a03", "PresentEnd" },
		{ "0044ad18", "DDFlipOrBlt" },
		{ "0044acf4", "DDSurfaceUnlock" },
		{ "0043f5b1", "AnimSprites" },
		{ "0043f68d", "AnimEntities" },
		{ "0043fb80", "DrawOverlays" },
		{ "00402b48", "PlayClockTick" },
		{ "0041e19d", "GameGoRelease" },
	};

	private static final String[][] GLOBAL_NAMES = {
		{ "0046ae68", "g_frame_count" },
		{ "004edb3c", "g_surface_locked" },
		{ "004edb50", "g_input_seen" },
		{ "004eee5c", "g_presenting" },
	};

	private static final String[] DUMP_FNS = { "0044ac5c", "0043e7d4" };

	private static final String[] XREF_GLOBALS = {
		"004eedf8", "004eedfa", "004ee9b6", "004ef670",
	};

	private static final String[] CALLER_FNS = { "0044e1ca", "0043e7d4" };

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		String outDirName = args.length > 0 ? args[0] : ".";
		Path outDir = Paths.get(outDirName);
		Files.createDirectories(outDir);
		println("ExwPacerNames: output dir = " + outDir.toAbsolutePath());

		try (PrintWriter out = new PrintWriter(
			Files.newBufferedWriter(outDir.resolve("exw-pacer-names.txt"), StandardCharsets.UTF_8))) {
			section(out, "INFO");
			out.println("program:       " + currentProgram.getName());
			section(out, "DUMP");
			DecompInterface decomp = new DecompInterface();
			decomp.setOptions(new DecompileOptions());
			decomp.toggleCCode(true);
			decomp.openProgram(currentProgram);
			for (String addrStr : DUMP_FNS) {
				Function fn = fnAt(addrStr);
				if (fn == null) {
					out.println("SKIP " + addrStr + " no function");
					continue;
				}
				out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
				try {
					DecompileResults results = decomp.decompileFunction(fn, 120, monitor);
					out.println(results.decompileCompleted() && results.getDecompiledFunction() != null
						? results.getDecompiledFunction().getC()
						: "// DECOMP FAILED: " + results.getErrorMessage());
				}
				catch (Exception e) {
					out.println("// DECOMP FAILED: " + e);
				}
				out.println("listing:");
				InstructionIterator it = currentProgram.getListing().getInstructions(fn.getBody(), true);
				while (it.hasNext()) {
					out.println(it.next().toString());
				}
			}
			decomp.dispose();
			section(out, "XREFS");
			for (String addrStr : XREF_GLOBALS) {
				writeXrefs(out, addrStr);
			}
			for (String addrStr : CALLER_FNS) {
				writeXrefs(out, addrStr);
			}
			section(out, "NAMES_APPLIED");
			for (String[] entry : FN_NAMES) {
				Function fn = fnAt(entry[0]);
				if (fn == null) {
					out.println(entry[0] + "\tSKIP no function\t" + entry[1]);
					continue;
				}
				String oldName = fn.getName();
				if (!oldName.equals(entry[1])) {
					try {
						fn.setName(entry[1], SourceType.USER_DEFINED);
						out.println(entry[0] + "\t" + oldName + " -> " + entry[1]);
					}
					catch (Exception e) {
						out.println(entry[0] + "\tSKIP rename failed " + e + "\t" + entry[1]);
					}
				}
				else {
					out.println(entry[0] + "\tALREADY " + entry[1]);
				}
			}
			SymbolTable st = currentProgram.getSymbolTable();
			for (String[] entry : GLOBAL_NAMES) {
				Address addr = parse(entry[0]);
				if (addr == null) {
					out.println(entry[0] + "\tSKIP bad address\t" + entry[1]);
					continue;
				}
				Symbol sym = st.getPrimarySymbol(addr);
				String oldName = sym != null ? sym.getName() : "<none>";
				if (!entry[1].equals(oldName)) {
					try {
						st.createLabel(addr, entry[1], SourceType.USER_DEFINED);
						out.println(entry[0] + "\t" + oldName + " -> " + entry[1]);
					}
					catch (Exception e) {
						try {
							sym.setName(entry[1], SourceType.USER_DEFINED);
							out.println(entry[0] + "\t" + oldName + " => " + entry[1] + " (rename)");
						}
						catch (Exception e2) {
							out.println(entry[0] + "\tSKIP " + e2 + "\t" + entry[1]);
						}
					}
				}
				else {
					out.println(entry[0] + "\tALREADY " + entry[1]);
				}
			}
		}
		println("ExwPacerNames: done.");
	}

	private Function fnAt(String addrStr) {
		Address addr = parse(addrStr);
		return addr != null ? currentProgram.getFunctionManager().getFunctionAt(addr) : null;
	}

	private void writeXrefs(PrintWriter out, String addrStr) {
		Address addr = parse(addrStr);
		if (addr == null) {
			out.println("SKIP " + addrStr + " bad address");
			return;
		}
		out.println("refs-to: " + addrStr);
		ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(addr);
		boolean any = false;
		while (it.hasNext()) {
			Reference ref = it.next();
			Function fromFn =
				currentProgram.getFunctionManager().getFunctionContaining(ref.getFromAddress());
			String from = fromFn != null
					? fromFn.getName() + "@" + fromFn.getEntryPoint() : "<no function>";
			out.println("  " + ref.getFromAddress() + "\t" + from + "\t" + ref.getReferenceType());
			any = true;
		}
		if (!any) {
			out.println("  <no references>");
		}
	}

	private void section(PrintWriter out, String name) {
		out.println();
		out.println("===== SECTION: " + name + " =====");
		println("ExwPacerNames: section " + name);
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
