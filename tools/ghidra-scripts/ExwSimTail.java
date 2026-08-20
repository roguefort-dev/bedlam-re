/*-
 * ExwSimTail.java - P2d sim-tail RE slice. Target: the EXW mission-sim
 * tail the P4 vertical slice needs (one squad-member move on
 * ZONEA/MISSION1). Functions located via the 8street disasm (navigation
 * reference only; every fact re-anchored here against EXW):
 *   gameplay loop  FUN_0044771c (8street "game_level", called from
 *                  GameMain; GameMain switches on its return = outcome)
 *   units tick     FUN_0040b9f6 ("robots")
 *   mover tick     FUN_0040c536 ("robot_move")
 *   move x/y       FUN_0040cac2 / FUN_0040cb4f ("move_x_who"/"move_y_who")
 *   walkability    FUN_0040cbda ("move_is_possible2") ->
 *                  FUN_0041e897 ("move_is_possible")
 *   L-click        FUN_0040b835 ("mouse_l_click"),
 *                  FUN_0040d197 ("sidebar_control")
 *   spawn loader   FUN_0040cca0 ("load_markers_mrk_file")
 *   tile init      FUN_00407e11 ("init_tiles")
 *   DAT reader     FUN_0041eb28 ("get_from_dat_file")
 *   z helper       FUN_0041e231 ("get_z_pos")
 * This script: full decompile + instruction listing of each, depth-1
 * callee closure (callees > 3000 bytes skipped at depth; roots never
 * skipped), callers census per root.
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

public class ExwSimTail extends GhidraScript {

    private PrintWriter out;
    private DecompInterface decomp;

    private static final String[] ROOTS = {
        "0x0044771c", // gameplay loop (game_level)
        "0x0040b9f6", // units tick (robots)
        "0x0040c536", // mover tick (robot_move)
        "0x0040cac2", // move_x_who
        "0x0040cb4f", // move_y_who
        "0x0040cbda", // move_is_possible2
        "0x0041e897", // move_is_possible
        "0x0040b835", // mouse_l_click
        "0x0040d197", // sidebar_control
        "0x0040cca0", // load_markers_mrk_file
        "0x00407e11", // init_tiles
        "0x0041eb28", // get_from_dat_file
        "0x0041e231", // get_z_pos
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

            // root list with sizes (resolve + report)
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
                }
            }
            out.println("===== DONE =====");
        } finally {
            if (decomp != null) decomp.dispose();
        }
        println("ExwSimTail: done.");
    }
}
