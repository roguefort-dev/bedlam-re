/*-
 * DumpAscii.java - print raw bytes (and printable runs) at given addresses
 * in the already-imported BEDLAM.EXW program.
 * Usage: DumpAscii <outFile> <addr> <len> [<addr> <len> ...]
 * Ghidra discipline: run with -process BEDLAM.EXW -noanalysis (never re-import).
 */
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;

public class DumpAscii extends GhidraScript {

	@Override
	public void run() throws Exception {
		String[] args = getScriptArgs();
		if (args.length < 3 || (args.length - 1) % 2 != 0) {
			throw new IllegalArgumentException("usage: DumpAscii <outFile> <addr> <len> ...");
		}
		Memory mem = currentProgram.getMemory();
		try (PrintWriter out =
			new PrintWriter(Files.newBufferedWriter(Paths.get(args[0]),
				StandardCharsets.UTF_8))) {
			for (int i = 1; i < args.length; i += 2) {
				Address a = currentProgram.getAddressFactory().getDefaultAddressSpace()
					.getAddress(args[i]);
				int len = Integer.decode(args[i + 1]);
				byte[] buf = new byte[len];
				mem.getBytes(a, buf);
				StringBuilder hex = new StringBuilder();
				StringBuilder ascii = new StringBuilder();
				for (int j = 0; j < len; j++) {
					int b = buf[j] & 0xff;
					hex.append(String.format("%02x ", b));
					ascii.append((b >= 0x20 && b < 0x7f) ? (char) b : '.');
				}
				out.println("----- " + args[i] + " len=" + len + " -----");
				out.println(hex);
				out.println(ascii);
			}
			out.println("===== DONE =====");
		}
		println("DumpAscii: done.");
	}
}
