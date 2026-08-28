; bedlam-shell.nsi -- the committed Windows installer definition
; (the p7-windows-installer deliverable; PLAN section 6 P7 "Windows
; installer"; docs/P7-PORTS.md section 2 row windows-installer;
; proving gate p7-windows-installer; decision D227).
;
; WHAT THIS IS: makensis's complete input -- the definition of the
; NSIS installer built from THIS repository's staged engine binary.
; CI builds it on every push (the ci.yml job `windows-installer`
; on windows-latest): the release engine is built, the binary is
; staged next to this script, and makensis compiles THIS file into
; the unsigned installer bedlam-shell-setup.exe (OutFile below).
;
; WHAT THIS IS NOT: a distribution of the game. The installer
; carries the ENGINE BINARY ONLY -- never the corpus, never art,
; never music (git is engine-only; D21). The user supplies their
; OWN original install at run time: bedlam-shell takes INSTALL_DIR
; as its first positional argument, and its documented default is
; resolved relative to the process working directory. The Start
; Menu shortcut below starts the engine with $INSTDIR as its
; working directory (NSIS stores $OUTDIR as the shortcut's working
; directory property, and SetOutPath runs first), so the engine's
; default lookup root sits directly inside the install folder.
; The README this installer drops next to the binary spells both
; ways out in plain words.
;
; UNSIGNED by design: no key material ever marks this installer
; (the signing-keys exclusion, D221); a store page is the
; publication-stores exclusion.
;
; THE GRAMMAR IS CLOSED: the proving gate parses this file with a
; closed NSIS command set -- unknown commands, unquoted string
; arguments, wildcards or path separators in File sources,
; switches on Delete/RMDir, C-style comments and line
; continuations are all parse errors (the file that ships is the
; file that is graded). makensis is invoked with its working
; directory equal to this script's directory (the CI job's
; working-directory: packaging), so every relative path below
; resolves to packaging\ under either candidate rule (the script's
; own directory, or the process working directory).

Name "Bedlam engine"
OutFile "bedlam-shell-setup.exe"
Unicode true
InstallDir "$PROGRAMFILES64\Bedlam"
RequestExecutionLevel admin
CRCCheck force
Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Bedlam engine"
  SetOutPath "$INSTDIR"
  ; exactly two files ride along -- the engine binary + its README
  ; (the checker pins the closed set; nothing else may enter)
  File "bedlam-shell.exe"
  File "windows-installer-README.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  ; the Add/Remove-Programs registration (the standard Windows
  ; uninstall entry)
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BedlamEngine" "DisplayName" "Bedlam engine"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BedlamEngine" "UninstallString" "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Bedlam"
  ; $OUTDIR is $INSTDIR here, so the shortcut's working directory
  ; is the install folder itself
  CreateShortcut "$SMPROGRAMS\Bedlam\Bedlam engine.lnk" "$INSTDIR\bedlam-shell.exe"
SectionEnd

Section "un.Uninstall"
  Delete "$SMPROGRAMS\Bedlam\Bedlam engine.lnk"
  RMDir "$SMPROGRAMS\Bedlam"
  Delete "$INSTDIR\bedlam-shell.exe"
  Delete "$INSTDIR\windows-installer-README.txt"
  Delete "$INSTDIR\uninstall.exe"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BedlamEngine"
  RMDir "$INSTDIR"
SectionEnd
