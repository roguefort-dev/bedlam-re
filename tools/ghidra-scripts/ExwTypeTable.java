/*-
 * ExwTypeTable.java - P4 slice RE: the 0x4de664 weapon-loadout table's
 * provenance + consumers. Questions this run answers:
 *   1. FUN_004089b1 - the ONLY reader of the TABLE.BIN arena pointer
 *      (0x46cbbc, 3 xrefs total: alloc/load/read) - what does it do
 *      with the 160000-byte buffer (i.e. what IS TABLE.BIN)?
 *   2. FUN_00420260 - the compiled-in weapon-name switch: the exact
 *      index -> string mapping (for the sidebar row text wiring).
 *   3. The GameMain word@0x4edb90 player-TYPE write at 0x41c34c -
 *      exact instruction window.
 *   4. FUN_004437ea / FUN_00443870 - the shop find-free-group helpers
 *      (they define when a purchased weapon occupies table group k).
 * Full decompile + instruction listing per root, depth-1 callees.
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
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

public class ExwTypeTable extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x004089b1", // TABLE.BIN consumer (map overlay family)
        "0x00420260", // weapon-name switch
        "0x004437ea", // shop: find free weapon group
        "0x00443870", // shop: find free chassis group
    };

    private Address addr(String hex) throws Exception {
        return currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
    }

    private void dumpFunction(Address entry, boolean withCallees) throws Exception {
        Function f = currentProgram.getFunctionManager().getFunctionAt(entry);
        if (f == null) {
            out.println("----- NO FUNCTION AT " + entry + " -----");
            Instruction insn = getInstructionAt(entry);
            if (insn == null) {
                disassemble(entry);
                insn = getInstructionAt(entry);
            }
            int count = 0;
            Address a = entry;
            while (count < 400) {
                insn = getInstructionAfter(a);
                if (insn == null) {
                    break;
                }
                out.println(insn.getAddress() + " " + insn);
                a = insn.getAddress();
                count++;
            }
            return;
        }
        out.println("----- DECOMP " + entry + " " + f.getName() + " (size=" +
            f.getBody().getNumAddresses() + ") -----");
        DecompileResults r = decomp.decompileFunction(f, 120, monitor);
        if (r != null && r.getDecompiledFunction() != null) {
            out.println(r.getDecompiledFunction().getC());
        }
        else {
            out.println("// decompile failed: " + (r != null ? r.getErrorMessage() : "null"));
        }
        out.println("----- ASM " + entry + " -----");
        Instruction insn = getInstructionAt(f.getEntryPoint());
        int count = 0;
        while (insn != null && count < 2000 && f.getBody().contains(insn.getAddress())) {
            out.println(insn.getAddress() + " " + insn);
            insn = getInstructionAfter(insn);
            count++;
        }
        if (withCallees) {
            Set<String> done = new LinkedHashSet<>();
            Reference[] refs = getReferencesFrom(f.getEntryPoint());
            // walk all instructions, collect call targets
            Instruction i2 = getInstructionAt(f.getEntryPoint());
            Set<Address> callees = new LinkedHashSet<>();
            int n = 0;
            while (i2 != null && n < 5000 && f.getBody().contains(i2.getAddress())) {
                for (Reference ref : i2.getReferencesFrom()) {
                    if (ref.getReferenceType().isCall()) {
                        callees.add(ref.getToAddress());
                    }
                }
                i2 = getInstructionAfter(i2);
                n++;
            }
            out.println("----- CALLEES OF " + entry + " -----");
            for (Address c : callees) {
                Function cf = currentProgram.getFunctionManager().getFunctionAt(c);
                long sz = cf != null ? cf.getBody().getNumAddresses() : -1;
                out.println("  " + c + " " + (cf != null ? cf.getName() : "?") + " size=" + sz);
                if (cf != null && sz <= 3000 && done.add(c.toString())) {
                    DecompileResults r2 = decomp.decompileFunction(cf, 90, monitor);
                    if (r2 != null && r2.getDecompiledFunction() != null) {
                        out.println("----- DECOMP " + c + " " + cf.getName() + " -----");
                        out.println(r2.getDecompiledFunction().getC());
                    }
                }
            }
        }
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: ExwTypeTable <outFile>");
        }
        try (PrintWriter w =
            new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
                StandardCharsets.UTF_8))) {
            out = w;
            decomp = new DecompInterface();
            decomp.setOptions(new DecompileOptions());
            decomp.openProgram(currentProgram);
            for (String root : ROOTS) {
                dumpFunction(addr(root), true);
            }
            // 3. the GameMain type-write window
            out.println("===== WINDOW 0x0041c320..0x0041c370 =====");
            Address a = addr("0x0041c320");
            for (int i = 0; i < 40 && a != null; i++) {
                Instruction insn = getInstructionAt(a);
                if (insn == null) {
                    break;
                }
                out.println(insn.getAddress() + " " + insn);
                a = insn.getAddress().add(insn.getLength());
            }
            // strings near the weapon-name table for direct byte checks
            out.println("===== STRING BYTES 0x00458801..0x00458c40 (TABLE.BIN refs + names) =====");
            Address s = addr("0x00458801");
            byte[] buf = new byte[0x440];
            currentProgram.getMemory().getBytes(s, buf);
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < buf.length; i++) {
                int b = buf[i] & 0xff;
                sb.append(String.format("%02x", b));
                if ((i & 15) == 15) {
                    sb.append('\n');
                }
            }
            out.print(sb);
            out.println("===== DONE =====");
        }
        decomp.dispose();
        println("ExwTypeTable: done.");
    }
}
