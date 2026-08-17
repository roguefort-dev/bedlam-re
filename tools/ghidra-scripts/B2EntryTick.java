import java.io.PrintWriter;
import java.util.HashMap;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

// B2 pass 1: entry-chain decompiles + tick-source census + RNG refs + full decompile sweep.
// NEVER re-import; runs via -process BEDLAM.EXE -noanalysis only.
public class B2EntryTick extends GhidraScript {
  private static final int DT = 90;
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
    DecompileResults r = di.decompileFunction(fn, DT, monitor);
    if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
      String[] lines = r.getDecompiledFunction().getC().split("[\r\n]+");
      for (int i = 0; i < lines.length && i < maxLines; i++) w.println(lines[i]);
      if (lines.length > maxLines) w.println("// TRUNCATED " + (lines.length - maxLines) + " lines");
    } else {
      w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
  }
  @Override
  public void run() throws Exception {
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    String out1 = "/home/kato/Documents/bedlam-re/ghidra-project/b2-entry-tick.txt";
    String out2 = "/home/kato/Documents/bedlam-re/ghidra-project/b2-decomp-all.txt";
    HashMap<String, Integer> intHist = new HashMap<String, Integer>();
    int ioCount = 0, intCount = 0;
    try { w = new PrintWriter(out1, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 entry-chain + tick-source census (pass 1)");
    w.println();
    w.println("== IN/OUT/STI/CLI/HLT/IRET instructions ==");
    InstructionIterator iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String m = ins.getMnemonicString();
      boolean io = m.equals("IN") || m.equals("OUT") || m.equals("INSB") || m.equals("INSW")
        || m.equals("OUTSB") || m.equals("OUTSW") || m.equals("STI") || m.equals("CLI")
        || m.equals("HLT") || m.equals("IRET") || m.equals("IRETD");
      if (io) {
        if (ioCount < 400) w.println(String.format("IO %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
        ioCount++;
      }
    }
    w.println("io_total " + ioCount);
    w.println();
    w.println("== INT instructions (histogram then first 400) ==");
    iit = currentProgram.getListing().getInstructions(true);
    java.util.List<String> intLines = new java.util.ArrayList<String>();
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String m = ins.getMnemonicString();
      if (m.equals("INT") || m.equals("INT3") || m.equals("INTO") || m.equals("BOUND") || m.equals("INT1")) {
        intCount++;
        String vec = "?";
        Object[] ops = ins.getOpObjects(0);
        if (ops != null && ops.length > 0 && ops[0] instanceof Scalar) vec = String.format("0x%x", ((Scalar)ops[0]).getValue());
        Integer c = intHist.get(m + " " + vec);
        intHist.put(m + " " + vec, c == null ? 1 : c + 1);
        if (intLines.size() < 400) intLines.add(String.format("INT %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
      }
    }
    for (java.util.Map.Entry<String, Integer> e : intHist.entrySet()) w.println("HIST " + e.getKey() + " count " + e.getValue());
    for (String line : intLines) w.println(line);
    w.println("int_total " + intCount);
    w.println();
    w.println("== port immediates loaded into DX + PIT command bytes into AL ==");
    long[] ports = {0x20,0x21,0x40,0x41,0x42,0x43,0x60,0x61,0x70,0xa0,0xa1,0x201,0x3da,0x3c0};
    long[] cmds = {0x34,0x36,0xb6};
    iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      if (!ins.getMnemonicString().equals("MOV") || ins.getNumOperands() != 2) continue;
      String op0 = ins.getDefaultOperandRepresentation(0);
      Object[] ops = ins.getOpObjects(1);
      if (ops == null || ops.length == 0 || !(ops[0] instanceof Scalar)) continue;
      long v = ((Scalar)ops[0]).getValue();
      if (op0.contains("DX")) {
        for (long p : ports) if (v == p) w.println(String.format("PORTDX %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
      } else if (op0.contains("AL")) {
        for (long c : cmds) if (v == c) w.println(String.format("PITCMD %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
      }
    }
    w.println();
    w.println("== PIT divisor / rate constants (full instruction) ==");
    long[] pits = {11932L, 19886L, 65536L, 1193182L, 0x1234ddL};
    iit = currentProgram.getListing().getInstructions(true);
    int ph = 0;
    while (iit.hasNext() && ph < 200) {
      Instruction ins = iit.next();
      for (int oi = 0; oi < ins.getNumOperands(); oi++) {
        Object[] ops = ins.getOpObjects(oi);
        if (ops == null) continue;
        for (Object o : ops) {
          if (o instanceof Scalar) {
            long v = ((Scalar)o).getValue();
            for (long p : pits) if (v == p) {
              w.println(String.format("PIT %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
              ph++; break;
            }
          }
        }
      }
    }
    w.println("pit_hits_first200 " + ph);
    w.println();
    w.println("== RNG global + reseed-site references (listing text scan) ==");
    String[] tokens = {"1280d4", "1280d8", "11ef1c"};
    iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String t = ins.toString().toLowerCase();
      for (String tok : tokens) if (t.contains(tok)) {
        w.println(String.format("RNG %08x %s   [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
        break;
      }
    }
    decompTo("ENTRY", 0x66a60L, 400);
    decompTo("CRT_INIT", 0x6b1bcL, 400);
    decompTo("GAME_INIT", 0x2f731L, 700);
    decompTo("CRT_NEXT", 0x6d96eL, 300);
    decompTo("CRT_DEEP", 0x71736L, 300);
    decompTo("RESEED_SITE", 0x5eaf9L, 700);
    w.flush();
    w.close();
    PrintWriter s;
    try { s = new PrintWriter(out2, "UTF-8"); } catch (Exception e) { println("OPEN2 FAIL " + e); di.dispose(); return; }
    FunctionIterator fit = currentProgram.getFunctionManager().getFunctions(true);
    int n = 0, fail = 0;
    while (fit.hasNext()) {
      Function fn = fit.next();
      n++;
      s.println();
      s.println("==== FN " + String.format("%08x", fn.getEntryPoint().getOffset()) + " " + fn.getName() + " size " + fn.getBody().getNumAddresses() + " ====");
      DecompileResults r = di.decompileFunction(fn, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) s.print(r.getDecompiledFunction().getC());
      else { s.println("// DECOMP FAILED: " + r.getErrorMessage()); fail++; }
      if (n % 40 == 0) { s.flush(); println("SWEEP " + n + " fns, " + fail + " failures"); }
    }
    s.flush();
    s.close();
    println("B2ENTRYTICK done: " + n + " fns, " + fail + " failures -> " + out1 + " + " + out2);
    di.dispose();
  }
}
