/*-
 * ExwNameGamethread.java - tiny naming pass after the game-thread dump.
 * NEVER re-import (AGENTS.md). -process mode only:
 *   analyzeHeadless ghidra-project BedlamWatcom -process BEDLAM.EXW -noanalysis \
 *     -scriptPath tools/ghidra-scripts -postScript ExwNameGamethread.java
 * Pattern: nameFunction only, graceful SKIPs, no output artifacts
 * (results are visible in the project + the console log).
 */
import java.util.ArrayList;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

public class ExwNameGamethread extends GhidraScript {

	/** { address, name, note } - high-confidence names from exw-gamethread.txt. */
	private static final String[][] NAMES = {
		{ "0041c050", "GameMain", "real game shell/loop; sole callee of GameThread trampoline" },
	};

	@Override
	public void run() throws Exception {
		List<String> log = new ArrayList<>();
		for (String[] entry : NAMES) {
			nameFunction(log, entry[0], entry[1], entry[2]);
		}
		for (String line : log) {
			println("ExwNameGamethread: " + line);
		}
		println("ExwNameGamethread: done.");
	}

	private void nameFunction(List<String> log, String addrStr, String newName, String note) {
		Address addr;
		try {
			addr = currentProgram.getAddressFactory().getAddress(addrStr);
		}
		catch (Exception e) {
			log.add(addrStr + "\tSKIP bad address\t" + note);
			return;
		}
		Function fn = currentProgram.getFunctionManager().getFunctionAt(addr);
		if (fn == null) {
			try {
				if (currentProgram.getListing().getInstructionAt(addr) == null) {
					disassemble(addr);
				}
				fn = createFunction(addr, newName);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP create failed: " + e + "\t" + note);
				return;
			}
			if (fn == null) {
				log.add(addr + "\tSKIP createFunction null\t" + note);
				return;
			}
		}
		String oldName = fn.getName();
		if (!oldName.equals(newName)) {
			try {
				fn.setName(newName, SourceType.USER_DEFINED);
				log.add(addr + "\t" + oldName + " -> " + newName + "\t" + note);
			}
			catch (Exception e) {
				log.add(addr + "\tSKIP rename failed: " + e + "\t" + note);
			}
		}
		else {
			log.add(addr + "\tALREADY " + newName + "\t" + note);
		}
	}
}
