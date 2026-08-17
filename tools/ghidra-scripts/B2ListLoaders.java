import ghidra.Ghidra;
import ghidra.app.script.GhidraScript;
import ghidra.app.util.opinion.Loader;

// Diagnostic: where are user settings, and which Loaders did ClassSearcher find?
public class B2ListLoaders extends GhidraScript {
	@Override
	public void run() throws Exception {
		println("USER_SETTINGS_DIR " + ghidra.framework.Application.getUserSettingsDirectory());
		println("USER_CACHE_DIR " + ghidra.framework.Application.getUserCacheDirectory());
		for (Loader l : ghidra.util.classfinder.ClassSearcher.getInstances(Loader.class)) {
			println("LOADER " + l.getName() + " class " + l.getClass().getName());
		}
	}
}
