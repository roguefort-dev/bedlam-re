/*-
 * ExwMissionRender.java - P4 mission-render RE slice. Target: the
 * isometric viewport terrain draw chain (init_tiles cache DAT_004ede24
 * consumers + the tile render buffer _DAT_004ede18 writers), ZONEA/
 * MISSION1 parity. Functions located via exw-simtail.txt navigation:
 *   FUN_00410823  called 0..3 in the MissionShell loop right before
 *                 FUN_00412010 (FX) - prime terrain-draw candidate
 *   FUN_00403938  entity pass (already decompiled in exw-font-strings)
 *   FUN_00401107  present: 480x480 window from the 0x64000 buffer
 *   FUN_0040798e  masked sprite draw used by FUN_00403938
 *   FUN_004254e1  called right before init_tiles at setup
 *   FUN_0042c4a0  called right before that
 * This script: (1) XREF census of 0x4ede24/0x4ede28/0x4ede18/0x4edde4/
 * 0x4edde8 (all read/write references with containing function), then
 * (2) full decompile + instruction listing of the ROOTS below with
 * depth-1 callee closure (callees > 3000 bytes skipped at depth).
 * Ghidra discipline: -process BEDLAM.EXW -noanalysis, never import.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

public class ExwMissionRender extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x00410823", // terrain draw candidate (0..3 loop in MissionShell)
        "0x0040798e", // masked sprite draw (FUN_00403938 callee)
        "0x004012f7", // present helper (wipe/scale)
        "0x004013e8", // present helper (row scale)
        "0x004254e1", // setup before init_tiles
        "0x0042c4a0", // setup before that
        "0x004197d4", // every-other-0..3 pass
    };

    private static final String[] XREF_TARGETS = {
        "0x004ede24", // viewport tile cache ptr
        "0x004ede28", // viewport tile cache count
        "0x004ede18", // tile render buffer ptr
        "0x004edde4", // camera x (Q5)
        "0x004edde8", // camera y (Q5)
    };

    private Address addr(String hex) throws Exception {
        return currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
    }

    private Function fnAt(Address a) {
        return currentProgram.getFunctionManager().getFunctionContaining(a);
    }

    private void dumpFunc(Function fn, int depth, Set<Function> seen) throws Exception {
        if (fn == null || seen.contains(fn)) {
            return;
        }
        seen.add(fn);
        out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName()
            + " (size=" + fn.getBody().getNumAddresses() + ") -----");
        DecompileResults r = decomp.decompileFunction(fn, 180, monitor);
        if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
            out.println(r.getDecompiledFunction().getC());
        } else {
            out.println("// DECOMP FAILED: " + r.getErrorMessage());
            return;
        }
        var it = currentProgram.getListing().getInstructions(fn.getBody(), true);
        while (it.hasNext()) {
            out.println(it.next().toString());
        }
        if (depth <= 0) {
            return;
        }
        for (Function callee : fn.getCalledFunctions(monitor)) {
            long size = callee.getBody().getNumAddresses();
            if (size > 3000) {
                out.println("// (skip large callee " + callee.getEntryPoint() + " "
                    + callee.getName() + " size=" + size + ")");
                continue;
            }
            dumpFunc(callee, depth - 1, seen);
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

            out.println("===== XREF CENSUS =====");
            for (String hex : XREF_TARGETS) {
                out.println("--- refs to " + hex + " ---");
                for (Reference ref : currentProgram.getReferenceManager()
                        .getReferencesTo(addr(hex))) {
                    Function c = fnAt(ref.getFromAddress());
                    out.println("  " + ref.getReferenceType() + " at " + ref.getFromAddress()
                        + " in " + (c == null ? "(no fn)" : c.getEntryPoint() + " " + c.getName()));
                }
            }

            out.println("===== ROOTS =====");
            for (String hex : ROOTS) {
                Function f = fnAt(addr(hex));
                out.println("  " + hex + " -> "
                    + (f == null ? "(no fn)" : f.getEntryPoint() + " " + f.getName()
                        + " size=" + f.getBody().getNumAddresses()));
            }

            Set<Function> seen = new LinkedHashSet<>();
            for (String hex : ROOTS) {
                Function f = fnAt(addr(hex));
                if (f == null) {
                    out.println("// NO FUNCTION AT " + hex);
                    continue;
                }
                out.println("########## ROOT " + hex + " ##########");
                dumpFunc(f, 1, seen);

                out.println("===== CALLERS OF " + f.getEntryPoint() + " =====");
                for (Reference ref : currentProgram.getReferenceManager()
                        .getReferencesTo(f.getEntryPoint())) {
                    if (!ref.getReferenceType().isCall()) {
                        continue;
                    }
                    Function c = fnAt(ref.getFromAddress());
                    out.println("  call at " + ref.getFromAddress() + " in "
                        + (c == null ? "(no fn)" : c.getEntryPoint() + " " + c.getName()));
                }
            }
            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwMissionRender: done.");
    }
}
