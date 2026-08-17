import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;

// B2 import verification gates (RESEARCH-BEDLAM2-CENSUS.md sec 4 step 2):
//  i. two blocks 0x10000 X / 0x80000 W spanning to 0x1304ee (zero fill tail)
// ii. entry at 0x66a60 (eip object relative + obj1 base)
//iii. fixup labels present (loader option) + applied relocations recorded
// iv. decompile at entry sane (watcom cspec)
//  v. LE header overlay blocks present (map extra option)
public class B2SmokeVerify extends GhidraScript {
	@Override
	public void run() throws Exception {
		println("PROGRAM " + currentProgram.getName());
		println("LANG " + currentProgram.getLanguageID() + " CSPEC " + currentProgram.getCompilerSpec().getCompilerSpecID());
		println("IMAGE_BASE " + currentProgram.getImageBase());
		for (MemoryBlock b : currentProgram.getMemory().getBlocks()) {
			println(String.format("BLOCK %-28s %08x-%08x size=%d R=%b W=%b X=%b init=%b",
				b.getName(), b.getStart().getOffset(), b.getEnd().getOffset(), b.getSize(),
				b.isRead(), b.isWrite(), b.isExecute(), b.isInitialized()));
		}
		var epIt = currentProgram.getSymbolTable().getExternalEntryPointIterator();
		while (epIt.hasNext()) {
			println("ENTRY " + epIt.next());
		}
		println("RELOC_COUNT " + currentProgram.getRelocationTable().getSize());
		int fixLabels = 0;
		int nofixLabels = 0;
		int pageLabels = 0;
		int imgLabels = 0;
		int symCount = 0;
		SymbolIterator sit = currentProgram.getSymbolTable().getAllSymbols(true);
		while (sit.hasNext()) {
			Symbol s = sit.next();
			symCount++;
			String n = s.getName();
			if (n.startsWith("fix_") || n.startsWith("fix-")) fixLabels++;
			else if (n.startsWith("nofix")) nofixLabels++;
			if (n.startsWith("page")) pageLabels++;
			if (n.startsWith("IMG_")) imgLabels++;
		}
		println("SYMBOL_COUNT " + symCount);
		println("FIXUP_LABELS " + fixLabels);
		println("NOFIX_LABELS " + nofixLabels);
		println("PAGE_LABELS " + pageLabels);
		println("IMG_LABELS " + imgLabels);
		Address entry = toAddr(Long.parseLong(args.length > 0 ? args[0] : "0x66a60", 16));
		Function f = getFunctionAt(entry);
		println("ENTRY_FUNC " + (f == null ? "NONE" : f.getName() + " body " + f.getBody()));
		if (f != null) {
			DecompInterface di = new DecompInterface();
			di.openProgram(currentProgram);
			DecompileResults res = di.decompileFunction(f, 90, monitor);
			if (res.decompileCompleted()) {
				println("=== DECOMP ENTRY START ===");
				for (String line : res.getDecompiledFunction().getC().split("[\r\n]+")) {
					println(line);
				}
				println("=== DECOMP ENTRY END ===");
			}
			else {
				println("DECOMP FAILED " + res.getErrorMessage());
			}
			di.dispose();
		}
		println("=== LISTING AT ENTRY (30 instrs) ===");
		InstructionIterator iit = currentProgram.getListing().getInstructions(entry, true);
		for (int i = 0; i < 30 && iit.hasNext(); i++) {
			Instruction ins = iit.next();
			println(String.format("%08x  %s", ins.getAddress().getOffset(), ins.toString()));
		}
		println("B2SMOKEVERIFY DONE");
	}
}
