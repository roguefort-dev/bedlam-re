import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.scalar.Scalar;

// P4.2/W1 probe 1: census + frame-tail candidates (VESA/IO) + RNG/word
// constants + strings + entry decompile. Runs via -process BEDLAM.EXD
// -noanalysis ONLY (never re-import).
public class EXDProbe extends GhidraScript {
  private static final int DT = 120;
  private PrintWriter w;
  private String fname(Instruction ins) {
    Function f = currentProgram.getFunctionManager().getFunctionContaining(ins.getAddress());
    return f == null ? "-" : f.getName() + "@" + f.getEntryPoint();
  }
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe.txt";
    try { w = new PrintWriter(out, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# EXD probe 1: census + frame-tail candidates + constants + strings (P4.2/W1)");

    w.println();
    w.println("== MEMORY BLOCKS ==");
    for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
      w.println(String.format("%s %08x-%08x %s", b.getName(), b.getStart().getOffset(),
        b.getEnd().getOffset(), b.isWrite() ? "RW" : "R"));
    }

    w.println();
    w.println("== FUNCTION CENSUS + TOP 30 BY SIZE ==");
    List<Function> fns = new ArrayList<Function>();
    FunctionIterator fit = currentProgram.getFunctionManager().getFunctions(true);
    while (fit.hasNext()) fns.add(fit.next());
    w.println("function_total " + fns.size());
    List<Function> bySize = new ArrayList<Function>(fns);
    bySize.sort(Comparator.comparingLong((Function f) -> f.getBody().getNumAddresses()).reversed());
    for (int i = 0; i < Math.min(30, bySize.size()); i++) {
      Function f = bySize.get(i);
      w.println(String.format("TOP %2d %08x size %6d %s", i, f.getEntryPoint().getOffset(),
        f.getBody().getNumAddresses(), f.getName()));
    }

    w.println();
    w.println("== INT instruction histogram (first 200 sites) ==");
    java.util.TreeMap<String, Integer> ih = new java.util.TreeMap<String, Integer>();
    List<String> intSites = new ArrayList<String>();
    InstructionIterator iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String m = ins.getMnemonicString();
      if (m.equals("INT") || m.equals("INT3")) {
        Object[] ops = ins.getOpObjects(0);
        String vec = (ops != null && ops.length > 0 && ops[0] instanceof Scalar)
          ? String.format("0x%x", ((Scalar) ops[0]).getValue()) : "?";
        Integer c = ih.get(vec); ih.put(vec, c == null ? 1 : c + 1);
        if (intSites.size() < 200) intSites.add(String.format("INT %08x %-12s [%s]",
          ins.getAddress().getOffset(), ins.toString(), fname(ins)));
      }
    }
    for (java.util.Map.Entry<String, Integer> e : ih.entrySet()) w.println("HIST " + e.getKey() + " x" + e.getValue());
    for (String s : intSites) w.println(s);

    w.println();
    w.println("== IO ports of interest (DX imm loads + IN imm) ==");
    long[] ports = {0x20, 0x40, 0x43, 0x60, 0x201, 0x3c0, 0x3c8, 0x3c9, 0x3da};
    iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      String m = ins.getMnemonicString();
      if (m.equals("IN") || m.equals("OUT")) {
        String t = ins.toString();
        if (t.contains("0x3da") || t.contains("0x3c9") || t.contains("0x3c8") || t.contains(",0x60")
            || t.contains("0x43") || t.contains("0x40"))
          w.println(String.format("IO %08x %-16s [%s]", ins.getAddress().getOffset(), t, fname(ins)));
        continue;
      }
      if (!m.equals("MOV") || ins.getNumOperands() != 2) continue;
      String op0 = ins.getDefaultOperandRepresentation(0);
      Object[] ops = ins.getOpObjects(1);
      if (ops == null || ops.length == 0 || !(ops[0] instanceof Scalar)) continue;
      long v = ((Scalar) ops[0]).getValue();
      if (op0.contains("DX")) for (long p : ports) if (v == p)
        w.println(String.format("PORTDX %08x %-16s [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
    }

    w.println();
    w.println("== KEY SCALAR CENSUS (code immediates) ==");
    // RNG seeds/additives, magic tile words, timers, VESA ops, PIT divisor,
    // damage-table anchors, spawn payout, counts.
    long[] keys = {0x1e240, 0x39447, 0x3619, 0x62e9, 0x7d2, 0x7d3, 0x7d4,
      0x197, 0x4f05, 0x4f07, 0x4f02, 0x2e9b, 0x1388, 0x1f40, 0x2710,
      0x2000, 0x96, 0x138};
    java.util.Map<Long, List<String>> hits = new java.util.TreeMap<Long, List<String>>();
    iit = currentProgram.getListing().getInstructions(true);
    while (iit.hasNext()) {
      Instruction ins = iit.next();
      for (int oi = 0; oi < ins.getNumOperands(); oi++) {
        Object[] ops = ins.getOpObjects(oi);
        if (ops == null) continue;
        for (Object o : ops) {
          if (o instanceof Scalar) {
            long v = ((Scalar) o).getValue();
            for (long k : keys) if (v == k) {
              List<String> l = hits.get(k);
              if (l == null) { l = new ArrayList<String>(); hits.put(k, l); }
              if (l.size() < 40) l.add(String.format("%08x %-20s [%s]", ins.getAddress().getOffset(), ins.toString(), fname(ins)));
            }
          }
        }
      }
    }
    for (java.util.Map.Entry<Long, List<String>> e : hits.entrySet()) {
      w.println();
      w.println(String.format("-- scalar 0x%x x%d", e.getKey(), e.getValue().size()));
      for (String s : e.getValue()) w.println(s);
    }

    w.println();
    w.println("== STRINGS (defined data, len>=4, first 700) ==");
    int sc = 0;
    java.util.Iterator<Data> dit = currentProgram.getListing().getDefinedData(true);
    while (dit.hasNext() && sc < 700) {
      Data d = dit.next();
      Object val = d.getValue();
      if (val instanceof String) {
        String s = (String) val;
        if (s.length() >= 4) { w.println(String.format("STR %08x %s", d.getAddress().getOffset(), s)); sc++; }
      }
    }
    w.println("strings_shown " + sc);

    w.println();
    DecompInterface di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    String[] targets = {"ENTRY 0x5fbb0"};
    long[] addrs = {0x5fbb0L};
    int[] maxLines = {260};
    for (int i = 0; i < addrs.length; i++) {
      w.println();
      w.println("==== " + targets[i] + " ====");
      Function fn = getFunctionAt(toAddr(addrs[i]));
      if (fn == null) { w.println("// no function at " + targets[i]); continue; }
      DecompileResults r = di.decompileFunction(fn, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
        String[] lines = r.getDecompiledFunction().getC().split("[\r\n]+");
        for (int j = 0; j < lines.length && j < maxLines[i]; j++) w.println(lines[j]);
        if (lines.length > maxLines[i]) w.println("// TRUNCATED " + (lines.length - maxLines[i]));
      } else w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
    di.dispose();
    w.flush(); w.close();
    println("EXDPROBE done -> " + out);
  }
}
