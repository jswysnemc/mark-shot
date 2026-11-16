#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装测试】【卸载编号】通过隔离安装登记验证重装编号变化及目录约束，不执行其他安装的程序。 #>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Uninstaller tests require Windows.' }
. (Join-Path $PSScriptRoot 'windows-setup-uninstaller.ps1')
$testId = [guid]::NewGuid().ToString('N')
$directory = Join-Path ([IO.Path]::GetTempPath()) "mark-shot-uninstaller-$testId/install with spaces"
$subKey = "Software\Microsoft\Windows\CurrentVersion\Uninstall\MarkShotPackageTest-$testId"
$key = $null
New-Item -ItemType Directory -Path $directory -Force | Out-Null
try {
    # 1. 【Windows安装测试】【重装编号】旧编号仍存在时采用注册表的新编号，后续重装重新读取当前登记
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($subKey)
    $key.SetValue('InstallLocation', "$directory\")
    [IO.File]::WriteAllBytes((Join-Path $directory 'unins000.exe'), [byte[]]@(0))
    foreach ($name in @('unins001.exe', 'unins002.exe')) {
        $file = Join-Path $directory $name
        [IO.File]::WriteAllBytes($file, [byte[]]@(0))
        $key.SetValue('UninstallString', "`"$file`"")
        if ((Get-WindowsSetupUninstaller -InstallDirectory $directory) -ne $file) { throw 'Uninstaller number was not refreshed from the registry.' }
    }
    # 2. 【Windows安装测试】【缺失及归属】缺少文件或指向安装目录之外的登记均必须失败
    foreach ($file in @((Join-Path $directory 'unins003.exe'), (Join-Path (Split-Path $directory -Parent) 'unins000.exe'))) {
        $key.SetValue('UninstallString', "`"$file`"")
        $rejected = $false
        try { Get-WindowsSetupUninstaller -InstallDirectory $directory | Out-Null }
        catch { $rejected = $true }
        if (-not $rejected) { throw 'Invalid registered uninstaller was accepted.' }
    }
    Write-Host '【Windows安装测试】【卸载编号】重装编号、缺失文件与目录归属检查通过'
}
finally {
    # 3. 【Windows安装测试】【环境恢复】删除本次独占的安装登记与占位文件
    if ($null -ne $key) { $key.Dispose() }
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($subKey, $false)
    Remove-Item -LiteralPath (Split-Path $directory -Parent) -Recurse -Force
}
