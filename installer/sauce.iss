#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif

[Setup]
AppName=Sauce
AppVersion={#AppVersion}
AppPublisher=Jen
DefaultDirName={commoncf}\VST3
OutputDir=Output
OutputBaseFilename=SauceInstaller
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Show directory page so user can see/change install location
DisableDirPage=no
; Show a welcome page
DisableWelcomePage=no

[Messages]
WelcomeLabel1=Sauce Auto-Tune
WelcomeLabel2=This will install Sauce VST3 plugin to your VST3 folder.%n%nAfter installation, open your DAW and scan for new plugins. Sauce will appear in your plugin list.

[Files]
; Install the VST3 bundle (folder containing the .vst3 dll)
Source: "..\target\bundled\Sauce.vst3\Contents\x86_64-win\Sauce.vst3"; DestDir: "{app}\Sauce.vst3\Contents\x86_64-win"; Flags: ignoreversion

[Icons]
Name: "{group}\Uninstall Sauce"; Filename: "{uninstallexe}"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    MsgBox('Sauce has been installed to:' + #13#10 + #13#10 + ExpandConstant('{app}\Sauce.vst3') + #13#10 + #13#10 + 'Open your DAW and scan for new VST3 plugins. Sauce will appear in your plugin list.', mbInformation, MB_OK);
  end;
end;
