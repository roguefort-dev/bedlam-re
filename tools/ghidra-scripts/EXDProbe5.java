import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.listing.Instruction;

// P4.2/W1 probe 5: DEADMAN loader (SFX banks+gate), animator twin
// (latch/pod ring), move-target array refs (order path / selection).
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe5 extends GhidraScript {
  private static final int DT = 120;
  private DecompInterface di;
  private PrintWriter w;
  private void decompTo(String hdr, long addr, int maxLines) {
    w.println();
    w.println("==== " + hdr + " " + String.format("%08x", addr) + " ====");
    Function fn = getFunctionAt(toAddr(addr));
    if (fn == null) { w.println("// no function at " + String.format("%08x", addr)); return; }
    w.println("// fn size " + fn.getBody().getNumAddresses());
    DecompileResults r = di.decompileFunction(fn, DT, monitor);
    if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
      String[] lines = r.getDecompiledFunction().getC().split("[\r\n]+");
      for (int i = 0; i < lines.length && i < maxLines; i++) w.println(lines[i]);
      if (lines.length > maxLines) w.println("// TRUNCATED " + (lines.length - maxLines));
    } else w.println("// DECOMP FAILED: " + r.getErrorMessage());
  }
  private void refsTo(String hdr, long addr, int max) {
    w.println();
    w.println("== REFS to " + String.format("%08x", addr) + " ==");
    int n = 0;
    for (Reference r : getReferencesTo(toAddr(addr))) {
      Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
      w.println(String.format("REF %08x %s [%s]", r.getFromAddress().getOffset(),
        r.getReferenceType().toString(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
      if (++n > max) { w.println("// capped"); break; }
    }
  }
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe5.txt";
    try { w = new PrintWriter(out, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# EXD probe 5: SFX loader + animator + order path");
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);

    // A. DEADMAN string refs -> SFX bank loader
    refsTo("DEADMAN1 str", 0x869eaL, 40);

    // B. animator twin full (latch + pod ring + payouts)
    decompTo("ANIMATOR_1f8c1", 0x1f8c1L, 420);

    // C. move-target array refs (order path)
    refsTo("MOVE_TGT_A f75ec", 0xf75ecL, 60);

    // D. per-player anchor refs (selection readers)
    refsTo("SEL_ANCHOR_4xC 971a4", 0x971a4L, 40);

    w.flush(); w.close();
    println("EXDPROBE5 done -> " + out);
    di.dispose();
  }
}
