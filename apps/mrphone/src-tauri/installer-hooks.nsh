; Umstieg aus der Electron-Version (mrphone bis 2.x, früher „SIP Phone“): Vor dem Installieren wird sie
; still entfernt. Ihre Daten in %APPDATA%\SIP Phone bleiben – diese Version benutzt sie direkt weiter
; (src-tauri/src/paths.rs). Der Uninstall-Schlüssel ist aus ihrer App-ID de.rau.sipphone abgeleitet und
; fest; sie läuft per Benutzer. Läuft die alte App noch, beendet ihr Deinstaller sie selbst.
!define ELECTRON_UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\bbbb2b48-a2e1-5e29-8a75-0784954697a7"

!macro NSIS_HOOK_PREINSTALL
  Push $R0
  Push $R1
  Push $R2
  Push $R3
  ReadRegStr $R0 HKCU "${ELECTRON_UNINSTALL_KEY}" "UninstallString"
  ; Form: "C:\…\Programs\SIP Phone\Uninstall SIP Phone.exe" /currentuser -> Pfad zwischen den Anführungszeichen
  StrCpy $R1 $R0 1
  ${If} $R1 == '"'
    StrCpy $R2 1
    electron_scan:
      StrCpy $R1 $R0 1 $R2
      ${If} $R1 != '"'
      ${AndIf} $R1 != ""
        IntOp $R2 $R2 + 1
        Goto electron_scan
      ${EndIf}
    IntOp $R2 $R2 - 1
    StrCpy $R1 $R0 $R2 1
    ${If} ${FileExists} "$R1"
      ${GetParent} "$R1" $R3
      DetailPrint "Entferne die bisherige mrphone-Version (Electron) – Konten, Kontakte und Verlauf bleiben …"
      ; _?= : im Ordner ausführen statt aus einer Kopie, damit ExecWait auf das Ende wartet
      ExecWait '"$R1" /S /currentuser _?=$R3' $R2
      ${If} $R2 = 0
        Delete "$R1"
        RMDir "$R3"
        ; Eintrag sicher weg, sonst hielte diese Version die alte noch für installiert
        DeleteRegKey HKCU "${ELECTRON_UNINSTALL_KEY}"
      ${Else}
        DetailPrint "Die bisherige Version konnte nicht entfernt werden (Code $R2) – bitte später selbst deinstallieren."
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $R3
  Pop $R2
  Pop $R1
  Pop $R0
!macroend
