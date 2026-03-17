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
DisableDirPage=yes

[Files]
Source: "..\target\bundled\Sauce.vst3\*"; DestDir: "{app}\Sauce.vst3"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Uninstall Sauce"; Filename: "{uninstallexe}"
