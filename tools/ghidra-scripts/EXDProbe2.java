import java.io.PrintWriter;
import java.util.List;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;

// P4.2/W1 probe 2: frame-tail pin (PresentFlip callers), RNG homes,
// mission-loader strings (raw scan), beacon armer + resolver bodies.
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe2 extends GhidraScript {
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
        Instruction ins = currentProgram.getListing().getInstructionAt(r.getFromAddress());
        Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
        w.println(String.format("CALL from %08x  in %s", r.getFromAddress().getOffset(),
          cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
      }
    }
  }
  private void refsTo(String hdr, long addr) {
    w.println();
    w.println("== REFS to " + String.format("%08x", addr) + " ==");
    for (Reference r : getReferencesTo(toAddr(addr))) {
      Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
      w.println(String.format("REF %08x %s  [%s]", r.getFromAddress().getOffset(),
        r.getReferenceType().toString(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
    }
  }
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe2.txt";
    try { w = new PrintWriter(out, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# EXD probe 2: frame tail + RNG homes + loader strings + armer/resolver");
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);

    // A. frame tail: PresentFlip twin callers
    callersOf("PresentFlip", 0x10670L);
    decompTo("PRESENT_FLIP", 0x10670L, 160);

    // B. RNG: seed home refs (0x107470 plant) + stepper caller counts
    refsTo("rng_a_seed?", 0x107470L);
    refsTo("rng_b_seed?", 0x10746cL);
    refsTo("rng_b_seed2?", 0x107474L);
    decompTo("RESEED_SITE_A", 0x596edL, 200);
    decompTo("SEED_SITE_B", 0x2c6e3L, 220);

    // C. mission-loader strings: raw ASCII scan of .object2 for section names
    w.println();
    w.println("== RAW STRING SCAN (.object2) for mission-section + path strings ==");
    Memory mem = currentProgram.getMemory();
    String[] pats = {".TOT", ".DAT", ".CGR", ".BIN", ".MIN", ".NME", ".TRT",
      ".POS", ".BDG", ".PAD", ".MRK", ".PTH", "EDITOR", "ZONE", "MISSION",
      "WEAPONS", "DROPSHIP", "SMOKER", "DEBRIS", "ROBNUMS", "SCANNER",
      "SAVED", "HISCORE", "CONFIG", "OPTIONS", "LANGUAGE"};
    long start = 0x80000L, end = 0x12583eL;
    byte[] buf = new byte[(int) (end - start)];
    mem.getBytes(toAddr(start), buf);
    StringBuilder cur = new StringBuilder();
    long curStart = 0;
    for (int i = 0; i <= buf.length; i++) {
      int b = i < buf.length ? buf[i] & 0xff : 0;
      if (b >= 0x20 && b < 0x7f) { if (cur.length() == 0) curStart = start + i; cur.append((char) b); }
      else {
        if (cur.length() >= 3) {
          String s = cur.toString();
          for (String p : pats) if (s.contains(p)) {
            w.println(String.format("RAW %08x %s", curStart, s));
            break;
          }
        }
        cur.setLength(0);
      }
    }

    // D. beacon armer + impact resolver bodies (T1 anchors)
    decompTo("BEACON_ARMER", 0x3570eL, 220);
    decompTo("IMPACT_RESOLVER_head", 0x2b150L, 120);

    // E. frame-counter hunt: INC/ADD dword [imm] in the top-10 big fns
    w.println();
    w.println("== dword-increment globals (INC dword [imm]) ==");
    InstructionIterator iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String m = ins.getMnemonicString();
      if (!(m.equals("INC") || m.equals("DEC") || m.equals("ADD"))) continue;
      String t = ins.toString();
      if (t.contains("dword ptr [") && !t.contains("+") && !t.contains("ESP") && !t.contains("EBP")) {
        Function cf = currentProgram.getFunctionManager().getFunctionContaining(ins.getAddress());
        w.println(String.format("GINC %08x %-38s [%s]", ins.getAddress().getOffset(), t,
          cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
      }
    }
    w.flush(); w.close();
    println("EXDPROBE2 done -> " + out);
    di.dispose();
  }
}
