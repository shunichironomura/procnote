; Add $INSTDIR\cli to user PATH on install
!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ; Check if already in PATH
  ${WordFind} "$0" "$INSTDIR\cli" "E+1{" $1
  IfErrors 0 _path_already_set
    ; Not found — append
    StrCmp $0 "" 0 +2
      StrCpy $0 "$INSTDIR\cli"
    StrCmp $0 "$INSTDIR\cli" +2 0
      StrCpy $0 "$0;$INSTDIR\cli"
    WriteRegExpandStr HKCU "Environment" "Path" "$0"
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  _path_already_set:
!macroend

; Remove $INSTDIR\cli from user PATH on uninstall
!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ; Remove "$INSTDIR\cli" (handles ";dir", "dir;", and standalone "dir")
  ${WordReplace} "$0" "$INSTDIR\cli" "" "+" $0
  ; Clean up double semicolons
  ${WordReplace} "$0" ";;" ";" "+*" $0
  ; Remove leading semicolon
  StrCpy $1 $0 1
  StrCmp $1 ";" 0 +2
    StrCpy $0 $0 "" 1
  ; Remove trailing semicolon
  StrLen $1 $0
  IntOp $1 $1 - 1
  StrCpy $2 $0 1 $1
  StrCmp $2 ";" 0 +2
    StrCpy $0 $0 $1
  WriteRegExpandStr HKCU "Environment" "Path" "$0"
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend
