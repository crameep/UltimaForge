; UltimaForge NSIS Installer Hooks
;
; This script is included by the Tauri-generated NSIS installer.
; It adds a pre-uninstall step that offers to remove the installed game files.
;
; CUSTOMIZE: Update SERVER_NAME below to match brand.json > product.serverName

!define SERVER_NAME "UnchainedFileServer"

; ============================================================================
; customUninstall
; Runs during the uninstall process (before the launcher files are removed).
; Reads the game_path.txt sidecar written by the launcher at install time and
; offers to delete the game directory.
; ============================================================================

!macro customUninstall
  !include "FileFunc.nsh"

  ; Path to the sidecar file the launcher writes when the game is installed
  StrCpy $0 "$APPDATA\UltimaForge\${SERVER_NAME}\game_path.txt"

  ; Check if the sidecar exists
  IfFileExists "$0" game_files_found no_game_files

  game_files_found:
    ; Read the install path from the sidecar (one line, no JSON needed)
    FileOpen $1 "$0" r
    IfErrors no_game_files
    FileRead $1 $2
    FileClose $1

    ; Trim trailing newline/CR from the path
    ${TrimNewLines} "$2" $2

    ; Guard: reject empty path
    StrCmp "$2" "" no_game_files

    ; Guard: reject paths <= 3 chars (e.g. "C:\" is 3 chars — a bare drive root)
    StrLen $3 "$2"
    IntCmp $3 3 no_game_files no_game_files path_length_ok
    path_length_ok:

    ; Guard: character at index 2 must be "\" — rejects bare "C:" or two-char junk
    StrCpy $4 "$2" 1 2
    StrCmp "$4" "\" has_subdirectory no_game_files
    has_subdirectory:

    ; Guard: reject well-known system paths.
    ; StrCmp is case-insensitive in NSIS, so "c:\windows" matches "C:\Windows" etc.
    StrCpy $5 "$2" 10
    StrCmp "$5" "C:\Windows" no_game_files
    StrCpy $5 "$2" 16
    StrCmp "$5" "C:\Program Files" no_game_files
    StrCpy $5 "$2" 24
    StrCmp "$5" "C:\Program Files (x86)" no_game_files
    StrCpy $5 "$2" 9
    StrCmp "$5" "C:\System" no_game_files

    ; Ask the user
    MessageBox MB_YESNO|MB_ICONQUESTION \
      "Remove game files?$\n$\nThe game was installed to:$\n$2$\n$\nDo you want to permanently delete these files?" \
      IDNO no_game_files

    ; Remove the game directory
    RMDir /r "$2"

    ; Remove the sidecar
    Delete "$0"

  no_game_files:
!macroend
