Bedlam engine (bedlam-shell) -- the 1996 game's reimplementation.

This install carries the ENGINE ONLY: no game data, no art, no
music ships with it. You supply your own original Bedlam install
(your legally obtained copy of the original 1996 release).

Two ways to point the engine at your original install:

  1. Copy your original install's BEDLAM folder into a folder
     named game-data\BEDLAM directly inside this install folder
     (the engine's documented default: bedlam-shell resolves
     game-data\BEDLAM relative to its working directory, and the
     Start Menu shortcut starts it in this install folder).

  2. Or pass the folder on the command line as the first
     argument:
     bedlam-shell.exe "C:\wherever\you\put\BEDLAM"

Music (CDDA): the engine plays user-supplied original tracks
(BEDLAM02..08.WAV or TRACK02..08.WAV rips of the original CD).
Put the WAV rips in %LOCALAPPDATA%\bedlam\music (or pass
--music-dir), and an optional local cache is generated on first
run. Music is never redistributed with the engine; a miss just
means silent music.

Run bedlam-shell.exe --help for every option (window modes, vsync,
volume mixers, save slots, scaling, presentation modes).

This installer is UNSIGNED (no code-signing key, by design). See
the project repository for source and provenance.
