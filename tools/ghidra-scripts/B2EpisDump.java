import java.io.PrintWriter;
import java.util.regex.Pattern;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.Memory;

// B2 episode-loop pass 1: INT8 counter reader census (listing-text scan),
// ISR gate writers, VESA call sites, background-service decompiles, table dumps.
// NEVER re-import; -process BEDLAM.EXE -noanalysis only.
public class B2EpisDump extends GhidraScript {
  private DecompInterface di;
  private PrintWriter w;
  private String fname(Instruction ins) {
    Function f = currentProgram.getFunctionManager().getFunctionContaining(ins.getAddress());
    return f == null ? "-" : f.getName() + "@" + f.getEntryPoint();
  }
  private void decompTo(String hdr, long addr, int maxLines) {
    w.println();
    w.println("==== " + hdr + " " + String.format("%08x", addr) + " ====");
    Function fn = getFunctionAt(toAddr(addr));
    if (fn == null) { w.println("// no function at " + String.format("%08x", addr)); return; }
    w.println("// fn " + fn.getName() + " body " + fn.getBody().getNumAddresses() + " bytes");
    DecompileResults r = di.decompileFunction(fn, 90, monitor);
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
  @Override
  public void run() throws Exception {
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    try { w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-epis.txt", "UTF-8"); }
    catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 episode-loop progression + INT8 counter readers (pass 1)");
    long[] ctrs = { 0x801a6L, 0x80010L, 0x11f158L, 0x11f0c8L, 0x11f0c4L, 0x11f0b4L, 0x11f0b0L };
    long[] gates = { 0x11ef50L, 0x11ef24L, 0x11f0e0L, 0x11efc4L, 0x8008eL, 0x11efd4L, 0x11efd8L,
                     0x80034L, 0x11ef54L, 0x11ef58L, 0x11f07cL, 0x11ef7cL, 0x11f0c0L, 0x11ef2cL };
    Pattern[] cp = new Pattern[ctrs.length], gp = new Pattern[gates.length];
    for (int i = 0; i < ctrs.length; i++) cp[i] = Pattern.compile("0x0*" + Long.toHexString(ctrs[i]) + "(?![0-9a-fA-F])");
    for (int i = 0; i < gates.length; i++) gp[i] = Pattern.compile("0x0*" + Long.toHexString(gates[i]) + "(?![0-9a-fA-F])");
    int shown = 0;
    w.println("== listing census: 7 counters + ISR gates ==");
    InstructionIterator iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String t = ins.toString();
      String tag = null;
      for (int i = 0; i < ctrs.length; i++) if (cp[i].matcher(t).find()) { tag = "CTR" + Long.toHexString(ctrs[i]); break; }
      if (tag == null) for (int i = 0; i < gates.length; i++) if (gp[i].matcher(t).find()) { tag = "GATE" + Long.toHexString(gates[i]); break; }
      if (tag != null && shown < 700) {
        w.println(String.format("%s %08x %-10s %s   [%s]", tag, ins.getAddress().getOffset(), ins.getMnemonicString(), t, fname(ins)));
        shown++;
      }
    }
    w.println("census lines " + shown);
    w.println();
    w.println("== VESA function scalars (4f00-4f07) in listing ==");
    shown = 0;
    Pattern vesa = Pattern.compile("0x4f0[0-7](?![0-9a-fA-F])");
    iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String t = ins.toString();
      if (vesa.matcher(t).find() && shown < 250) {
        w.println(String.format("%08x %-10s %s   [%s]", ins.getAddress().getOffset(), ins.getMnemonicString(), t, fname(ins)));
        shown++;
      }
    }
    w.println("vesa lines " + shown);
    w.println();
    w.println("== getReferencesTo census (7 counters) ==");
    for (int i = 0; i < ctrs.length; i++) {
      Address a = toAddr(ctrs[i]);
      w.println("-- refs to " + String.format("%08x", ctrs[i]));
      boolean any = false;
      for (ghidra.program.model.symbol.Reference rf : getReferencesTo(a)) {
        any = true;
        Function f = currentProgram.getFunctionManager().getFunctionContaining(rf.getFromAddress());
        w.println("   " + rf.getFromAddress() + " " + rf.getReferenceType() + "  [" + (f == null ? "-" : f.getName()) + "]");
      }
      if (!any) { w.println("   (none)"); }
    }
    dumpMem("slot full-mask table", 0x81d9aL, 32);
    dumpMem("save record 0 head", 0x8b1d4L, 64);
    dumpMem("per-mission word array 0x126634", 0x126634L, 64);
    dumpMem("gate/descriptor region", 0x11ef50L, 176);
    dumpMem("page ptrs region", 0x11f070L, 96);
    decompTo("ISR background service", 0x136e0L, 400);
    listingTo("isr-bg entry", 0x136e0L, 40);
    decompTo("isr-bg callee spawn", 0x1398dL, 80);
    decompTo("isr-bg callee fe/ff", 0x13e28L, 120);
    decompTo("isr-bg callee free", 0x145ffL, 80);
    decompTo("bank switcher", 0x12bc0L, 80);
    decompTo("mouse poll", 0x1259fL, 120);
    decompTo("mouse post", 0x12a9cL, 80);
    decompTo("campaign reset helper", 0x509d0L, 120);
    decompTo("menu fx loader", 0x505b0L, 80);
    decompTo("menu helper", 0x5e4cfL, 150);
    decompTo("mission runner", 0x57651L, 1500);
    decompTo("debrief", 0x5aa39L, 900);
    w.close();
    println("B2EpisDump DONE");
  }
}
