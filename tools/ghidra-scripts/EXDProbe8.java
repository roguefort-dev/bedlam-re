import java.io.PrintWriter;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Instruction;

// P4.2/W1 probe 8: instruction windows around FUN_000596ed's five
// PresentFlip sites (find the mission-loop counter increment form).
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe8 extends GhidraScript {
  private void insWin(PrintWriter w, String hdr, long lo, long hi) {
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
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe8.txt";
    try (PrintWriter w = new PrintWriter(out, "UTF-8")) {
      w.println("# EXD probe 8: mission-loop tail windows");
      insWin(w, "flip1 0x597c9", 0x59790L, 0x597e0L);
      insWin(w, "flip2 0x59811", 0x597f0L, 0x59830L);
      insWin(w, "flip3 0x59c70", 0x59c30L, 0x59c90L);
      insWin(w, "flip4 0x5a640", 0x5a600L, 0x5a670L);
      insWin(w, "flip5 0x5a6eb", 0x5a6b0L, 0x5a720L);
    }
    println("EXDPROBE8 done");
  }
}
