/*-
 * ExwBrfDrop.java - queue item 1: find the EXW play site of
 * BRF_DROP.SMK. Binary facts first (strings/xxd, DGROUP off->VA =
 * +0x401a00): the string block at VA 0x4591c2.. holds two
 * "GAMEGFX\BRF_" prefixes (+ ".SMK" at 0x4591cf, + ".BIN" at
 * 0x4591e1), "SOUND\MIDI\BRIEF" 0x4591e5, the full literal
 * "GAMEGFX\BRF_DROP.SMK" 0x4591f7, a dedicated error
 * "ERROR: COULD NOT OPEN BRF_DROP SMACK\n" 0x45920c,
 * "GAMEGFX\BRFPAL.PAL" 0x459232, and a generic
 * "ERROR: COULD NOT OPEN %s SMACK\n" 0x459243.
 * This script: (1) every reference into 0x4591c2..0x459260,
 * (2) decompile each referencing function + small callee closure,
 * (3) callers of those functions (one hop) + decompile,
 * (4) callers of the menu-modal runner FUN_0043e7d4.
 * Ghidra discipline: -process BEDLAM.EXW -noanalysis, never import.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.LinkedHashSet;
import java.util.Set;
import java.util.TreeSet;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;

public class ExwBrfDrop extends GhidraScript {

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
        DecompileResults r = decomp.decompileFunction(fn, 120, monitor);
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
            if (size > 2500) {
                out.println("// (skip large callee " + callee.getEntryPoint() + " "
                    + callee.getName() + " size=" + size + ")");
                continue;
            }
            dumpFunc(callee, depth - 1, seen);
        }
    }

    private Set<Function> callersOf(Function fn) {
        Set<Function> callers = new LinkedHashSet<>();
        for (Reference ref : currentProgram.getReferenceManager()
                .getReferencesTo(fn.getEntryPoint())) {
            if (!ref.getReferenceType().isCall()) {
                continue;
            }
            Function caller = fnAt(ref.getFromAddress());
            if (caller != null) {
                callers.add(caller);
            }
        }
        return callers;
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

            Set<Function> hitFns = new LinkedHashSet<>();
            TreeSet<Long> hitTargets = new TreeSet<>();
            out.println("===== REFS INTO 0x4591c2..0x459260 (BRF string block) =====");
            Address base = addr("0x4591c2");
            for (long off = 0; off <= 0x9e; off++) {
                Address a = base.add(off);
                for (Reference ref : currentProgram.getReferenceManager().getReferencesTo(a)) {
                    Function f = fnAt(ref.getFromAddress());
                    out.println("  target " + a + " <- from " + ref.getFromAddress()
                        + " type=" + ref.getReferenceType() + " in "
                        + (f == null ? "(no fn)" : f.getEntryPoint() + " " + f.getName()));
                    hitTargets.add(a.getOffset());
                    if (f != null) {
                        hitFns.add(f);
                    }
                }
            }
            out.println("===== distinct referenced targets =====");
            for (Long t : hitTargets) {
                out.println("  " + Long.toHexString(t));
            }
            out.println("===== distinct containing functions =====");
            for (Function f : hitFns) {
                out.println("  " + f.getEntryPoint() + " " + f.getName());
            }

            Set<Function> seen = new LinkedHashSet<>();
            for (Function f : hitFns) {
                dumpFunc(f, 1, seen);
            }

            Set<Function> up = new LinkedHashSet<>();
            for (Function f : hitFns) {
                for (Function c : callersOf(f)) {
                    out.println("===== CALLER OF " + f.getEntryPoint() + " " + f.getName()
                        + ": " + c.getEntryPoint() + " " + c.getName() + " =====");
                    up.add(c);
                }
            }
            for (Function c : up) {
                dumpFunc(c, 0, seen);
            }

            Function modal = fnAt(addr("0x0043e7d4"));
            if (modal != null) {
                out.println("===== CALLERS OF FUN_0043e7d4 (menu-modal runner hyp) =====");
                for (Function c : callersOf(modal)) {
                    out.println("  " + c.getEntryPoint() + " " + c.getName());
                }
            }
            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwBrfDrop: done.");
    }
}
