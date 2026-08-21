import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

// P4.2/W1 probe 4: loader decompiles (bank homes / arrays / type table /
// spawn stagger) + MissionShell hunt via map-loader callers + 0xfa0 money
// store windows. -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe4 extends GhidraScript {
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
  private void callersOf(String hdr, long addr) {
    w.println();
    w.println("== CALLERS of " + String.format("%08x", addr) + " ==");
    Function f = getFunctionAt(toAddr(addr));
    if (f == null) { w.println("// no function"); return; }
    for (Reference r : getReferencesTo(f.getEntryPoint())) {
      if (r.getReferenceType().isCall()) {
        Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
        w.println(String.format("CALL from %08x  in %s", r.getFromAddress().getOffset(),
          cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
      }
    }
  }
  private void insWin(String hdr, long lo, long hi) {
    w.println();
    w.println("== INS " + hdr + " ==");
    for (long a = lo; a < hi;) {
      Instruction ins = getInstructionAt(toAddr(a));
      if (ins == null) { a++; continue; }
      w.println(String.format("INS %08x %s", a, ins.toString()));
      a += ins.getLength();
    }
  }
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe4.txt";
    try { w = new PrintWriter(out, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# EXD probe 4: loader bodies + MissionShell hunt + money stores");
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);

    callersOf("MAP_LOADER 2e5c3", 0x2e5c3L);
    callersOf("NME_LOADER 26dc1", 0x26dc1L);
    decompTo("MAP_LOADER", 0x2e5c3L, 340);
    decompTo("POS_BDG_LOADER", 0x2adb4L, 300);
    decompTo("TRT_LOADER", 0x279e3L, 200);
    decompTo("MRK_LOADER_SPAWN", 0x1d9cdL, 340);
    decompTo("PLATFORM_RING_337f4", 0x337f4L, 200);
    insWin("0xfa0 ctx A (4c80c)", 0x4ccc0L, 0x4cd00L);
    insWin("0xfa0 ctx B (4c80c)", 0x4e290L, 0x4e2d0L);
    insWin("0xfa0 ctx C (2e27d)", 0x2e360L, 0x2e3a0L);

    w.flush(); w.close();
    println("EXDPROBE4 done -> " + out);
    di.dispose();
  }
}
