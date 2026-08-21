import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

// P4.2/W1 probe 7: FUN_000596ed full decompile (mission-session driver)
// + callers of FUN_0001c7dc (mission tick+draw monolith).
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe7 extends GhidraScript {
  private static final int DT = 180;
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe7.txt";
    DecompInterface di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    try (PrintWriter w = new PrintWriter(out, "UTF-8")) {
      w.println("# EXD probe 7: mission-session driver full");
      w.println("== CALLERS of 0001c7dc ==");
      Function f = getFunctionAt(toAddr(0x1c7dcL));
      if (f != null) {
        for (Reference r : getReferencesTo(f.getEntryPoint())) {
          if (r.getReferenceType().isCall()) {
            Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
            w.println(String.format("CALL from %08x [%s]", r.getFromAddress().getOffset(),
              cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
          }
        }
      }
      w.println();
      w.println("==== SESSION_DRIVER 000596ed FULL ====");
      Function fn = getFunctionAt(toAddr(0x596edL));
      DecompileResults r = di.decompileFunction(fn, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
        w.print(r.getDecompiledFunction().getC());
      } else w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
    di.dispose();
    println("EXDPROBE7 done");
  }
}
