; The Windows installer (decision 0043), built by Inno Setup in the
; Windows EXE workflow on every merge into main.
;
; It installs for the current user only (no administrator needed), into
; %LOCALAPPDATA%\Programs\<name> unless the user chooses another folder
; (one they can write to), adds a Start menu entry and an entry in
; Settings > Apps for uninstalling, and leaves installed.txt beside the
; program so it knows it may update itself.
;
; The name and file names aren't written here: the workflow reads them
; from crates/app/src/branding.rs and passes them in:
;   iscc /DAppName=... /DAppId=... /DRepo=... /DAppVersion=... /DBuild=...
;        /DSource=<folder with the program> /DOutput=<folder> setup.iss

#ifndef AppName
  #error Pass /DAppName (branding::APP_NAME)
#endif
#ifndef AppId
  #error Pass /DAppId (branding::APP_ID)
#endif
#ifndef Repo
  #error Pass /DRepo (branding::UPDATE_REPO)
#endif
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef Build
  #define Build "0"
#endif
#ifndef Source
  #define Source "..\dist"
#endif
#ifndef Output
  #define Output "..\dist"
#endif

#define AppExe AppId + ".exe"

[Setup]
; Fixed for good: Windows knows a new version replaces the old by it.
AppId={{56B32A85-F967-4190-A41D-F6DB369D78F2}
AppName={#AppName}
AppVersion={#AppVersion}.{#Build}
AppVerName={#AppName} {#AppVersion} (build {#Build})
AppPublisher=Rhyslos
AppPublisherURL=https://github.com/{#Repo}
AppSupportURL=https://github.com/{#Repo}/issues
AppUpdatesURL=https://github.com/{#Repo}/releases
VersionInfoVersion={#AppVersion}.{#Build}
; For this user only: no administrator question, and the program can
; replace itself when it updates.
PrivilegesRequired=lowest
DefaultDirName={autopf}\{#AppName}
; The user may choose the folder. Updates (quiet installs) go to the same
; one: Inno Setup remembers it by AppId.
DisableDirPage=no
UsePreviousAppDir=yes
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableReadyPage=yes
UninstallDisplayIcon={app}\{#AppExe}
OutputDir={#Output}
OutputBaseFilename={#AppId}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Updates close the program if it's still open, and never restart Windows.
CloseApplications=force
RestartApplications=no
LicenseFile={#Source}\LICENSE

[Tasks]
Name: "desktopicon"; Description: "Put a shortcut on the desktop"; Flags: unchecked

[Files]
Source: "{#Source}\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs

[INI]
; installed.txt: the program updates itself only when this is beside it.
Filename: "{app}\installed.txt"; Section: "installed"; Key: "build"; String: "{#Build}"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; After a first install by hand: offer to open it.
Filename: "{app}\{#AppExe}"; Description: "Open {#AppName}"; Flags: nowait postinstall skipifsilent
; After Restart to update (a quiet install with /RELAUNCH=1): open it again.
Filename: "{app}\{#AppExe}"; Flags: nowait; Check: Relaunch

[UninstallDelete]
Type: files; Name: "{app}\installed.txt"

[Code]
function Relaunch: Boolean;
begin
  Result := WizardSilent and (ExpandConstant('{param:RELAUNCH|0}') = '1');
end;

{ Whether this user can write in Folder (made if it isn't there yet).
  Without an administrator, folders such as Program Files are refused, and
  updates must be able to replace the program later. }
function CanWriteIn(Folder: String): Boolean;
var
  Probe: String;
  Made: Boolean;
begin
  Made := not DirExists(Folder);
  Result := ForceDirectories(Folder);
  if Result then
  begin
    Probe := AddBackslash(Folder) + '.write-test';
    Result := SaveStringToFile(Probe, 'test', False);
    DeleteFile(Probe);
  end;
  if Made then
    RemoveDir(Folder);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = wpSelectDir) and not CanWriteIn(WizardDirValue) then
  begin
    MsgBox('{#AppName} can''t be installed in' + #13#10 + WizardDirValue + #13#10#13#10 +
      'Your Windows account can''t write there without an administrator, and ' +
      '{#AppName} must be able to replace itself when it updates. Choose a ' +
      'folder of your own, such as one in your Documents or on another drive.',
      mbError, MB_OK);
    Result := False;
  end;
end;
