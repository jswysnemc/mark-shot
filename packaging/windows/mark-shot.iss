; 【Windows安装】【安装定义】复用已验证的完整便携目录，以当前用户权限安装并清理所属启动项
#ifndef PackageDirectory
  #error PackageDirectory is required
#endif
#ifndef PackageVersion
  #error PackageVersion is required
#endif
#ifndef FileVersion
  #error FileVersion is required
#endif
#ifndef OutputDirectory
  #error OutputDirectory is required
#endif

[Setup]
AppId={code:InstallationId}
AppName=Mark Shot
AppVersion={#PackageVersion}
AppPublisher=Mark Shot
DefaultDirName={localappdata}\Programs\Mark Shot
DefaultGroupName=Mark Shot
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#OutputDirectory}
OutputBaseFilename=mark-shot-native-windows-x86_64-msvc-setup
VersionInfoVersion={#FileVersion}
UninstallDisplayIcon={app}\mark-shot.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
UsePreviousLanguage=no
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#PackageDirectory}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Mark Shot"; Filename: "{app}\mark-shot.exe"; WorkingDir: "{app}"
Name: "{group}\Mark Shot Settings"; Filename: "{app}\mark-shot.exe"; Parameters: "--settings"; WorkingDir: "{app}"

[Run]
Filename: "{app}\mark-shot.exe"; Description: "Start Mark Shot"; Flags: nowait postinstall skipifsilent

[Code]
{ 【Windows安装】【安装身份】按 Param 返回稳定应用标识；测试参数指定隔离标识，避免覆盖现有安装记录 }
function InstallationId(Param: String): String;
var
  TestId: String;
begin
  Result := '{CBB2A8C1-08E0-4C78-AF9D-54CFF8D4C6AD}';
  TestId := ExpandConstant('{param:MarkShotPackageTest|}');
  if TestId <> '' then
    Result := Result + '-PackageTest-' + TestId;
end;

{ 【Windows安装】【卸载清理】按卸载阶段删除指向本安装目录的启动值，无返回值，保留其他程序位置的注册 }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  StartupCommand: String;
  InstalledCommand: String;
begin
  if CurUninstallStep = usUninstall then
  begin
    { 1. 【Windows安装】【路径归属】只删除当前安装程序拥有的 MarkShot 启动值 }
    InstalledCommand := '"' + ExpandConstant('{app}\mark-shot.exe') + '"';
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run',
      'MarkShot', StartupCommand) then
    begin
      if Pos(Lowercase(InstalledCommand), Lowercase(StartupCommand)) = 1 then
        RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'MarkShot');
    end;
  end;
end;
