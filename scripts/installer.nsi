# -*- coding: utf-8 -*-
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
Name "SSMP - Slulisee's Simple Music Player"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\SSMP"
InstallDirRegKey HKCU "Software\Slulisee\SSMP" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 32
BrandingText "made by Slulisee"
Icon "${ROOT}\assets\SSMP.ico"
UninstallIcon "${ROOT}\assets\SSMP.ico"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "SSMP"
VIAddVersionKey "FileDescription" "Slulisee's Simple Music Player Setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Slulisee - GPL-3.0-or-later"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\SSMP.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open SSMP"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${ROOT}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "French"
Var AutomaticUpdate
Var NoIntegration

!macro WAIT_FOR_APP PREFIX
  StrCpy $0 0
  ${Do}
    System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\Slulisee.SSMP.Application") p.r1'
    FindWindow $2 "" "SSMP — Slulisee's Simple Music Player"
    FindWindow $3 "" "SSMP — Slulisee's Simple Media Player"
    StrCpy $4 0
    IfFileExists "$INSTDIR\SSMP.exe" ${PREFIX}check_file ${PREFIX}file_checked
    ${PREFIX}check_file:
      System::Call 'kernel32::CreateFileW(w "$INSTDIR\SSMP.exe", i 0x40000000, i 7, p 0, i 3, i 0, p 0) p.r4'
      ${If} $4 != -1
        System::Call 'kernel32::CloseHandle(p r4)'
        StrCpy $4 0
      ${EndIf}
    ${PREFIX}file_checked:
    ${If} $1 = 0
    ${AndIf} $2 = 0
    ${AndIf} $3 = 0
    ${AndIf} $4 = 0
      ${ExitDo}
    ${EndIf}
    ${If} $1 != 0
      System::Call 'kernel32::CloseHandle(p r1)'
    ${EndIf}
    IntOp $0 $0 + 1
    ${If} $AutomaticUpdate != "1"
    ${AndIf} $0 >= 40
      IfSilent ${PREFIX}busy
      MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Close all SSMP windows before continuing. Your library and media stay in place." IDRETRY ${PREFIX}retry
      ${PREFIX}busy:
      SetErrorLevel 2
      Quit
      ${PREFIX}retry:
      StrCpy $0 0
    ${EndIf}
    Sleep 250
  ${Loop}
!macroend

Function .onInit
  SetShellVarContext current
  System::Call 'kernel32::CreateMutexW(p 0, i 0, w "Local\Slulisee.SSMP.Installer") p.r5 ?e'
  Pop $6
  ${If} $6 = 183
    SetErrorLevel 0
    Quit
  ${EndIf}
  ${GetParameters} $0
  ClearErrors
  ${GetOptions} $0 "/AUTO" $1
  ${IfNot} ${Errors}
    StrCpy $AutomaticUpdate "1"
  ${EndIf}
  ClearErrors
  ${GetOptions} $0 "/NOINTEGRATION" $1
  ${IfNot} ${Errors}
    StrCpy $NoIntegration "1"
  ${EndIf}
  !insertmacro MUI_LANGDLL_DISPLAY
  !insertmacro WAIT_FOR_APP init
FunctionEnd

Section "SSMP" Main
  !insertmacro WAIT_FOR_APP install
  SetOverwrite on
  !include "${PAYLOAD}"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  ${If} $NoIntegration != "1"
    WriteRegStr HKCU "Software\Slulisee\SSMP" "InstallDir" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "DisplayName" "SSMP - Slulisee's Simple Music Player"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "Publisher" "Slulisee"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "DisplayIcon" "$INSTDIR\SSMP.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP" "NoRepair" 1
    CreateDirectory "$SMPROGRAMS\SSMP"
    CreateShortcut "$SMPROGRAMS\SSMP\SSMP.lnk" "$INSTDIR\SSMP.exe"
  ${EndIf}
SectionEnd

Function un.onInit
  SetShellVarContext current
  !insertmacro WAIT_FOR_APP uninstall
FunctionEnd

Section "Uninstall"
  !include "${UNINSTALL_FILES}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  ReadRegStr $0 HKCU "Software\Slulisee\SSMP" "InstallDir"
  ${If} $0 == $INSTDIR
    DeleteRegKey HKCU "Software\Slulisee\SSMP"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\SSMP"
    Delete "$SMPROGRAMS\SSMP\SSMP.lnk"
    RMDir "$SMPROGRAMS\SSMP"
  ${EndIf}
SectionEnd
