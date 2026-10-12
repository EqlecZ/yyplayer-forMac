YYPlayer's Windows setup executable is compiled with the Nullsoft Scriptable
Install System (NSIS). The NSIS compiler is a build-time tool; it is not included
as a separate runtime in this package.

NSIS copyright and applicable license terms are maintained by its authors.
The upstream license appendix describes the licenses used by NSIS and its
compression modules:

https://nsis.sourceforge.io/Docs/AppendixI.html

The installer source used for this package is packaging/windows/yyplayer.nsi
in the YYPlayer source commit listed in BUILD-INFO.txt.

The local double-click launcher uses the NSIS 3.12 portable ZIP recorded in
packaging/windows/nsis.json. It verifies the archive SHA-256 and extracts a fresh
copy per run under target/windows-packaging-runs/. The cached archive is under
target/windows-packaging-tools/; no system-wide NSIS installation is performed.
An explicit -NsisCompiler option and the command-line/CI entry point may use a
developer-provided compiler instead.

Pinned upstream archive: https://sourceforge.net/projects/nsis/files/NSIS%203/3.12/
