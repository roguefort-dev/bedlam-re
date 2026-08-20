/*-
 * ExwMissionRender2.java - follow-up to ExwMissionRender: full dump of
 * FUN_00403938 (the terrain+entity viewport renderer - xref census in
 * exw-missionrender.txt shows it reads the init_tiles cache 0x4ede24/
 * 0x4ede28 and the tile render buffer 0x4ede18) plus the other two
 * cache readers FUN_00440a2d / FUN_00440c34 (inverse transform /
 * click-to-tile candidates). No callee closure (the roots are big).
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

public class ExwMissionRender2 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x00403938", // terrain+entity viewport renderer
        "0x00440a2d", // tile-cache reader (cursor transform?)
        "0x00440c34", // tile-cache reader (click-to-tile?)
    };

    private Address addr(String hex) throws Exception {
        return currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
    }

    private Function fnAt(Address a) {
        return currentProgram.getFunctionManager().getFunctionContaining(a);
    }

    private void dumpFunc(Function fn, Set<Function> seen) throws Exception {
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
                dumpFunc(f, seen);

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
        println("ExwMissionRender2: done.");
    }
}
