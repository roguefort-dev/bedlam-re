/*-
 * ExwWeaponFire2.java - P4 7j.14 weapon-fire family SECOND HOP slice.
 * Roots (all re-anchored against EXW in BedlamWatcom/BEDLAM.EXW):
 *   0x0041bc1c  terrain/robot damage resolver (312 B, 10 callers) —
 *               paired with FUN_0041a894 at every fire/impact site
 *               (first hop done 7j.13)
 *   0x0041eaa1  terrain probe (135 B, 3 callers; FUN_00412010 proj tick)
 *   0x004124a4  debris co-stager A (568 B, 9 callers) — HEAD/arg map only
 *   0x004126dc  debris co-stager B (364 B, 6 callers) — HEAD/arg map only
 * Output: full decompile + instruction listing per root, depth-1 callee
 * closure (callees > 3000 bytes skipped at depth), caller census per root,
 * and a 0x30-byte pre-call instruction window (arg setup) for every call
 * site of each root.
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
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;

public class ExwWeaponFire2 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x0041bc1c", // terrain/robot damage resolver
        "0x0041eaa1", // terrain probe
        "0x004124a4", // debris co-stager A
        "0x004126dc", // debris co-stager B
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

    private void dumpCallWindow(Address callSite) {
        out.println("  --- pre-call window at " + callSite + " ---");
        Address lo = callSite.subtract(0x30);
        Address hi = callSite.add(6);
        InstructionIterator it = currentProgram.getListing()
            .getInstructions(new AddressSet(lo, hi), true);
        while (it.hasNext()) {
            Instruction ins = it.next();
            out.println("    " + ins.getAddress() + "  " + ins);
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
                    dumpCallWindow(ref.getFromAddress());
                }
            }
            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwWeaponFire2: done.");
    }
}
