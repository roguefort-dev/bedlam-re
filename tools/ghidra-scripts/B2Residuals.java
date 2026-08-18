import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.SourceType;
import ghidra.app.cmd.label.AddLabelCmd;

// B2 census sec-7 residuals pass: (a) campaign table tail dump (order[8] alias
// + mission[27]), (b) PresentFlip/VesaModeInit listings at the 4f02/4f07 sites,
// (c) fade-chain naming. NEVER re-import; -process BEDLAM.EXE -noanalysis only.
public class B2Residuals extends GhidraScript {
  private DecompInterface di;
  private PrintWriter w;
  private int fOK = 0, fSkip = 0, lOK = 0;
  private void decompTo(String hdr, long addr, int maxLines) {
    w.println();
    w.println("==== " + hdr + " " + String.format("%08x", addr) + " ====");
    Function fn = getFunctionAt(toAddr(addr));
    if (fn == null) { w.println("// no function at " + String.format("%08x", addr)); return; }
    w.println("// fn " + fn.getName() + " body " + fn.getBody().getNumAddresses() + " bytes");
    DecompileResults r = di.decompileFunction(fn, 120, monitor);
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
  private void nfnC(long addr, String name, String comment) {
    try {
      Function f = getFunctionAt(toAddr(addr));
      if (f == null) { println("FN-MISS " + String.format("%08x", addr) + " " + name); fSkip++; return; }
      f.setName(name, SourceType.USER_DEFINED);
      f.setComment(comment);
      fOK++;
    } catch (Exception e) { println("FN-EXC " + name + " " + e); fSkip++; }
  }
  private void nlb(long addr, String name) {
    try {
      Address a = toAddr(addr);
      AddLabelCmd c = new AddLabelCmd(a, name, true, SourceType.USER_DEFINED);
      if (!c.applyTo(currentProgram)) { println("LB-FAIL " + name); return; }
      lOK++;
    } catch (Exception e) { println("LB-EXC " + name + " " + e); }
  }
  @Override
  public void run() throws Exception {
    di = new DecompInterface();
    di.setOptions(new DecompileOptions());
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    try { w = new PrintWriter("/home/kato/Documents/bedlam-re/ghidra-project/b2-residuals.txt", "UTF-8"); }
    catch (Exception e) { println("OPEN FAIL " + e); return; }
    w.println("# B2 census sec-7 residuals: campaign tables tail, VESA 4f02/4f07 listings, fade chain");
    listingTo("PresentFlip full (4f07 sites)", 0x1066bL, 150);
    listingTo("VesaModeInit 4f02+4f07 region", 0x12418L, 70);
    dumpMem("campaign block fullmask+order+zone+mission", 0x81d9aL, 336);
    dumpMem("mission table tail + past-end", 0x81e46L, 160);
    decompTo("fade dac-read helper", 0x10802L, 40);
    decompTo("fade cancel", 0x1081aL, 40);
    decompTo("fade step", 0x126c8L, 60);
    decompTo("fade setup", 0x3046cL, 90);
    w.close();
    // names (fade chain + residual labels)
    nfnC(0x126c8L, "B2FadeStep", "plate: ISR-serviced palette fade stepper. 768 channels, 8.8 fixed acc += step pairs at 0x9f05c, DAC record bytes = acc>>8 (first byte forced 0), then B2DacUpload, g_b2_fade_ticks_left--. Called from Int8TickHandler when countdown 0x11ef88 != 0 = every 100.01Hz tick while fading (vs EXW 50Hz). See census sec 8.");
    nfnC(0x3046cL, "B2FadeSetup", "plate: start fade to target palette EAX over EDX ticks: instant path via B2FadeCancel when g_b2_fade_enabled==0, else acc = cur<<8, step = +/-((delta*0x100+1)/ticks), countdown = ticks. 8.8 fixed like EXW FadeStep 00425901.");
    nfnC(0x1081aL, "B2FadeCancel", "plate: cancel fade (countdown=0) + immediate DAC upload.");
    nfnC(0x1082cL, "B2DacUpload", "plate: out 0x3c8 start-index, out 0x3c9 rgb triplets from {start,count,rgb[768]} record at 0x9f058 (ESI).");
    nfnC(0x3439bL, "B2FadeWait", "plate: spin while g_b2_fade_ticks_left > EAX (only when g_b2_fade_enabled).");
    nfnC(0x10802L, "B2DacRead", "plate: read current DAC (in 0x3c7/in 0x3c9) into the accumulator base - fades interpolate from live hardware palette.");
    nlb(0x11ef88L, "g_b2_fade_ticks_left");
    nlb(0x125e18L, "g_b2_fade_enabled");
    nlb(0x11f05cL, "g_b2_fade_state_ptr");
    nlb(0x11f058L, "g_b2_dac_record_ptr");
    nlb(0x801ceL, "g_vesa_mode_num");
    nlb(0x126814L, "g_maproom_menu_pick");
    println("B2Residuals DONE fns=" + fOK + "/" + fSkip + " labels=" + lOK);
  }
}
