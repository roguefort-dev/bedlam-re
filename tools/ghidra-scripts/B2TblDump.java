import java.io.PrintWriter;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;

// B2 pass 3: dump zone/mission stride tables + RNG init region straight from program memory.
public class B2TblDump extends GhidraScript {
  private void dump(Memory m, PrintWriter w, long a, int n, String label) {
    w.println("== " + label + " @ " + String.format("%08x", a) + " (" + n + " bytes) ==");
    byte[] b = new byte[n];
    try { m.getBytes(toAddr(a), b); } catch (Exception e) { w.println("READ FAIL " + e); return; }
    for (int i = 0; i < n; i += 16) {
      StringBuilder hex = new StringBuilder();
      StringBuilder asc = new StringBuilder();
      for (int j = 0; j < 16 && i + j < n; j++) {
        int v = b[i+j] & 0xff;
        hex.append(String.format("%02x ", v));
        asc.append(v >= 0x20 && v < 0x7f ? (char)v : ".");
      }
      w.println(String.format("%08x  %-48s %s", a + i, hex, asc));
    }
  }
  private void dumpI(PrintWriter w, long a, int n, String label) {
    w.println("== " + label + " dwords @ " + String.format("%08x", a) + " ==");
    Memory m = currentProgram.getMemory();
    StringBuilder sb = new StringBuilder();
    for (int i = 0; i < n; i++) {
      try {
        int v = m.getInt(toAddr(a + 4L*i));
        sb.append(v).append(i == n-1 ? "" : ",");
      } catch (Exception e) { sb.append("ERR"); }
    }
    w.println(sb);
  }
  @Override
  public void run() throws Exception {
    Memory m = currentProgram.getMemory();
    PrintWriter w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-tbl-dump.txt", "UTF-8");
    dumpI(w, 0x81dbaL, 8, "order table");
    dump(m, w, 0x81ddaL, 0x70, "zone letters");
    dumpI(w, 0x81e46L, 27, "mission table");
    dump(m, w, 0x80be4L, 0x90, "zone param tables 0x80be4..");
    dump(m, w, 0x11ef10L, 0x30, "rng init region");
    dump(m, w, 0x801a6L, 0x10, "int8 counters low");
    w.flush(); w.close();
    println("B2TBLDUMP done");
  }
}
