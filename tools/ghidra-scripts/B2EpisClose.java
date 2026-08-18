import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;

// B2 episode-loop pass 2 (close-out): ISR decompile, orphan stub + real driver
// callback creation/decompile, VESA init + bank helpers, static table dumps.
// NEVER re-import; -process BEDLAM.EXE -noanalysis only.
public class B2EpisClose extends GhidraScript {
  private DecompInterface di;
  private PrintWriter w;
  private void decompTo(String hdr, long addr, int maxLines) {
    w.println();
    w.println("==== " + hdr + " " + String.format("%08x", addr) + " ====");
    Function fn = getFunctionAt(toAddr(addr));
    if (fn == null) { w.println("// no function at " + String.format("%08x", addr)); return; }
    w.println("// fn " + fn.getName() + " body " + fn.getBody().getNumAddresses() + " bytes");
    DecompileResults r = di.decompileFunction(fn, 120, monitor);
    if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
      String[] lines = r.getDecompiledFunction().getC().split("[\r\n]+");
      for (int i = 0; i < lines.length && i < maxLines; i++) w.println(lines[i]);
      if (lines.length > maxLines) w.println("// TRUNCATED " + (lines.length - maxLines) + " lines");
    } else {
      w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
  }
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
  }
  private void dumpMem(String hdr, long addr, int bytes) {
    Memory m = currentProgram.getMemory();
    byte[] buf = new byte[bytes];
    try { m.getBytes(toAddr(addr), buf); } catch (Exception e) { w.println(hdr + " READ FAIL " + e); return; }
    w.println();
    w.println("== " + hdr + " @ " + String.format("%08x", addr) + " (" + bytes + " bytes) ==");
    for (int i = 0; i < bytes; i += 16) {
      String hx = "", asc = "";
      for (int j = 0; j < 16 && i + j < bytes; j++) {
        int b = buf[i + j] & 0xff;
        hx += String.format("%02x ", b);
        asc += (b >= 0x20 && b < 0x7f) ? String.valueOf((char) b) : ".";
      }
      w.println(String.format("%08x  %-48s %s", addr + i, hx, asc));
    }
  }
  private void createFn(String fallback, long addr) {
    try {
      Address a = toAddr(addr);
      Function ex = getFunctionAt(a);
      if (ex != null) { w.println("createFn " + String.format("%08x", addr) + " ALREADY " + ex.getName()); return; }
      Function f = createFunction(a, fallback);
      w.println("createFn " + String.format("%08x", addr) + " -> " + (f == null ? "FAILED" : f.getName() + " body " + f.getBody().getNumAddresses()));
    } catch (Exception e) { w.println("createFn " + String.format("%08x", addr) + " EXC " + e); }
  }
  @Override
  public void run() throws Exception {
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    try { w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-epis-close.txt", "UTF-8"); }
    catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 episode-loop close-out pass 2");
    decompTo("ISR Int8TickHandler", 0x12734L, 700);
    listingTo("ISR patched dispatch slots", 0x127988L, 36);
    createFn("SndStubA", 0x12ecfL);
    createFn("SndStubB", 0x12ee4L);
    createFn("SndStubC", 0x12eefL);
    createFn("SndOrphan607b0", 0x607b0L);
    createFn("SndDrvCbA", 0x686b0L);
    createFn("SndDrvCbB", 0x686d0L);
    createFn("SndDrvCbC", 0x68740L);
    decompTo("stub A 801a6 user", 0x12ecfL, 120);
    decompTo("stub B", 0x12ee4L, 120);
    decompTo("stub C", 0x12eefL, 120);
    decompTo("orphan 80010 user", 0x607b0L, 120);
    decompTo("driver cb A", 0x686b0L, 250);
    decompTo("driver cb B", 0x686d0L, 250);
    decompTo("driver cb C", 0x68740L, 250);
    listingTo("stub region", 0x12ecfL, 44);
    listingTo("orphan region", 0x607a0L, 16);
    listingTo("driver cb region", 0x686a0L, 100);
    decompTo("WaitVRetrace", 0x10856L, 60);
    decompTo("bank select impl", 0x12572L, 60);
    decompTo("vesa window wrapper", 0x12ac8L, 80);
    decompTo("VESA mode init", 0x12290L, 500);
    listingTo("wait ticks helper", 0x3264bL, 14);
    dumpMem("zone param table 0x80be4", 0x80be4L, 96);
    dumpMem("endgame dispatch 0x8abf8", 0x8abf8L, 176);
    dumpMem("sensor fixed pts y 0x8d210", 0x8d210L, 72);
    dumpMem("sensor fixed pts x 0x8d20c", 0x8d20cL, 72);
    w.close();
    println("B2EpisClose DONE");
  }
}
