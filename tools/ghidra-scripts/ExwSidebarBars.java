/*-
 * ExwSidebarBars.java - P4 sidebar bars + score strip RE. Questions:
 *   1. FUN_0040807f (HP + armor bars) exact semantics: clamps, sprite
 *      ids, coords, which record words feed each bar (+0x78 hp dword,
 *      +0x2E armor word, the +0x30 gate).
 *   2. FUN_004085ce (score/money strip, NUMBERS.BIN): digit layout,
 *      value clamps, which globals feed it (_DAT_004dd40c score,
 *      DAT_0046ae70 money).
 *   3. FUN_004072bf (portrait pass): the exact armor tick (+0x2E
 *      -1/frame clamp) semantics + the HP dither gate.
 *   4. FUN_0040eba0 (pickup consumer): all cases - case 4 score/money,
 *      case 8 ammo, and the ARMOR pickup (+0x2E producer).
 *   5. XRef census: 0x4dd40c (score), 0x46ae70 (money), 0x4c6a5c
 *      (hp robot-0 displacement), 0x4c6a12 (armor word), 0x4c6a14
 *      (+0x30 gate), 0x46af3c (NUMBERS.BIN bank), 0x46ccf0 (score
 *      strip countdown).
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

public class ExwSidebarBars extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x0040807f", // HP + armor bars
        "0x004085ce", // score/money strip
        "0x004072bf", // portrait pass (armor tick)
        "0x0040eba0", // pickup consumer
    };

    private static final String[] XREF_TARGETS = {
        "0x004dd40c", "0x0046ae70", "0x004c6a5c", "0x004c6a12",
        "0x004c6a14", "0x0046af3c", "0x0046ccf0",
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
        while (insn != null && count < 2500 && f.getBody().contains(insn.getAddress())) {
            out.println(insn.getAddress() + " " + insn);
            insn = getInstructionAfter(insn);
            count++;
        }
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException("usage: ExwSidebarBars <outFile>");
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
            out.println("===== DONE =====");
        }
        decomp.dispose();
        println("ExwSidebarBars: done.");
    }
}
