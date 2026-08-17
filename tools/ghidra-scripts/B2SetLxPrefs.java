import java.util.prefs.Preferences;
import ghidra.app.script.GhidraScript;

// Sets default loader options for yetmorecode ghidra-lx-loader.
// The loader reads these prefs when building its default option set,
// so they apply to any subsequent headless import in any project.
// Used for the B2 BEDLAM.EXE LE import: fixup labels + page labels ON,
// map LE loader/fixup/data header sections as overlay blocks ON.
public class B2SetLxPrefs extends GhidraScript {
	@Override
	public void run() throws Exception {
		var p = Preferences.userRoot().node("yetmorecode.ghidra.lx.Options");
		p.putBoolean("Create labels at fixup positions", true);
		p.putBoolean("Create labels at page beginnings", true);
		p.putBoolean("Map LE Loader, Fixup & Data Sections", true);
		p.putBoolean("Log fixup statistics", true);
		p.flush();
		println("B2SetLxPrefs: lx loader defaults set (fixup+page labels, map extra, fixup stats)");
	}
}
