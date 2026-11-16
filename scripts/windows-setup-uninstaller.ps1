#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装】【卸载定位】根据当前用户的安装登记定位本次安装目录中的实际卸载程序。 #>

function Get-WindowsSetupUninstaller {
    <#
    .SYNOPSIS
    【Windows安装】【卸载查询】按 InstallDirectory 返回对应安装登记的卸载程序路径；登记不唯一、路径越界或文件缺失时报错。
    #>
    param([Parameter(Mandatory)][string]$InstallDirectory)

    $directory = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\')
    $rootKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Uninstall')
    if ($null -eq $rootKey) { throw 'Installation has no uninstall registry key.' }
    $candidates = [Collections.Generic.List[string]]::new()
    try {
        # 1. 【Windows安装】【目录匹配】安装标识可能经安装器缩短，只根据本次独占安装目录匹配登记
        foreach ($name in $rootKey.GetSubKeyNames()) {
            $key = $rootKey.OpenSubKey($name)
            if ($null -eq $key) { continue }
            try {
                $location = [string]$key.GetValue('InstallLocation', '')
                if ($location.TrimEnd('\') -ne $directory) { continue }
                $command = [string]$key.GetValue('UninstallString', '')
                if ($command -notmatch '^"([^"]+)"$') { throw 'Unexpected installer uninstall command.' }
                $file = [IO.Path]::GetFullPath($Matches[1])
                # 2. 【Windows安装】【路径归属】只执行本次安装目录中的卸载程序，不采用旧编号或其他目录
                if ([IO.Path]::GetDirectoryName($file).TrimEnd('\') -ne $directory -or [IO.Path]::GetExtension($file) -ne '.exe') {
                    throw 'Uninstaller does not belong to this installation directory.'
                }
                if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw 'Registered uninstaller file is missing.' }
                $candidates.Add($file)
            }
            finally { $key.Dispose() }
        }
    }
    finally { $rootKey.Dispose() }
    if ($candidates.Count -ne 1) { throw 'Installation must have exactly one registered uninstaller.' }
    return $candidates[0]
}
