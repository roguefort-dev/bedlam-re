import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;
import ghidra.app.cmd.label.AddLabelCmd;

// B2 episode-loop pass 3: persist fn names + data labels + plate comments.
// NEVER re-import; -process BEDLAM.EXE -noanalysis only.
public class B2EpisNames extends GhidraScript {
  private int fOK = 0, fSkip = 0, lOK = 0;
  private void nfn(long addr, String name) {
    try {
      Function f = getFunctionAt(toAddr(addr));
      if (f == null) { println("FN-MISS " + String.format("%08x", addr) + " " + name); fSkip++; return; }
      f.setName(name, SourceType.USER_DEFINED);
      try { f.setComment("plate: see docs/RESEARCH-BEDLAM2-CENSUS.md sec 7"); } catch (Exception e) {}
      fOK++;
    } catch (Exception e) { println("FN-EXC " + name + " " + e); fSkip++; }
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
    nfnC(0x1066bL, "PresentFlip", "plate: VESA page-flip present. Toggles display start (4f07) 0<->0x11ef38, bank pair {0,5}, holds ISR lock 0x11ef2c + flip lock 0x8008e during bank ops, WaitVRetrace, copies 0x96-dword cursor block 0x11ef54->0x11ef58. VESA-off path = plain WaitVRetrace. 14 call sites in MissionRun.");
    nfnC(0x136e0L, "PcmMixerService", "plate: ISR-called PCM voice service. Walks channel records stride 0x26 (20 ch x 0x2f8 bank at 0x86a98), spawns/frees sub-voices (MixVoiceAlloc/MixVoiceFree), handles fe/ff events via MixEventFeFf. Gated by 0x11ef50 && 0x11ef24 && 0x11f0e0.");
    nfn(0x1398dL, "MixVoiceAlloc");
    nfn(0x13905L, "MixChannelFind");
    nfn(0x13e28L, "MixEventFeFf");
    nfn(0x145ffL, "MixVoiceFree");
    nfnC(0x12eb0L, "SoundStubInstall", "plate: installs no-sound stub callbacks (0x12ecf reset-audio-ticks, 0x12ee4 elapsed-ms = ticks*10, 0x12eef nop) into ptr table 0x81f98/0x81fa0 + ISR slot 0x1279b4. Called from SoundInit NO SOUND FX branch.");
    nfnC(0x685f0L, "SoundDriverInstall", "plate: real audio driver install: callback vector (0x68740 arm-PIT, 0x686d0 hi-res elapsed-ms via PIT phase, 0x686b0 irq-tail), driver struct 0x1276dc rate 11025, PIT divisor 1193181/rate at 0x1280b8.");
    nfn(0x50033L, "SoundInit");
    nfn(0x505b0L, "RawSoundLoad");
    nfn(0x507aeL, "RawSoundPlay");
    nfnC(0x3264bL, "WaitTicks100Hz", "plate: zero g_ctr_timeout then busy-wait CMP/JG until counter >= EAX. 100.01 Hz time base for screen timeouts (2000/750/500-tick waits).");
    nfn(0x12564L, "SelectDrawBank");
    nfnC(0x12572L, "BankWrite64K", "plate: select VESA window then copy 0x4000 dwords (64KB) through 0xa0000.");
    nfn(0x12ac8L, "VesaSetWindow");
    nfnC(0x12290L, "VesaModeInit", "plate: DPMI alloc + VESA 4f00/4f01 mode 0x101 (64K win @0xa0000, granule shift -> 0x1273fe, LFB ptr -> 0x11f148), 4f02 set, pages bank {0,5} display-start {0,0x200}.");
    nfnC(0x50a87L, "MapRoomSelect", "plate: map room / mission select + save UI. BRF_*.BIN per stage slot 2..8, SAVEICON, MAPROOM1/2.RAW loops; player picks sub gated by completed mask; save records 5 x 61B @0x8b1d4 {mask,slot,linear,..,stats}; 2000-tick timeout loops on g_ctr_timeout.");
    nfnC(0x57651L, "MissionRun", "plate: mission runner (10820B). Main loop LAB_00057947: sensor octile distances, mouse-region hit tests, unit purchase/dispatch, then PresentFlip = vblank-paced. NO INT8 counter gates sim/render. Returns outcome; 0 = completed.");
    nfn(0x5aa39L, "DebriefScreen");
    nfn(0x5498bL, "BriefingScreen");
    nfn(0x5deb3L, "BriefingMode2");
    nfn(0x330e6L, "DistOctile");
    nfn(0x33147L, "RandBelowB");
    nfnC(0x12ecfL, "SndStubResetTicks", "plate: zeroes 0x801a6/0x801aa (stub audio tick base).");
    nfnC(0x12ee4L, "SndStubElapsedMs", "plate: returns 0x801a6 * 10 (100Hz ticks to ms).");
    nfn(0x12eefL, "SndStubNop");
    nfnC(0x607b0L, "SndSetPos80010", "plate: sets 0x80010 from EDX (audio position base).");
    nfnC(0x686b0L, "SndDrvIrqTail", "plate: decrements 0x82200, at 0 calls 0x6831f(0).");
    nfnC(0x686d0L, "SndDrvElapsedMsHiRes", "plate: ms = 0x821fc*1000/rate + (divisor - PIT ch0 count)*1000/1193181, monotonic clamp via 0x1280b4.");
    nfnC(0x68740L, "SndDrvArmPit", "plate: 0x82200++ -> 1 arms: starts 0x67fc9(rate, 0x68780, 0x1280b0) + reprograms PIT 0x43/0x40 divisor 0x82594 = sample clock on IRQ0.");
    nlb(0x801a6L, "g_ctr_snd_a");
    nlb(0x80010L, "g_ctr_snd_b");
    nlb(0x11f158L, "g_ctr_dead1");
    nlb(0x11f0c8L, "g_isr_phase");
    nlb(0x11f0c4L, "g_ctr_timeout");
    nlb(0x11f0b4L, "g_ctr_dead2");
    nlb(0x11f0b0L, "g_ctr_delay5");
    nlb(0x12576cL, "g_campaign_linear");
    nlb(0x125774L, "g_campaign_mask");
    nlb(0x126848L, "g_stage_slot");
    nlb(0x126858L, "g_sub_mission");
    nlb(0x126844L, "g_save_slot");
    nlb(0x126860L, "g_save_mask");
    nlb(0x126864L, "g_save_linear");
    nlb(0x126810L, "g_money");
    nlb(0x8b1d4L, "g_save_records");
    nlb(0x11f024L, "g_zone");
    nlb(0x11f01cL, "g_mission");
    nlb(0x11f144L, "g_mission_end");
    nlb(0x11ef7cL, "g_page_state");
    nlb(0x11f0c0L, "g_page_bank_b");
    nlb(0x11ef38L, "g_display_start_b");
    nlb(0x11ef54L, "g_page_ptr_a");
    nlb(0x11ef58L, "g_page_ptr_b");
    nlb(0x11f148L, "g_lfb_ptr");
    nlb(0x1273feL, "g_vesa_gran_shift");
    nlb(0x801ceL, "g_vesa_mode_req");
    nlb(0x11efc4L, "g_banked_video");
    nlb(0x8008eL, "g_flip_lock");
    nlb(0x11ef50L, "g_snd_drv_active");
    nlb(0x11ef24L, "g_snd_enabled");
    nlb(0x11f0e0L, "g_snd_service_arm");
    nlb(0x8abf8L, "g_snd_handles");
    println("B2EpisNames DONE fns=" + fOK + "/" + fSkip + " labels=" + lOK);
  }
}
