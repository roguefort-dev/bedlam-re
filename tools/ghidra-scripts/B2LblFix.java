import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;
import ghidra.program.model.symbol.SourceType;

// Remove 2 mislabels from B2Residuals: 0x125e18 is g_tick_installed (sec 6.2,
// fade rides the INT8 ISR - the setup gate is tick-installed, NOT a separate
// fade-enable); 0x801ce is g_vesa_mode_req (sec 7.5). NEVER re-import.
public class B2LblFix extends GhidraScript {
  @Override
  public void run() throws Exception {
    SymbolTable st = currentProgram.getSymbolTable();
    long[] addrs = {0x125e18L, 0x801ceL};
    String[] bad = {"g_b2_fade_enabled", "g_vesa_mode_num"};
    String[] keep = {"g_tick_installed", "g_vesa_mode_req"};
    for (int i = 0; i < 2; i++) {
      Address a = toAddr(addrs[i]);
      for (Symbol s : st.getSymbols(a)) {
        if (s.getName().equals(bad[i])) {
          println("REMOVING " + bad[i] + " @ " + String.format("%08x", addrs[i]));
          st.removeSymbolSpecial(s);
        }
      }
      Symbol prim = st.getPrimarySymbol(a);
      println("primary now: " + (prim == null ? "NONE" : prim.getName()) + " @ " + String.format("%08x", addrs[i]));
      if (prim != null && !prim.getName().equals(keep[i])) {
        for (Symbol s : st.getSymbols(a)) {
          if (s.getName().equals(keep[i])) { s.setPrimary(); println("restored primary " + keep[i]); break; }
        }
      }
    }
    println("B2LblFix DONE");
  }
}
