/*-
 * ExwTitleMenu.java - P2g title-menu RE slice. Target: the title-menu
 * screen code in EXW. Known entry: FUN_0043a5fc (predecessor-named
 * NameEntryScreen; called by GameMain right after the boot attract at
 * the outer restart point 0041c3d6). Data-side anchor: the LANGUAGE.*
 * [MENU_ITEMS] table is loaded at boot into 0046af5c (96 entries x
 * 0x30, ends 0x46b15c). This script:
 *   (1) reference census into 0046af5c..0046b15c (who reads menu
 *       strings) grouped by containing function;
 *   (2) full decompile + instruction listing of FUN_0043a5fc;
 *   (3) depth-1 callee closure (callees > 2500 bytes skipped at depth,
 *       root never skipped);
 *   (4) callers census of FUN_0043a5fc.
 * Ghidra discipline: -process BEDLAM.EXW -noanalysis, never import.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;
import java.util.TreeMap;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

public class ExwTitleMenu extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

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
            if (size > 2500) {
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

            // (1) reference census into the MENU_ITEMS table
            out.println("===== REFS INTO 0046af5c..0046b15c (MENU_ITEMS 96x0x30) =====");
            Address base = addr("0x0046af5c");
            TreeMap<String, Integer> byFn = new TreeMap<>();
            for (long off = 0; off <= 0x600; off++) {
                Address a = base.add(off);
                for (Reference ref : currentProgram.getReferenceManager().getReferencesTo(a)) {
                    Function f = fnAt(ref.getFromAddress());
                    String fs = (f == null ? "(no fn)"
                        : f.getEntryPoint() + " " + f.getName());
                    out.println("  target " + a + " (idx " + (off / 0x30) + " +0x"
                        + Long.toHexString(off % 0x30) + ") <- from " + ref.getFromAddress()
                        + " type=" + ref.getReferenceType() + " in " + fs);
                    byFn.merge(fs, 1, Integer::sum);
                }
            }
            out.println("===== MENU_ITEMS refs grouped by function =====");
            for (var e : byFn.entrySet()) {
                out.println("  " + e.getValue() + "x  " + e.getKey());
            }

            // (2)+(3) title screen decompile + closure
            Set<Function> seen = new LinkedHashSet<>();
            Function title = fnAt(addr("0x0043a5fc"));
            if (title == null) {
                out.println("// NO FUNCTION AT 0043a5fc");
            } else {
                dumpFunc(title, 1, seen);
            }

            // (4) callers census
            if (title != null) {
                out.println("===== CALLERS OF " + title.getEntryPoint() + " =====");
                for (Reference ref : currentProgram.getReferenceManager()
                        .getReferencesTo(title.getEntryPoint())) {
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
        println("ExwTitleMenu: done.");
    }
}
