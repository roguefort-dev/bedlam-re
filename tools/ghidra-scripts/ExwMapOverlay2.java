/*-
 * ExwMapOverlay2.java - P4 map-overlay family RE, round 2: the data
 * sources + the toggle consumption. Questions:
 *   1. FUN_00422171 (MAPTRAN loader) + FUN_0042209b (PALTRAN loader):
 *      the .TRN parse - what lands at 0x45cdd8 (type -> color word).
 *   2. XRef census: 0x4edd9c (4x4 mask bank), 0x4dd464 + 0x4dd444
 *      (ramp tables), 0x4c420c (per-tile variant byte array),
 *      0x4eb8d0/d4/d8 (active order staging), 0x4eaaee/0x4eaaf2/
 *      0x4eab0c (per-player order staging), 0x4edb88 (game mode).
 *   3. ASM windows: MissionShell 0x447860..0x4478a0 (overlay-bit
 *      entry zeroing), 0x448700..0x448770 (0x4eb8dc countdown),
 *      0x448090..0x448140 (0x4ede34/0x4ea8f8 writers),
 *      FUN_00403938 head 0x403930..0x4039b0 + tail 0x407268..0x4072c0,
 *      FUN_0040b835 0x40b840..0x40b8b0, FUN_00410644 0x410650..0x410690,
 *      FUN_0044764c 0x447690..0x4476c0, load_mission 0x41de70..0x41df10.
 *   4. DECOMP FUN_0044874b (MissionShell helper reading 0x4e44f8).
 * Ghidra discipline: -process BEDLAM.EXW -noanalysis, never import.
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ExwMapOverlay2 extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x00422171", // MAPTRAN loader
        "0x0042209b", // PALTRAN loader
        "0x0044874b", // MissionShell helper (order staging reader)
    };

    private static final String[] XREF_TARGETS = {
        "0x004edd9c", "0x004dd464", "0x004dd444", "0x004c420c",
        "0x004eb8d0", "0x004eb8d4", "0x004eb8d8",
        "0x004eaaee", "0x004eaaf2", "0x004eab0c", "0x004edb88",
        "0x0045cdd8",
    };

    private static final String[][] WINDOWS = {
        {"0x00447860", "0x004478a0"},   // MissionShell entry zeroing
        {"0x00448700", "0x00448770"},   // 0x4eb8dc countdown
        {"0x00448090", "0x00448140"},   // 0x4ede34 / 0x4ea8f8 writers
        {"0x00403930", "0x004039b0"},   // FUN_00403938 head
        {"0x00407268", "0x004072c0"},   // FUN_00403938 tail site 3
        {"0x0040b840", "0x0040b8b0"},   // mouse_l_click router
        {"0x00410650", "0x00410690"},   // FUN_00410644
        {"0x00447690", "0x004476c0"},   // FUN_0044764c
        {"0x0041de70", "0x0041df10"},   // load_mission 0x4e44f8 staging
    };

    private Address addr(String hex) throws Exception {
        return currentProgram.getAddressFactory().getDefaultAddressSpace().getAddress(hex);
    }

    private void dumpFunction(Address entry) throws Exception {
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
        while (insn != null && count < 1500 && f.getBody().contains(insn.getAddress())) {
            out.println(insn.getAddress() + " " + insn);
            insn = getInstructionAfter(insn);
            count++;
        }
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: ExwMapOverlay2 <outFile>");
        }
        try (PrintWriter w =
            new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
                StandardCharsets.UTF_8))) {
            out = w;
            decomp = new DecompInterface();
            decomp.setOptions(new DecompileOptions());
            decomp.openProgram(currentProgram);
            for (String root : ROOTS) {
                dumpFunction(addr(root));
            }
            for (String t : XREF_TARGETS) {
                Address a = addr(t);
                out.println("----- XREF TO " + t + " -----");
                ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(a);
                while (it.hasNext()) {
                    Reference r = it.next();
                    Function f = currentProgram.getFunctionManager()
                        .getFunctionContaining(r.getFromAddress());
                    out.println("  " + r.getFromAddress() + " " + r.getReferenceType() + " in " +
                        (f != null ? f.getName() : "<nofunc>"));
                }
            }
            for (String[] win : WINDOWS) {
                out.println("===== ASM WINDOW " + win[0] + ".." + win[1] + " =====");
                Address a = addr(win[0]);
                Address end = addr(win[1]);
                int i = 0;
                while (a != null && a.compareTo(end) < 0 && i < 200) {
                    Instruction insn = getInstructionAt(a);
                    if (insn == null) {
                        break;
                    }
                    out.println(insn.getAddress() + " " + insn);
                    a = insn.getAddress().add(insn.getLength());
                    i++;
                }
            }
            out.println("===== DONE =====");
        }
        decomp.dispose();
        println("ExwMapOverlay2: done.");
    }
}
