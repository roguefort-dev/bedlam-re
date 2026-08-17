/*-
 * ExwInputNames.java - naming pass for the input/control-map unit.
 * Persists labels from the exw-input-{sinks,readers*} analysis into the
 * BedlamWatcom project. -process BEDLAM.EXW -noanalysis, NEVER re-import.
 * Names: KeySink, MouseSink, AnyKeyWait, AnyKeyWaitAlt, InputReset,
 * InputResetImpl, ScanToChar, NameEntryScreen, MissionShell, MusicVolumeSet
 * (alias only - FUN_0044c630 already named in music tails run), plus data
 * labels g_keystore, g_mouse_flags, g_scroll_flags, g_music_volume,
 * g_drag_active, g_cursor_x, g_cursor_y, g_input_seen(already), and the
 * latch dwords.
 */
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.SourceType;

public class ExwInputNames extends GhidraScript {

	private static final String[] FN_NAMES = {
		"0041be05:KeySink",
		"0041bf35:MouseSink",
		"0041f9d1:AnyKeyWait",
		"0041fa02:ScanToChar",
		"0041f9b5:InputReset",
		"0043a5fc:NameEntryScreen",
		"0044771c:MissionShell",
	};

	private static final String[] DATA_LABELS = {
		"004edc44:g_keystore",
		"004dc6e4:g_mouse_flags",
		"004eddcc:g_scroll_flags",
		"004ddb2c:g_music_volume",
		"004ede60:g_drag_active",
		"004eddc4:g_cursor_x",
		"004eddc8:g_cursor_y",
		"004edc08:g_latch_MSpace",
		"004edc0c:g_latch_F1",
		"004edc10:g_latch_F2",
		"004edc14:g_latch_F3",
		"004edc18:g_latch_1",
		"004edc1c:g_latch_2",
		"004edc20:g_latch_3",
		"004edc24:g_latch_4",
		"004edc28:g_latch_5",
		"004edc2c:g_latch_6",
		"004edc30:g_latch_7",
		"004edc34:g_latch_P",
	};

	@Override
	public void run() throws Exception {
		int ok = 0;
		for (String entry : FN_NAMES) {
			String[] parts = entry.split(":");
			Address a = currentProgram.getAddressFactory().getAddress(parts[0]);
			ghidra.program.model.listing.Function fn =
				currentProgram.getFunctionManager().getFunctionAt(a);
			if (fn != null) {
				fn.setName(parts[1], SourceType.USER_DEFINED);
				ok++;
			}
			else {
				println("no function at " + parts[0]);
			}
		}
		for (String entry : DATA_LABELS) {
			String[] parts = entry.split(":");
			Address a = currentProgram.getAddressFactory().getAddress(parts[0]);
			try {
				currentProgram.getSymbolTable().createLabel(a, parts[1],
					SourceType.USER_DEFINED);
				ok++;
			}
			catch (Exception e) {
				println("label failed at " + parts[0] + ": " + e);
			}
		}
		println("ExwInputNames: " + ok + " names applied");
	}
}
