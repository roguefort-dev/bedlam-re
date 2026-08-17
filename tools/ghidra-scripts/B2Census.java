import java.io.PrintWriter;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.StringDataInstance;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;

// B2 post-import census: seed the function DB + strings pass for asset
// filenames. Writes ghidra-project/b2-functions.txt and b2-strings.txt.
public class B2Census extends GhidraScript {
	@Override
	public void run() throws Exception {
		String outDir = "/home/kato/Documents/bedlam-re/ghidra-project";
		int fnCount = 0;
		try (PrintWriter fw = new PrintWriter(outDir + "/b2-functions.txt")) {
			fw.println("# B2 BEDLAM.EXE function census (addr size name)");
			FunctionIterator fit = currentProgram.getFunctionManager().getFunctions(true);
			while (fit.hasNext()) {
				Function f = fit.next();
				fnCount++;
				fw.println(String.format("%08x %6d %s", f.getEntryPoint().getOffset(), f.getBody().getNumAddresses(), f.getName()));
			}
		}
		int strCount = 0;
		int fileish = 0;
		try (PrintWriter sw = new PrintWriter(outDir + "/b2-strings.txt")) {
			sw.println("# B2 BEDLAM.EXE defined strings census (addr len value)");
			DataIterator dit = currentProgram.getListing().getDefinedData(true);
			while (dit.hasNext()) {
				Data d = dit.next();
				if (d.hasStringValue()) {
					String s = StringDataInstance.getStringDataInstance(d).getStringValue();
					if (s == null || s.length() < 4) continue;
					strCount++;
					sw.println(String.format("%08x %4d %s", d.getAddress().getOffset(), s.length(), s));
					if (s.matches(".*\\.(PAL|RAW|MRS|MRW|PCX|BIN|NFO|MS|INI|DLG|ENG|BDL|LOG|EXE|COM|386|DAT|LZ|ANM|VOC|WAV|MID)")) {
						fileish++;
					}
				}
			}
		}
		println("B2CENSUS functions=" + fnCount + " strings=" + strCount + " fileish=" + fileish);
	}
}
