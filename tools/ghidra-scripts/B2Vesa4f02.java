import java.io.PrintWriter;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Instruction;

// B2 residuals micro-pass: 4f02 set-mode BX (LFB bit?) + page/window mapper
// decompile context. NEVER re-import; -process BEDLAM.EXE -noanalysis only.
public class B2Vesa4f02 extends GhidraScript {
  private PrintWriter w;
  private void listingTo(String hdr, long addr, int maxUnits) {
    w.println();
    w.println("---- listing " + hdr + " " + String.format("%08x", addr) + " ----");
    Instruction ins = currentProgram.getListing().getInstructionAt(toAddr(addr));
    int n = 0;
    while (ins != null && n < maxUnits) {
      w.println(String.format("%08x  %s", ins.getAddress().getOffset(), ins.toString()));
      ins = currentProgram.getListing().getInstructionAfter(ins.getAddress());
      n++;
    }
    if (n == 0) w.println("// NO INSTRUCTION AT " + String.format("%08x", addr));
  }
  private void listingBack(String hdr, long addr, int maxUnits) {
    w.println();
    w.println("---- backwalk " + hdr + " from " + String.format("%08x", addr) + " ----");
    java.util.List<String> lines = new java.util.ArrayList<>();
    ghidra.program.model.listing.Instruction ins = currentProgram.getListing().getInstructionAt(toAddr(addr));
    int n = 0;
    while (ins != null && n < maxUnits) {
      lines.add(String.format("%08x  %s", ins.getAddress().getOffset(), ins.toString()));
      ins = currentProgram.getListing().getInstructionBefore(ins.getAddress());
      n++;
    }
    for (int i = lines.size() - 1; i >= 0; i--) w.println(lines.get(i));
    if (lines.isEmpty()) w.println("// NO INSTRUCTION AT " + String.format("%08x", addr));
  }
  @Override
  public void run() throws Exception {
    try { w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-vesa-4f02.txt", "UTF-8"); }
    catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 4f02 BX probe + page mappers");
    listingBack("4f02 prelude", 0x12439L, 14);
    listingTo("4f02 exact", 0x12439L, 30);
    
    listingTo("init 4f07 region", 0x12476L, 36);
    listingTo("page mapper 128df", 0x128dfL, 70);
    listingTo("page mapper 12960", 0x12960L, 70);
    listingTo("page mapper 129f2", 0x129f2L, 70);
    w.close();
    println("B2Vesa4f02 DONE");
  }
}
