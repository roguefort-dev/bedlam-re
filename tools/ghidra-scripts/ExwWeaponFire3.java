/*-
 * ExwWeaponFire3.java - P4 7j.15 weapon-fire family THIRD HOP slice.
 * Roots (all re-anchored against EXW in BedlamWatcom/BEDLAM.EXW):
 *   0x00419aff  FUN_00419aff (381 B, 28 callers) - the per-weapon
 *               STAT lookup feeding every damage argument; base
 *               field 0x46cbf8 per the 7j.13 census. Goal: full
 *               table layout (which id -> which damage/field).
 *   producer census for the TERRAIN-STRUCTURE array 0x4cccf8:
 *               xrefs to 0x4cccf8 / 0x4ccd08 (+0x10 hp) /
 *               0x4ccd14 (+0x1C z) + the count writer 0x46ccd4
 *               (+0x4cccd8 guard + 0x46cbf8 stat base for context).
 * Output: full decompile + listing per root, depth-1 callee closure,
 * caller census with 0x40-byte pre-call arg windows, xref census
 * with +/-3 instruction context, section/memory probes.
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
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;

public class ExwWeaponFire3 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x00419aff", // per-weapon stat lookup
    };

    private static final String[] XREF_TARGETS = {
        "0x004cccf8", // terrain-structure rec base
        "0x004ccd08", // +0x10 (hp dword)
        "0x004ccd14", // +0x1C (z level)
        "0x0046ccd4", // count writer
        "0x004cccd8", // id-0 guard (context)
        "0x0046cbf8", // weapon-stat base field (context)
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

    private void dumpCallWindow(Address callSite, int back) {
        out.println("  --- pre-call window at " + callSite + " ---");
        Address lo = callSite.subtract(back);
        Address hi = callSite.add(6);
        InstructionIterator it = currentProgram.getListing()
            .getInstructions(new AddressSet(lo, hi), true);
        while (it.hasNext()) {
            Instruction ins = it.next();
            out.println("    " + ins.getAddress() + "  " + ins);
        }
    }

    private void dumpContext(Address site, int before, int after) {
        Address lo = site.subtract(before);
        Address hi = site.add(after);
        InstructionIterator it = currentProgram.getListing()
            .getInstructions(new AddressSet(lo, hi), true);
        while (it.hasNext()) {
            Instruction ins = it.next();
            String mark = ins.getAddress().equals(site) ? " >>" : "   ";
            out.println("  " + mark + " " + ins.getAddress() + "  " + ins);
        }
    }

    private void probeMemory(String hex, int len) {
        try {
            Address a = addr(hex);
            Memory m = currentProgram.getMemory();
            String blk = m.getBlock(a) == null ? "<noblock>"
                : m.getBlock(a).getName() + "(init=" + m.getBlock(a).isInitialized() + ")";
            out.println("----- MEM " + hex + " block=" + blk + " -----");
            byte[] buf = new byte[len];
            m.getBytes(a, buf);
            for (int i = 0; i < len; i += 16) {
                StringBuilder hexs = new StringBuilder();
                for (int j = 0; j + i < len && j < 16; j++) {
                    hexs.append(String.format("%02x ", buf[i + j] & 0xff));
                }
                out.println("  +" + String.format("%03x", i) + "  " + hexs);
            }
        } catch (Exception e) {
            out.println("----- MEM " + hex + " FAILED: " + e + " -----");
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
                    dumpCallWindow(ref.getFromAddress(), 0x40);
                }
            }

            out.println("########## XREF CENSUS ##########");
            for (String hex : XREF_TARGETS) {
                Address target = addr(hex);
                out.println("===== XREFS TO " + hex
                    + (fnAt(target) != null ? " (fn " + fnAt(target).getName() + ")" : "")
                    + " =====");
                int n = 0;
                for (Reference ref : currentProgram.getReferenceManager()
                        .getReferencesTo(target)) {
                    n++;
                    Function c = fnAt(ref.getFromAddress());
                    out.println("  [" + n + "] " + ref.getFromAddress() + " "
                        + ref.getReferenceType() + " in "
                        + (c == null ? "<nofunc>" : c.getEntryPoint() + " " + c.getName()));
                    dumpContext(ref.getFromAddress(), 0x18, 0x18);
                }
                if (n == 0) {
                    out.println("  (no references)");
                }
            }

            out.println("########## MEMORY PROBES ##########");
            probeMemory("0x0046cbf8", 0x60); // weapon-stat base field region head
            probeMemory("0x004cccd8", 0x40); // guard + rec[0] head
            probeMemory("0x0046ccb0", 0x40); // words around the count/guard block

            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwWeaponFire3: done.");
    }
}
