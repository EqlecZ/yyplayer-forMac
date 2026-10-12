Unicode true
!include "MUI2.nsh"
!include "x64.nsh"

!ifndef APP_VERSION
  !error "APP_VERSION must be supplied by scripts/Package-Windows.ps1"
!endif
!ifndef APP_COMMIT
  !define APP_COMMIT "local"
!endif
!define APP_STAGE "$%YYPLAYER_PACKAGE_STAGE%"
!define OUTPUT_DIR "$%YYPLAYER_PACKAGE_OUTPUT%"
!define APP_ID "{A858C6E1-6B63-4A1B-90EC-275C31F61D38}"
!ifndef APP_ICON
  !error "APP_ICON must point to assets/icons/yyplayer/liquid-orbit-disc/yyplayer.ico"
!endif
!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"

Name "YYPlayer ${APP_VERSION}"
OutFile "${OUTPUT_DIR}\YYPlayer-${APP_VERSION}-${APP_COMMIT}-windows-x64-setup.exe"
InstallDir "$PROGRAMFILES64\YYPlayer"
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "YYPlayer"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "YYPlayer Windows x64 Setup"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-only"

!define MUI_ABORTWARNING
!define MUI_LICENSEPAGE_CHECKBOX
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${APP_STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\yyplayer.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch YYPlayer"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "This package requires 64-bit Windows."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Section "YYPlayer" MainSection
  SectionIn RO
  SetShellVarContext all
  SetOutPath "$INSTDIR"
  File /r "${APP_STAGE}\*"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\YYPlayer"
  CreateShortcut "$SMPROGRAMS\YYPlayer\YYPlayer.lnk" "$INSTDIR\yyplayer.exe" "" "$INSTDIR\yyplayer.exe" 0
  CreateShortcut "$SMPROGRAMS\YYPlayer\Uninstall YYPlayer.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\yyplayer.exe" 0
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayName" "YYPlayer"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayIcon" "$INSTDIR\yyplayer.exe,0"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext all
  Delete "$SMPROGRAMS\YYPlayer\YYPlayer.lnk"
  Delete "$SMPROGRAMS\YYPlayer\Uninstall YYPlayer.lnk"
  RMDir "$SMPROGRAMS\YYPlayer"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
  RMDir /r "$INSTDIR"
SectionEnd
