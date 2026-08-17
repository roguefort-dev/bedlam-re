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
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolTable;

// B2 pass 2: persist entry-chain/tick names; create+decompile INT8 handler at 0x12734;
// refs hunt for the MISSION string table; NEVER re-import. -process BEDLAM.EXE -noanalysis only.
public class B2EntryNames extends GhidraScript {
  private static final int DT = 90;
  private static final String[][] FN_NAMES = {
    { "0006b1bc", "CrtInitChain", "Watcom CRT init; passes argc/argv (g_argc/g_argv) to GameInit" },
    { "0002f731", "GameInit", "boot shell: OPTIONS.BDL, mouse chk, LANGUAGE load, plants rng seeds 123456/234567, TickInstall" },
    { "0005eaf9", "RngReseedSite", "reseed g_rng_a_seed=123456 site (5664B fn)" },
    { "0001220e", "RngStepA", "steps coupled 16-bit pair g_rng_a_seed (0x11ef1c/0x11ef1e)" },
    { "0001224f", "RngStepB", "steps coupled 16-bit pair g_rng_b_seed (0x11ef18/0x11ef1a)" },
    { "00032546", "TickInstall", "zero clock ctrs; DosGetVector(8); PitProgram(0x2e9b=100.01Hz); DosSetVector(8, LAB_12734)" },
    { "00032507", "TickShutdown", "DosSetVector(8,0); PitProgram(0xffff) restore 18.2Hz" },
    { "000325f9", "PitProgram", "OUT 0x43,0x34; OUT 0x40 lo/hi divisor (arg EAX); saved g_pit_divisor" },
    { "00012708", "PortOut", "out(DX, AL) wrapper" },
    { "0001270a", "DosGetVector", "INT 21h AH=35h get vector; saves old CS to g_int8_old_cs" },
    { "00012727", "DosSetVector", "INT 21h AH=25h set vector" },
    { "00010856", "WaitVRetrace", "poll 0x3da bit3 twice (wait deassert then assert) - vblank pacer, gate g_wait_vsync" },
    { "0001287b", "ClockDivider100Hz", "hundredths(99)->seconds(59)->minutes(59)->hours chain off 100Hz tick" },
    { "00034d90", "SystemShutdown", "full teardown incl TickShutdown, DPMI free, mode restore" },
  };
  private static final String[][] DATA_NAMES = {
    { "0011ef1c", "g_rng_a_seed" },
    { "0011ef18", "g_rng_b_seed" },
    { "0011f0b8", "g_pit_divisor" },
    { "0011f0c8", "g_int8_ctr0" },
    { "0011f128", "g_clock_hundredths" },
    { "0011f140", "g_clock_seconds" },
    { "0011f118", "g_clock_minutes" },
    { "0011f124", "g_clock_hours" },
    { "0011f110", "g_clock_enabled" },
    { "00125e18", "g_tick_installed" },
    { "0011f130", "g_wait_vsync" },
    { "0011f0ec", "g_int8_old_cs" },
    { "001280d4", "g_argc" },
    { "001280d8", "g_argv" },
    { "0011efd4", "g_screen_w_320" },
    { "0011efd8", "g_screen_h_240" },
  };
  @Override
  public void run() throws Exception {
    SymbolTable st = currentProgram.getSymbolTable();
    List<String> log = new ArrayList<>();
    PrintWriter w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-entry-names.txt", "UTF-8");
    DecompInterface di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    // ---- names ----
    for (String[] e : FN_NAMES) {
      Address a = toAddr(Long.parseLong(e[0], 16));
      Function fn = currentProgram.getFunctionManager().getFunctionAt(a);
      if (fn == null) {
        if (currentProgram.getListing().getInstructionAt(a) == null) disassemble(a);
        fn = createFunction(a, e[1]);
      }
      if (fn == null) { log.add(e[0] + " SKIP createFunction null"); continue; }
      String old = fn.getName();
      if (!old.equals(e[1])) {
        try { fn.setName(e[1], SourceType.USER_DEFINED); log.add(e[0] + " " + old + " -> " + e[1] + " ; " + e[2]); }
        catch (Exception ex) { log.add(e[0] + " SKIP rename " + ex); }
      } else log.add(e[0] + " ALREADY " + e[1]);
    }
    for (String[] e : DATA_NAMES) {
      Address a = toAddr(Long.parseLong(e[0], 16));
      try {
        Symbol s = st.getPrimarySymbol(a);
        if (s != null && s.getName().equals(e[1])) log.add(e[0] + " ALREADY " + e[1]);
        else { createLabel(a, e[1], true, SourceType.USER_DEFINED); log.add(e[0] + " LABEL -> " + e[1]); }
      } catch (Exception ex) { log.add(e[0] + " SKIP label " + ex); }
    }
    for (String l : log) { w.println(l); println(l); }
    // ---- INT8 handler create + decompile + listing ----
    w.println();
    w.println("===== INT8 HANDLER 0x12734 =====");
    Address h = toAddr(0x12734L);
    Function hf = currentProgram.getFunctionManager().getFunctionAt(h);
    if (hf == null) {
      if (currentProgram.getListing().getInstructionAt(h) == null) disassemble(h);
      hf = createFunction(h, "Int8TickHandler");
    }
    if (hf != null) {
      w.println("function: " + hf.getName() + " @ " + hf.getEntryPoint() + " size " + hf.getBody().getNumAddresses());
      DecompileResults r = di.decompileFunction(hf, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) w.println(r.getDecompiledFunction().getC());
      else w.println("// DECOMP FAILED: " + r.getErrorMessage());
      w.println("----- listing -----");
      InstructionIterator it = currentProgram.getListing().getInstructions(hf.getBody(), true);
      while (it.hasNext()) {
        Instruction ins = it.next();
        w.println(String.format("%08x  %s", ins.getAddress().getOffset(), ins.toString()));
        for (Reference ref : ins.getReferencesFrom()) {
          if (ref.getReferenceType().isCall()) w.println("    ; calls -> " + ref.getToAddress());
        }
      }
    } else w.println("// createFunction FAILED at 0x12734");
    // ---- extra decompiles ----
    long[] extras = { 0x12290L, 0x1066bL };
    String[] extraN = { "FN_00012290", "FN_0001066b" };
    for (int i = 0; i < extras.length; i++) {
      w.println();
      w.println("===== EXTRA " + extraN[i] + " =====");
      Function f = currentProgram.getFunctionManager().getFunctionAt(toAddr(extras[i]));
      if (f == null) { w.println("// none"); continue; }
      DecompileResults r = di.decompileFunction(f, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) w.println(r.getDecompiledFunction().getC());
      else w.println("// DECOMP FAILED: " + r.getErrorMessage());
    }
    // ---- MISSION string refs hunt ----
    w.println();
    w.println("===== REFS TO MISSION STRINGS =====");
    long[] sadds = { 0x85b3bL, 0x85b44L, 0x85b4eL, 0x85b5eL };
    List<Address> owners = new ArrayList<>();
    for (long sa : sadds) {
      Address sad = toAddr(sa);
      w.println("-- target " + String.format("%08x", sa));
      for (Reference ref : getReferencesTo(sad)) {
        Address fa = ref.getFromAddress();
        Function f = currentProgram.getFunctionManager().getFunctionContaining(fa);
        w.println(String.format("REF %08x type %s in %s", fa.getOffset(), ref.getReferenceType(),
          f == null ? "-" : f.getName() + "@" + f.getEntryPoint()));
        if (f != null && !owners.contains(f.getEntryPoint())) owners.add(f.getEntryPoint());
      }
      // raw listing scan (getReferencesTo misses scaled-index)
      InstructionIterator iit = currentProgram.getListing().getInstructions(true);
      String want = String.format("%08x", sa);
      while (iit.hasNext()) {
        Instruction ins = iit.next();
        if (ins.toString().toLowerCase().contains(want)) {
          Function f = currentProgram.getFunctionManager().getFunctionContaining(ins.getAddress());
          w.println(String.format("SCAN %08x %s in %s", ins.getAddress().getOffset(), ins.toString(),
            f == null ? "-" : f.getName() + "@" + f.getEntryPoint()));
          if (f != null && !owners.contains(f.getEntryPoint())) owners.add(f.getEntryPoint());
        }
      }
    }
    int decomp = 0;
    for (Address oe : owners) {
      if (decomp >= 6) { w.println("// owner cap reached"); break; }
      Function f = currentProgram.getFunctionManager().getFunctionAt(oe);
      if (f == null) continue;
      w.println();
      w.println("===== OWNER " + f.getName() + " @ " + f.getEntryPoint() + " =====");
      DecompileResults r = di.decompileFunction(f, DT, monitor);
      if (r.decompileCompleted() && r.getDecompiledFunction() != null) w.println(r.getDecompiledFunction().getC());
      else w.println("// DECOMP FAILED: " + r.getErrorMessage());
      decomp++;
    }
    if (owners.isEmpty()) w.println("// NO OWNERS FOUND for MISSION strings");
    w.flush(); w.close(); di.dispose();
    println("B2ENTRYNAMES done -> ghidra-project/b2-entry-names.txt ; owners " + owners.size());
  }
}
