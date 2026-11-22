; BACKCORD's Windows installer. scripts/package-windows.py compiles it with
; Inno Setup 7 under Wine, from the same files as the portable ZIP.
;
; It installs for the person running it, without administrator rights, and can
; start BACKCORD in the tray when they sign in. That is the Run key value
; Settings > Advanced > "Start when you sign in" writes (src/autostart.rs), so
; the installer and the setting turn the same thing on and off.

#ifndef AppVersion
  #error Compile through scripts/package-windows.py, which defines AppVersion, VersionNumber and SourceDir
#endif
; The full preview defines Edition and FileSuffix; the Discord release neither.
; Both share the AppId, so installing one replaces the other.
#ifdef Edition
  #define DisplayName "BACKCORD (" + Edition + ")"
#else
  #define DisplayName "BACKCORD"
  #define FileSuffix ""
#endif

[Setup]
; Windows knows an installed BACKCORD by this; never change it.
AppId={{19FF8D53-8D7E-4E66-91BB-E2F5BE307E9A}
AppName=BACKCORD
AppVersion={#AppVersion}
AppVerName={#DisplayName} {#AppVersion}
AppPublisher=BACKCORD
AppPublisherURL=https://github.com/Spike848/BACKCORD
AppSupportURL=https://github.com/Spike848/BACKCORD/issues
AppUpdatesURL=https://github.com/Spike848/BACKCORD/releases
VersionInfoVersion={#VersionNumber}
; Per user: %LOCALAPPDATA%\Programs\BACKCORD, with no UAC prompt.
PrivilegesRequired=lowest
DefaultDirName={autopf}\BACKCORD
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=BACKCORD-{#AppVersion}{#FileSuffix}-windows-x64-setup
SetupIconFile=backcord.ico
UninstallDisplayIcon={app}\backcord.exe
UninstallDisplayName={#DisplayName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; BACKCORD keeps running in the tray with its window closed, so an update or
; uninstall closes it rather than asking.
CloseApplications=force
RestartApplications=no
#ifdef LicenseFile
LicenseFile={#LicenseFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartOnSignIn=Start BACKCORD automatically when Windows starts
english.StartOnSignInGroup=Startup:

[Tasks]
Name: "startonsignin"; Description: "{cm:StartOnSignIn}"; GroupDescription: "{cm:StartOnSignInGroup}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; README.txt explains the ZIP; the installer needs no instructions.
Source: "{#SourceDir}\*"; Excludes: "README.txt"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\BACKCORD"; Filename: "{app}\backcord.exe"
Name: "{autodesktop}\BACKCORD"; Filename: "{app}\backcord.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "BACKCORD"; ValueData: """{app}\backcord.exe"" --background"; Tasks: startonsignin
; Chosen here, it starts even if Task Manager had turned it off before.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "BACKCORD"; Flags: deletevalue dontcreatekey; Tasks: startonsignin
; BACKCORD's own links open here; claiming Discord's stays the person's choice.
Root: HKCU; Subkey: "Software\Classes\backcord"; ValueType: string; ValueName: ""; ValueData: "URL:BACKCORD Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\backcord"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\backcord\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\backcord.exe"",0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\backcord\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\backcord.exe"" --open-link ""%1"""; Flags: uninsdeletekey
; Settings keeps the app discord:// links opened before claiming them here;
; uninstalling gives them back (below) and drops what was kept.
Root: HKCU; Subkey: "Software\BACKCORD"; Flags: uninsdeletekeyifempty dontcreatekey
Root: HKCU; Subkey: "Software\BACKCORD\DiscordProtocol"; Flags: uninsdeletekey dontcreatekey
; Each start registers BACKCORD's name and icon for its notifications
; (src/notify.rs); uninstalling forgets them.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\io.github.spike848.BACKCORD"; Flags: uninsdeletekey dontcreatekey
; Uninstalling stops starting at sign-in, however it was turned on.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "BACKCORD"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "BACKCORD"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\backcord.exe"; Description: "{cm:LaunchProgram,BACKCORD}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Before the files go: discord:// links open in the app that had them again,
; if Settings claimed them for this BACKCORD.
Filename: "{app}\backcord.exe"; Parameters: "--release-discord-links"; Flags: runhidden waituntilterminated; RunOnceId: "ReleaseDiscordLinks"
