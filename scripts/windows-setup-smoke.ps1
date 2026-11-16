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
$root = Split-Path $PSScriptRoot -Parent
$evidence = Join-Path $root 'target/windows-evidence/setup'
$testId = [guid]::NewGuid().ToString('N')
$temporary = Join-Path $root "target/setup-smoke-$testId"
$installed = Join-Path $temporary 'install with spaces'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$original = Get-ItemPropertyValue -Path $runKey -Name MarkShot -ErrorAction SilentlyContinue
$originalKind = if ($null -eq $original) { 'String' } else { (Get-Item $runKey).GetValueKind('MarkShot').ToString() }
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
    $uninstaller = Join-Path $installed 'unins000.exe'
    # 2. 【Windows安装】【所属清理】卸载必须删除本目录启动值，保留设置与其他安装位置的启动项
    if (-not (Test-Path $runKey)) { New-Item -Path $runKey | Out-Null }
    New-ItemProperty -Path $runKey -Name MarkShot -Value "`"$installed\mark-shot.exe`" --background" -PropertyType String -Force | Out-Null
    Invoke-SetupProcess -File $uninstaller -Arguments @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=$evidence/uninstall.log")
    if (Test-Path (Join-Path $installed 'mark-shot.exe')) { throw 'Uninstall left the application executable behind.' }
    if ($null -ne (Get-ItemPropertyValue -Path $runKey -Name MarkShot -ErrorAction SilentlyContinue)) { throw 'Uninstall left its login startup entry behind.' }
    Invoke-SetupProcess -File $Installer -Arguments $arguments
    $other = '"C:\Other Location\mark-shot.exe" --background'
    New-ItemProperty -Path $runKey -Name MarkShot -Value $other -PropertyType String -Force | Out-Null
    Invoke-SetupProcess -File $uninstaller -Arguments @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=$evidence/uninstall-other.log")
    if ((Get-ItemPropertyValue -Path $runKey -Name MarkShot) -ne $other) { throw 'Uninstall removed another installation startup entry.' }
}
finally {
    # 3. 【Windows安装】【测试恢复】恢复测试前用户注册，删除本次隔离安装目录
    if ($null -eq $original) {
        Remove-ItemProperty -Path $runKey -Name MarkShot -ErrorAction SilentlyContinue
    } else {
        New-ItemProperty -Path $runKey -Name MarkShot -Value $original -PropertyType $originalKind -Force | Out-Null
    }
    if (Test-Path $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
