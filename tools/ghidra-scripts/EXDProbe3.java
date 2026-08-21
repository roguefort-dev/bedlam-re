import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

// P4.2/W1 probe 3: loader twins via mission-string xrefs, spread picker,
// stagger/money/platform scalars, frame-tail instruction window.
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe3 extends GhidraScript {
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
  private void xrefsToData(String name, long addr) {
    w.println();
    w.println("== XREFS to " + name + " @" + String.format("%08x", addr) + " ==");
    for (Reference r : getReferencesTo(toAddr(addr))) {
      Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
      w.println(String.format("XREF %08x %s [%s]", r.getFromAddress().getOffset(),
        r.getReferenceType().toString(), cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
    }
  }
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe3.txt";
    try { w = new PrintWriter(out, "UTF-8"); } catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# EXD probe 3: loader twins + spread picker + scalars + frame tail");
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);

    // A. loader-string xrefs
    xrefsToData(".MRK", 0x85064L);
    xrefsToData(".NME", 0x85087L);
    xrefsToData(".TRT", 0x8508cL);
    xrefsToData(".POS", 0x85094L);
    xrefsToData(".BDG", 0x85099L);
    xrefsToData(".TOT", 0x862a9L);
    xrefsToData(".DAT", 0x862aeL);
    xrefsToData(".CGR", 0x862b3L);
    xrefsToData(".MIN", 0x862bdL);
    xrefsToData(".PAD", 0x862ccL);
    xrefsToData("ZONE", 0x86f97L);
    xrefsToData("MISSION", 0x86f9cL);

    // B. spread picker (claims home)
    decompTo("SPREAD_PICKER", 0x3581bL, 160);

    // C. scalar scans with function context
    long[] keys = {0xfa0, 0x7d0, 0x1b, 0x12c, 0xc7, 0xfa, 0x3e8, 0x276, 0x15e};
    java.util.Map<Long, List<String>> hits = new java.util.TreeMap<Long, List<String>>();
    InstructionIterator iit = currentProgram.getListing().getInstructions(true);
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
              if (l.size() < 60) {
                Function cf = currentProgram.getFunctionManager().getFunctionContaining(ins.getAddress());
                l.add(String.format("%08x %-30s [%s]", ins.getAddress().getOffset(), ins.toString(),
                  cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
              }
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

    // D. frame-tail instruction windows in FUN_0004c80c
    w.println();
    w.println("== INSTRUCTIONS 0x4d1d0..0x4d240 (counter/flip order) ==");
    for (long a = 0x4d1d0L; a < 0x4d240L;) {
      Instruction ins = getInstructionAt(toAddr(a));
      if (ins == null) { a++; continue; }
      w.println(String.format("INS %08x %s", a, ins.toString()));
      a += ins.getLength();
    }
    w.println();
    w.println("== INSTRUCTIONS 0x4cb00..0x4cb40 (first flip site) ==");
    for (long a = 0x4cb00L; a < 0x4cb40L;) {
      Instruction ins = getInstructionAt(toAddr(a));
      if (ins == null) { a++; continue; }
      w.println(String.format("INS %08x %s", a, ins.toString()));
      a += ins.getLength();
    }

    // E. MissionShell-analog head
    decompTo("MS_CANDIDATE_4c80c_head", 0x4c80cL, 150);
    decompTo("MS_CANDIDATE_5638d_head", 0x5638dL, 120);

    w.flush(); w.close();
    println("EXDPROBE3 done -> " + out);
    di.dispose();
  }
}
