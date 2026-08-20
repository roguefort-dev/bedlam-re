/*-
 * ExwMissionRender3.java - leaf blits of the terrain draw chain found
 * in exw-missionrender2.txt (FUN_00403938 lines: terrain loop uses
 * FUN_00401471 tile blit, FUN_0040179b sprite-list flush, FUN_00408030
 * off-map edge, FUN_0040167a water remap). Also FUN_0041df10 re-dump
 * with instruction listing context (the type-DB/CGR setup pass) and
 * the cursor transforms FUN_00440a2d / FUN_00440c34 for the inverse
 * projection constants.
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

public class ExwMissionRender3 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x00401471", // tile sprite blit (terrain loop leaf)
        "0x0040179b", // sprite-list flush blit
        "0x00408030", // off-map edge selector
        "0x0040167a", // water/LNK remap path
        "0x0041df10", // type-DB/CGR setup pass
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
        DecompileResults r = decomp.decompileFunction(fn, 240, monitor);
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
        println("ExwMissionRender3: done.");
    }
}
