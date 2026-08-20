/*-
 * ExwBootAttract2.java - follow-up for queue item 1: decompile the
 * movie RUNNER FUN_0044567c itself plus its called-function closure,
 * the Smacker init/shutdown bracket (FUN_0042582a / FUN_00425851),
 * and list callers of 0044567c / 004459f7. Ghidra discipline:
 * -process BEDLAM.EXW -noanalysis only, never re-import.
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

public class ExwBootAttract2 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private Function fnAt(String hex) throws Exception {
        Address a = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
        return currentProgram.getFunctionManager().getFunctionContaining(a);
    }

    private void dumpFunc(Function fn, int depth, Set<Function> seen) throws Exception {
        if (fn == null || seen.contains(fn)) {
            return;
        }
        seen.add(fn);
        out.println("----- DECOMP " + fn.getEntryPoint() + " " + fn.getName() + " -----");
        DecompileResults r = decomp.decompileFunction(fn, 90, monitor);
        if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
            out.println(r.getDecompiledFunction().getC());
        } else {
            out.println("// DECOMP FAILED: " + r.getErrorMessage());
            return;
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
            dumpFunc(fnAt("0044567c"), 2, seen);

            for (String a : new String[] { "0042582a", "00425851", "00445aab" }) {
                dumpFunc(fnAt(a), 1, seen);
            }

            for (String sym : new String[] { "0044567c", "004459f7", "0042582a", "00425851" }) {
                out.println("===== CALLERS OF " + sym + " =====");
                Address sa = currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(sym);
                for (Reference ref : currentProgram.getReferenceManager().getReferencesTo(sa)) {
                    if (!ref.getReferenceType().isCall()) {
                        continue;
                    }
                    Function caller = currentProgram.getFunctionManager()
                        .getFunctionContaining(ref.getFromAddress());
                    out.println("  from " + ref.getFromAddress() + " in "
                        + (caller == null ? "?" : caller.getEntryPoint() + " " + caller.getName()));
                }
            }
            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwBootAttract2: done.");
    }
}
