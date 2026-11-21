; RustMagic for Windows, Inno Setup 6.
;
; Built by the release workflow after `cargo build --release`, with the staged
; tree in ..\..\dist\windows. Run locally with:
;
;   iscc /DMyAppVersion=0.2.0 packaging\windows\rustmagic.iss
;
; Inno Setup is what KeyMagic itself uses for its installer, so the result feels
; the same to someone switching over: a normal setup wizard, an entry in Add or
; Remove Programs, and nothing to unzip.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#define MyAppName "RustMagic"
#define MyAppPublisher "RustMagic contributors"
#define MyAppURL "https://github.com/codewiththiha/rustmagic"
#define MyAppExeName "rustmagic.exe"
#define Stage "..\..\dist\windows"

[Setup]
AppId={{DD548953-E974-51F9-8F62-1AFFC8F90801}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile={#Stage}\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=RustMagic-{#MyAppVersion}-Setup-x64
SetupIconFile=..\..\dist\brand\rustmagic.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; The text service registers its CLSID and its language profile under HKLM —
; Windows keeps the list of input methods there — so the installer needs
; administrator rights once, the way every input method does.
PrivilegesRequired=admin
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "path"; Description: "Add RustMagic to the PATH (the command line front end)"; GroupDescription: "Other:"

[Files]
Source: "{#Stage}\rustmagic.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\rustmagic-cli.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\km_ffi.dll"; DestDir: "{app}"; Flags: ignoreversion
; regserver runs `regsvr32 /s` after copying and `regsvr32 /u /s` on uninstall,
; which is what makes RustMagic a keyboard in Windows Settings.
Source: "{#Stage}\RustMagicTSF.dll"; DestDir: "{app}"; Flags: ignoreversion regserver
Source: "{#Stage}\rustmagic.h"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\keyboards\*.km2"; DestDir: "{app}\keyboards"; Flags: ignoreversion
Source: "{#Stage}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\OFL-NotoSansMyanmar.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKA; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: path; Check: NeedsAddPath('{app}')

[UninstallDelete]
Type: filesandordirs; Name: "{app}\keyboards"

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  Existing: string;
begin
  if not RegQueryStringValue(HKA, 'Environment', 'Path', Existing) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(Existing) + ';') = 0;
end;

procedure RemoveFromPath();
var
  Existing, Cleaned, App: string;
begin
  App := ExpandConstant('{app}');
  if not RegQueryStringValue(HKA, 'Environment', 'Path', Existing) then
    exit;
  // StringChangeEx, not StringReplace: the latter is Delphi's, and this script
  // engine does not have it. This is the same edit in place.
  Cleaned := Existing;
  StringChangeEx(Cleaned, ';' + App, '', True);
  StringChangeEx(Cleaned, App + ';', '', True);
  if Cleaned <> Existing then
    RegWriteExpandStringValue(HKA, 'Environment', 'Path', Cleaned);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath();
end;
