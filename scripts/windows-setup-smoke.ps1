#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装】【安装回归】验证静默安装、所有包内文件、独立启动和启动项所属关系。 #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Installer,
    [Parameter(Mandatory)][string]$PackageDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-smoke.ps1')
. (Join-Path $PSScriptRoot 'windows-registry-entry.ps1')
. (Join-Path $PSScriptRoot 'windows-setup-uninstaller.ps1')
$root = Split-Path $PSScriptRoot -Parent
$evidence = Join-Path $root 'target/windows-evidence/setup'
$testId = [guid]::NewGuid().ToString('N')
$temporary = Join-Path $root "target/setup-smoke-$testId"
$installed = Join-Path $temporary 'install with spaces'
$runSubKey = 'Software\Microsoft\Windows\CurrentVersion\Run'
$runKey = "HKCU:\$runSubKey"
$original = Get-WindowsRegistryEntry -SubKey $runSubKey -Name MarkShot
New-Item -ItemType Directory -Force $evidence, $temporary | Out-Null

function Invoke-SetupProcess {
    <# .SYNOPSIS 【Windows安装】【进程验证】按 File 和 Arguments 执行安装器或卸载器，返回空值，超时或非零退出报错。 #>
    param([string]$File, [string[]]$Arguments)
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $File
    $start.UseShellExecute = $false
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    try {
        if (-not $process.WaitForExit(120000)) {
            $process.Kill($true)
            throw "Installer process timed out: $File"
        }
        if ($process.ExitCode -ne 0) { throw "Installer process failed ($($process.ExitCode)): $File" }
    }
    finally { $process.Dispose() }
}

try {
    # 1. 【Windows安装】【静默验证】静默安装不启动托盘，不依赖开发工具所在的路径
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/NOICONS', "/MarkShotPackageTest=$testId", "/DIR=$installed", "/LOG=$evidence/install.log")
    Invoke-SetupProcess -File $Installer -Arguments $arguments
    foreach ($line in (Get-Content (Join-Path $PackageDirectory 'SHA256SUMS'))) {
        if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw "Invalid package checksum: $line" }
        $expected = $Matches[1]
        $file = Join-Path $installed $Matches[2]
        if ((Get-FileHash $file -Algorithm SHA256).Hash -ne $expected) { throw "Installed file mismatch: $file" }
    }
    Invoke-WindowsHelp -PackageDirectory $installed -OutputPath (Join-Path $evidence 'installed-help.txt')
    $uninstaller = Get-WindowsSetupUninstaller -InstallDirectory $installed
    # 2. 【Windows安装】【所属清理】卸载必须删除本目录启动值，保留设置与其他安装位置的启动项
    if (-not (Test-Path $runKey)) { New-Item -Path $runKey | Out-Null }
    New-ItemProperty -Path $runKey -Name MarkShot -Value "`"$installed\mark-shot.exe`" --background" -PropertyType String -Force | Out-Null
    Invoke-SetupProcess -File $uninstaller -Arguments @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=$evidence/uninstall.log")
    if (Test-Path (Join-Path $installed 'mark-shot.exe')) { throw 'Uninstall left the application executable behind.' }
    if ($null -ne (Get-WindowsRegistryEntry -SubKey $runSubKey -Name MarkShot)) { throw 'Uninstall left its login startup entry behind.' }
    Invoke-SetupProcess -File $Installer -Arguments $arguments
    $uninstaller = Get-WindowsSetupUninstaller -InstallDirectory $installed
    $other = '"C:\Other Location\mark-shot.exe" --background'
    New-ItemProperty -Path $runKey -Name MarkShot -Value $other -PropertyType String -Force | Out-Null
    Invoke-SetupProcess -File $uninstaller -Arguments @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=$evidence/uninstall-other.log")
    $remaining = Get-WindowsRegistryEntry -SubKey $runSubKey -Name MarkShot
    if ($null -eq $remaining -or $remaining.Value -ne $other) { throw 'Uninstall removed another installation startup entry.' }
}
finally {
    # 3. 【Windows安装】【测试恢复】恢复测试前用户注册，删除本次隔离安装目录
    if ($null -eq $original) {
        if ($null -ne (Get-WindowsRegistryEntry -SubKey $runSubKey -Name MarkShot)) {
            Remove-ItemProperty -Path $runKey -Name MarkShot
        }
    } else {
        New-ItemProperty -Path $runKey -Name MarkShot -Value $original.Value -PropertyType $original.Kind -Force | Out-Null
    }
    if (Test-Path $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
