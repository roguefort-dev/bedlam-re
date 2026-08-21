import java.io.PrintWriter;
import java.util.Map;
import java.util.TreeMap;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

// P4.2/W1 probe 6: FUN_000448e7 call-target fingerprint + callers.
// -process BEDLAM.EXD -noanalysis ONLY.
public class EXDProbe6 extends GhidraScript {
  @Override
  public void run() throws Exception {
    String out = "/home/kato/Documents/bedlam-re/ghidra-project/exd-probe6.txt";
    try (PrintWriter w = new PrintWriter(out, "UTF-8")) {
      w.println("# EXD probe 6: monolith call fingerprint");
      // callers of 0x448e7
      w.println("== CALLERS of 000448e7 ==");
      Function f = getFunctionAt(toAddr(0x448e7L));
      if (f != null) {
        for (Reference r : getReferencesTo(f.getEntryPoint())) {
          if (r.getReferenceType().isCall()) {
            Function cf = currentProgram.getFunctionManager().getFunctionContaining(r.getFromAddress());
            w.println(String.format("CALL from %08x [%s]", r.getFromAddress().getOffset(),
              cf == null ? "-" : cf.getName() + "@" + cf.getEntryPoint()));
          }
        }
      }
      // call-target histogram inside 0x448e7..end
      w.println();
      w.println("== CALL TARGETS in FUN_000448e7 (target xN) ==");
      Map<Long, Integer> hist = new TreeMap<Long, Integer>();
      Address a = toAddr(0x448e7L);
      Address end = f.getBody().getMaxAddress();
      while (a.compareTo(end) <= 0) {
        Instruction ins = getInstructionAt(a);
        if (ins == null) { a = a.add(1); continue; }
        if (ins.getMnemonicString().startsWith("CALL")) {
          for (Reference r : ins.getReferencesFrom()) {
            if (r.getReferenceType().isCall()) {
              long t = r.getToAddress().getOffset();
              Integer c = hist.get(t); hist.put(t, c == null ? 1 : c + 1);
            }
          }
        }
        a = ins.getMaxAddress().add(1);
      }
      for (Map.Entry<Long, Integer> e : hist.entrySet()) {
        if (e.getValue() >= 2) {
          Function tf = getFunctionAt(toAddr(e.getKey()));
          w.println(String.format("TGT %08x x%d [%s]", e.getKey(), e.getValue(),
            tf == null ? "-" : tf.getName()));
        }
      }
      w.println("distinct_targets " + hist.size());
    }
    println("EXDPROBE6 done");
  }
}
