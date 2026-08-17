/*-
 * ExwRandFix.java - correct music-run naming errors + discover RandB.
 * 1) 00402965: was wrongly named Rand16; it is a rep-stos zero-fill (memset 0).
 * 2) 00402975: the real RNG #1 over 004ede48/004ede4a -> RandA.
 * 3) 004029b6: undiscovered twin RNG #2 over 004ede4c/004ede4e -> create + RandB.
 * Usage: analyzeHeadless <proj> BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *   -scriptPath <dir> -postScript ExwRandFix.java
 */
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.symbol.SourceType;

public class ExwRandFix extends GhidraScript {
	@Override
	public void run() throws Exception {
		FunctionManager fm = currentProgram.getFunctionManager();
		Address a = toAddr(0x402965L);
		Function f = fm.getFunctionAt(a);
		if (f != null) {
			f.setName("MemZero", SourceType.USER_DEFINED);
			f.setComment("rep-stos zero fill (memset 0); was misnamed Rand16 by ExwMusicFollowup");
			println("renamed 00402965 -> MemZero (was " + (f == null ? "?" : "done") + ")");
		}
		f = fm.getFunctionAt(toAddr(0x402975L));
		if (f != null) {
			f.setName("RandA", SourceType.USER_DEFINED);
			f.setComment("RNG #1: word pair 004ede48/004ede4a, rot+add 0x62e9/0x3619 (seed 123456)");
			println("renamed 00402975 -> RandA");
		}
		Address b = toAddr(0x4029b6L);
		f = fm.getFunctionAt(b);
		if (f == null) {
			disassemble(b);
			f = createFunction(b, "RandB");
			println("created function at 004029b6: " + (f != null));
		}
		if (f != null) {
			f.setName("RandB", SourceType.USER_DEFINED);
			f.setComment("RNG #2: word pair 004ede4c/004ede4e, same algo as RandA (seed 234567)");
			println("named 004029b6 -> RandB");
		}
	}
}
