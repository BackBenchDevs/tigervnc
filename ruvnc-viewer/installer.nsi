!include "MUI2.nsh"

Name "RuVNC Viewer"
OutFile "ruvnc-viewer-setup.exe"
InstallDir "$PROGRAMFILES64\RuVNC Viewer"
InstallDirRegKey HKLM "Software\RuVNC Viewer" "InstallDir"
RequestExecutionLevel admin

!define MUI_ICON "dist\ruvnc-viewer.ico"
!define MUI_UNICON "dist\ruvnc-viewer.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
    SetOutPath "$INSTDIR"

    File "dist\ruvnc-viewer.exe"
    File "dist\*.dll"
    File "dist\ruvnc-viewer.ico"

    WriteUninstaller "$INSTDIR\uninstall.exe"

    CreateDirectory "$SMPROGRAMS\RuVNC Viewer"
    CreateShortCut "$SMPROGRAMS\RuVNC Viewer\RuVNC Viewer.lnk" "$INSTDIR\ruvnc-viewer.exe" "" "$INSTDIR\ruvnc-viewer.ico"
    CreateShortCut "$SMPROGRAMS\RuVNC Viewer\Uninstall.lnk" "$INSTDIR\uninstall.exe"
    CreateShortCut "$DESKTOP\RuVNC Viewer.lnk" "$INSTDIR\ruvnc-viewer.exe" "" "$INSTDIR\ruvnc-viewer.ico"

    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "DisplayName" "RuVNC Viewer"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "DisplayIcon" "$INSTDIR\ruvnc-viewer.ico"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "Publisher" "BackBenchDevs"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer" "NoRepair" 1
    WriteRegStr HKLM "Software\RuVNC Viewer" "InstallDir" "$INSTDIR"
SectionEnd

Section "Uninstall"
    Delete "$INSTDIR\ruvnc-viewer.exe"
    Delete "$INSTDIR\*.dll"
    Delete "$INSTDIR\ruvnc-viewer.ico"
    Delete "$INSTDIR\uninstall.exe"

    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\RuVNC Viewer\RuVNC Viewer.lnk"
    Delete "$SMPROGRAMS\RuVNC Viewer\Uninstall.lnk"
    RMDir "$SMPROGRAMS\RuVNC Viewer"
    Delete "$DESKTOP\RuVNC Viewer.lnk"

    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuVNC Viewer"
    DeleteRegKey HKLM "Software\RuVNC Viewer"
SectionEnd
