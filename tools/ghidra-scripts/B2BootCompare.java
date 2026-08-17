import java.io.PrintWriter;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

// B2 vs EXW boot/init first comparison. EXW anchors (docs/RE-EXW-*):
//  - RNG seeds 123456 (0x1e240) / 234567 (0x39447) at 004ede48/004ede4c
//  - 100Hz tick service, worker thread GameThread -> GameMain shell
// Probes B2 for the same constants in code scalars and initialized data.
public class B2BootCompare extends GhidraScript {
	private DecompInterface di;
	@Override
	public void run() throws Exception {
		di = new DecompInterface();
		di.openProgram(currentProgram);
		String out = "/home/kato/Documents/bedlam-re/ghidra-project/b2-boot-compare.txt";
		try (PrintWriter w = new PrintWriter(out)) {
			w.println("# B2 boot/init vs EXW anchors (first pass)");
			long[] probes = { 123456L, 234567L, 100L, 60L };
			String[] names = { "rng_seed_a_123456", "rng_seed_b_234567", "const_100", "const_60" };
			for (int pi = 0; pi < probes.length; pi++) {
				w.println("== probe " + names[pi] + " (0x" + Long.toHexString(probes[pi]) + ") ==");
				int hits = 0;
				InstructionIterator iit = currentProgram.getListing().getInstructions(true);
				while (iit.hasNext() && hits < 40) {
					Instruction ins = iit.next();
					for (int oi = 0; oi < ins.getNumOperands(); oi++) {
						Object[] ops = ins.getOpObjects(oi);
						for (Object o : ops) {
							if (o instanceof Scalar && ((Scalar)o).getValue() == probes[pi]) {
								w.println(String.format("CODE %08x %s", ins.getAddress().getOffset(), ins.toString()));
								hits++;
								break;
							}
						}
					}
				}
				w.println("code_hits_first40 " + hits);
				int dhits = 0;
				DataIterator dit = currentProgram.getListing().getDefinedData(true);
				while (dit.hasNext() && dhits < 40) {
					Data d = dit.next();
					if (d.hasStringValue()) continue;
					Object v = d.getValue();
					if (v instanceof Number && ((Number)v).longValue() == probes[pi]) {
						w.println(String.format("DATA %08x %s", d.getAddress().getOffset(), d.toString()));
						dhits++;
					}
				}
				w.println("data_hits_first40 " + dhits);
			}
			w.println("== entry chain ==");
			Address entry = toAddr(0x66a60L);
			Function f = getFunctionAt(entry);
			int depth = 0;
			while (f != null && depth < 6) {
				w.println("HOP" + depth + " " + String.format("%08x", f.getEntryPoint().getOffset()) + " " + f.getName());
				DecompileResults res = di.decompileFunction(f, 60, monitor);
				if (res.decompileCompleted()) {
					String c = res.getDecompiledFunction().getC();
					String[] lines = c.split("[\r\n]+");
					int printed = 0;
					for (String line : lines) {
						if (line.contains("FUN_") && printed < 12) {
							w.println("  call? " + line.trim());
							printed++;
						}
					}
				}
				Address next = null;
				DecompileResults r2 = di.decompileFunction(f, 60, monitor);
				if (r2.decompileCompleted()) {
					for (String line : r2.getDecompiledFunction().getC().split("[\r\n]+")) {
						java.util.regex.Matcher m = java.util.regex.Pattern.compile("FUN_([0-9a-f]{8})\\(\\)").matcher(line);
						if (m.find()) { next = toAddr(Long.parseLong(m.group(1), 16)); break; }
					}
				}
				if (next == null) break;
				f = getFunctionAt(next);
				depth++;
			}
		}
		di.dispose();
		println("B2BOOTCOMPARE written " + out);
	}
}
