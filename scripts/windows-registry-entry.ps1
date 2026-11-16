#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装】【注册表快照】读取当前用户可缺省的注册表值，保留原始内容及类型。 #>

function Get-WindowsRegistryEntry {
    <#
    .SYNOPSIS
    【Windows安装】【值读取】按 SubKey 子键路径与 Name 值名返回 Value、Kind 快照；键或值不存在时返回空值。
    #>
    param(
        [Parameter(Mandatory)][string]$SubKey,
        [Parameter(Mandatory)][string]$Name
    )

    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($SubKey)
    if ($null -eq $key) { return $null }
    try {
        # 1. 【Windows安装】【缺省判断】缺少值属于正常未注册状态，不抑制权限或其他注册表错误
        if ($key.GetValueNames() -notcontains $Name) { return $null }
        # 2. 【Windows安装】【原值保留】不展开环境变量，恢复时保持字符串、二进制等原始值及类型
        return [pscustomobject]@{
            Value = $key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            Kind = $key.GetValueKind($Name).ToString()
        }
    }
    finally { $key.Dispose() }
}
