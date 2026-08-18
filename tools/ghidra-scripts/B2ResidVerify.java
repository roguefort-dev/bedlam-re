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
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

// B2 residuals close-out verify pass. READ-ONLY: checks the name persistence
// of the B2Residuals + B2LblFix passes, dumps the zone-letter strings at
// 0x8412c, decompiles MapRoomSelect (campaign formula) and the function
// containing the 4f02 site (BX construction / LFB bit), and xrefs the
// campaign table bounds. NEVER re-import; -process BEDLAM.EXE -noanalysis
// only; no save performed.
public class B2ResidVerify extends GhidraScript {
  private PrintWriter w;
  private DecompInterface di;
  private int pass = 0, fail = 0;

  private void checkFn(long addr, String name) {
    Function f = getFunctionAt(toAddr(addr));
    String got = "NONE";
    if (f != null) got = f.getName();
    boolean ok = name.equals(got);
    if (ok) pass++; else fail++;
    w.println("FN " + String.format("%08x", addr) + " want " + name + " got " + got + (ok ? " OK" : " FAIL"));
  }
  private void checkLabel(long addr, String name, boolean wantPrimary) {
    SymbolTable st = currentProgram.getSymbolTable();
    StringBuilder sb = new StringBuilder();
    boolean found = false, prim = false;
    for (Symbol s : st.getSymbols(toAddr(addr))) {
      if (sb.length() > 0) sb.append(", ");
      sb.append(s.getName());
      if (s.getName().equals(name)) { found = true; prim = s.isPrimary(); }
    }
    boolean ok = found && (!wantPrimary || prim);
    if (ok) pass++; else fail++;
    w.println("LB " + String.format("%08x", addr) + " want " + name + (wantPrimary ? " primary" : "") + " got [" + sb + "] " + (ok ? "OK" : "FAIL"));
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
  private void decompTo(String hdr, long addr, int maxLines) {
    w.println();
    w.println("==== " + hdr + " " + String.format("%08x", addr) + " ====");
    Function fn = getFunctionAt(toAddr(addr));
    if (fn == null) { w.println("// no function at " + String.format("%08x", addr)); return; }
    w.println("// fn " + fn.getName() + " body " + fn.getBody().getNumAddresses() + " bytes");
    DecompileResults r = di.decompileFunction(fn, 180, monitor);
    if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
      List<String> ls = new ArrayList<>();
      r.getDecompiledFunction().getC().lines().forEach(x -> ls.add(x));
      int i = 0;
      for (String ln : ls) {
        if (i >= maxLines) { w.println("// TRUNCATED " + (ls.size() - maxLines) + " lines"); break; }
        w.println(ln); i++;
      }
    } else {
      w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
  }
  private void xrefsTo(String hdr, long addr) {
    w.println();
    w.println("---- xrefs " + hdr + " " + String.format("%08x", addr) + " ----");
    int n = 0;
    for (Reference r : getReferencesTo(toAddr(addr))) {
      Function f = getFunctionContaining(r.getFromAddress());
      String in = "?";
      if (f != null) in = f.getName();
      w.println(r.getFromAddress().getOffset() + " " + r.getReferenceType() + " in " + in);
      n++;
    }
    if (n == 0) w.println("// none");
  }
  @Override
  public void run() throws Exception {
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    try { w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-resid-verify.txt", "UTF-8"); }
    catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 residuals close-out verify: persistence + letters + MapRoomSelect + VesaModeInit + table xrefs");
    w.println();
    w.println("== (a) name persistence (B2Residuals + B2LblFix) ==");
    checkFn(0x126c8L, "B2FadeStep");
    checkFn(0x3046cL, "B2FadeSetup");
    checkFn(0x1081aL, "B2FadeCancel");
    checkFn(0x1082cL, "B2DacUpload");
    checkFn(0x3439bL, "B2FadeWait");
    checkFn(0x10802L, "B2DacRead");
    checkLabel(0x11ef88L, "g_b2_fade_ticks_left", false);
    checkLabel(0x11f05cL, "g_b2_fade_state_ptr", false);
    checkLabel(0x11f058L, "g_b2_dac_record_ptr", false);
    checkLabel(0x126814L, "g_maproom_menu_pick", false);
    checkLabel(0x125e18L, "g_tick_installed", true);
    checkLabel(0x801ceL, "g_vesa_mode_req", true);
    checkLabel(0x11ef7cL, "g_page_state", false);
    checkLabel(0x11ef38L, "g_display_start_b", false);
    w.println();
    w.println("== (b) video/page symbol census ==");
    SymbolTable st = currentProgram.getSymbolTable();
    for (Symbol s : st.getAllSymbols(true)) {
      String nm = s.getName();
      if (nm.startsWith("g_page") || nm.startsWith("g_vesa") || nm.startsWith("g_banked") || nm.startsWith("g_lfb") || nm.startsWith("g_display")) {
        w.println(String.format("%08x", s.getAddress().getOffset()) + " " + nm + (s.isPrimary() ? " primary" : ""));
      }
    }
    dumpMem("zone letters region", 0x8412cL, 0x60);
    decompTo("MapRoomSelect campaign formula", 0x50a87L, 420);
    Function vm = getFunctionContaining(toAddr(0x12439L));
    if (vm != null) {
      w.println();
      w.println("// 4f02 site 0x12439 is inside " + vm.getName() + " @ " + vm.getEntryPoint());
      decompTo("VesaModeInit full (4f02 BX construction)", vm.getEntryPoint().getOffset(), 200);
    } else {
      w.println("// no containing function at 0x12439");
    }
    xrefsTo("zone table base", 0x81ddaL);
    xrefsTo("mission table base", 0x81e46L);
    xrefsTo("lone dword after mission", 0x81eb2L);
    xrefsTo("letters ptr table", 0x81eb6L);
    xrefsTo("fullmask base", 0x81d9aL);
    xrefsTo("order base", 0x81dbaL);
    w.close();
    println("B2ResidVerify DONE pass=" + pass + " fail=" + fail);
  }
}
