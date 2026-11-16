#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装测试】【注册表快照】通过隔离的真实 HKCU 键验证缺省值与原始类型，不修改实际启动项。 #>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Registry tests require Windows.' }
. (Join-Path $PSScriptRoot 'windows-registry-entry.ps1')
$subKey = "Software\MarkShot\PackageTests\$([guid]::NewGuid().ToString('N'))"
$key = $null
try {
    # 1. 【Windows安装测试】【缺省环境】缺少键和缺少值均返回空值，不中断首次安装验证
    if ($null -ne (Get-WindowsRegistryEntry -SubKey $subKey -Name MarkShot)) { throw 'Missing key returned a value.' }
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($subKey)
    if ($null -ne (Get-WindowsRegistryEntry -SubKey $subKey -Name MarkShot)) { throw 'Missing value returned an entry.' }
    # 2. 【Windows安装测试】【原值类型】空字符串、可展开字符串、数字及二进制必须保持内容和类型
    foreach ($case in @(
        @{ Value = ''; Kind = [Microsoft.Win32.RegistryValueKind]::String },
        @{ Value = '%USERPROFILE%\Mark Shot\mark-shot.exe'; Kind = [Microsoft.Win32.RegistryValueKind]::ExpandString },
        @{ Value = 42; Kind = [Microsoft.Win32.RegistryValueKind]::DWord },
        @{ Value = [byte[]]@(); Kind = [Microsoft.Win32.RegistryValueKind]::Binary }
    )) {
        $key.SetValue('MarkShot', $case.Value, $case.Kind)
        $entry = Get-WindowsRegistryEntry -SubKey $subKey -Name MarkShot
        if ($null -eq $entry -or $entry.Kind -ne $case.Kind.ToString()) { throw 'Registry value type was not preserved.' }
        if ($case.Kind -eq [Microsoft.Win32.RegistryValueKind]::Binary) {
            if ($entry.Value -isnot [byte[]] -or $entry.Value.Length -ne 0) { throw 'Empty binary value was not preserved.' }
        } elseif ($entry.Value -ne $case.Value) { throw 'Raw registry value was not preserved.' }
    }
    $key.DeleteValue('MarkShot')
    if ($null -ne (Get-WindowsRegistryEntry -SubKey $subKey -Name MarkShot)) { throw 'Deleted value returned an entry.' }
    Write-Host '【Windows安装测试】【注册表快照】缺省键、缺省值、原始值类型与删除检查通过'
}
finally {
    # 3. 【Windows安装测试】【环境恢复】只删除本次隔离键，不触碰当前用户启动注册
    if ($null -ne $key) { $key.Dispose() }
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($subKey, $false)
}
