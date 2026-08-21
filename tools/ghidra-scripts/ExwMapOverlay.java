/*-
 * ExwMapOverlay.java - P4 map-overlay family RE (queue item: the
 * strategic-map overlay). Questions this run answers:
 *   1. FUN_004089b1 (size 1051) - the full overlay draw: the
 *      0x408c94..0x408dc4 order-target loop + robot/PAD marker
 *      placement (7d sketched the head only).
 *   2. FUN_00402ab8 (size 63) - the per-tile coloring primitive.
 *   3. XRef census of the toggle family: 0x4edba0 (overlay bit),
 *      0x4eb8dc (=5 writer's consumers), 0x4ede34, 0x4ea8f8,
 *      the map buffer 0x4ede18, the tile-color table 0x45cdd8,
 *      the PAD/order staging 0x4e44f8.
 *   4. The MAPTRAN/PALTRAN .TRN string block 0x458c00..0x458c60 +
 *      references into it (to pin the loader / table producer).
 *   5. ASM window of the FUN_00403938 tail 0x4071a0..0x407260 (the
 *      overlay-vs-sidebar per-frame consumption).
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
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwMapOverlay extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x004089b1", // map overlay draw (TABLE.BIN backdrop + markers)
        "0x00402ab8", // tile coloring primitive
    };

    private static final String[] XREF_TARGETS = {
        "0x004edba0", "0x004eb8dc", "0x004ede34", "0x004ea8f8",
        "0x004ede18", "0x0045cdd8", "0x0045cdda", "0x004e44f8",
        "0x004e4500", "0x0046cbbc",
    };

    private Address addr(String hex) throws Exception {
        return currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
    }

    private void dumpFunction(Address entry, boolean withCallees) throws Exception {
        Function f = currentProgram.getFunctionManager().getFunctionAt(entry);
        if (f == null) {
            out.println("----- NO FUNCTION AT " + entry + " -----");
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
                if (cf != null && sz <= 2000 && done.add(c.toString())) {
                    DecompileResults r2 = decomp.decompileFunction(cf, 90, monitor);
                    if (r2 != null && r2.getDecompiledFunction() != null) {
                        out.println("----- DECOMP " + c + " " + cf.getName() + " -----");
                        out.println(r2.getDecompiledFunction().getC());
                    }
                }
            }
        }
    }

    private void dumpXrefs(String hex) throws Exception {
        Address a = addr(hex);
        out.println("----- XREF TO " + hex + " -----");
        ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(a);
        while (it.hasNext()) {
            Reference r = it.next();
            Function f = currentProgram.getFunctionManager()
                .getFunctionContaining(r.getFromAddress());
            out.println("  " + r.getFromAddress() + " " + r.getReferenceType() + " in " +
                (f != null ? f.getName() : "<nofunc>"));
        }
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: ExwMapOverlay <outFile>");
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
            for (String t : XREF_TARGETS) {
                dumpXrefs(t);
            }
            // 4. string block + refs into it
            out.println("===== BYTES 0x00458c00..0x00458c60 =====");
            Address s = addr("0x00458c00");
            byte[] buf = new byte[0x60];
            currentProgram.getMemory().getBytes(s, buf);
            StringBuilder hex = new StringBuilder();
            StringBuilder asc = new StringBuilder();
            for (int i = 0; i < buf.length; i++) {
                int b = buf[i] & 0xff;
                hex.append(String.format("%02x ", b));
                asc.append((b >= 0x20 && b < 0x7f) ? (char) b : '.');
                if ((i & 15) == 15) {
                    out.println(String.format("%04x  %-48s |%s|",
                        0x458c00 + (i & ~15), hex, asc));
                    hex.setLength(0);
                    asc.setLength(0);
                }
            }
            out.println("----- REFS INTO 0x00458c00..0x00458c60 -----");
            for (long v = 0x458c00L; v < 0x458c60L; v++) {
                Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
                    .getAddress(v);
                ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(a);
                while (it.hasNext()) {
                    Reference r = it.next();
                    Function f = currentProgram.getFunctionManager()
                        .getFunctionContaining(r.getFromAddress());
                    out.println("  ->" + Long.toHexString(v) + " from " + r.getFromAddress() +
                        " " + r.getReferenceType() + " in " +
                        (f != null ? f.getName() : "<nofunc>"));
                }
            }
            // 5. terrain-loop tail window
            out.println("===== ASM WINDOW 0x004071a0..0x00407260 =====");
            Address a = addr("0x004071a0");
            for (int i = 0; i < 90 && a != null; i++) {
                Instruction insn = getInstructionAt(a);
                if (insn == null) {
                    break;
                }
                out.println(insn.getAddress() + " " + insn);
                a = insn.getAddress().add(insn.getLength());
            }
            out.println("===== DONE =====");
        }
        decomp.dispose();
        println("ExwMapOverlay: done.");
    }
}
