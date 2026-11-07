#Requires -Version 7.0
<# .SYNOPSIS 提供 Windows 运行库收集与 PE 导入表验证，不依赖 PATH 搜索第三方 DLL。 #>

function Get-PeDependencies {
    <# .SYNOPSIS 检查 PE 为 x64，并返回普通及延迟导入的 DLL 名称，同时记录 dumpbin 证据。 #>
    param([string]$Path, [string]$LogPath)
    $headers = @(& dumpbin /nologo /headers $Path 2>&1)
    if ($LASTEXITCODE -ne 0 -or ($headers -join "`n") -notmatch '8664 machine') {
        throw "Not an x64 PE image: $Path"
    }
    $imports = @(& dumpbin /nologo /dependents $Path 2>&1)
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect imports: $Path" }
    @("### $Path") + $headers + $imports | Add-Content -LiteralPath $LogPath
    $names = @($imports | ForEach-Object {
        if ("$_" -match '^\s+([^\s\\/]+\.dll)\s*$') { $Matches[1] }
    } | Sort-Object -Unique)
    if ($names.Count -eq 0) { throw "No DLL imports found: $Path" }
    return $names
}

function Copy-RuntimeFile {
    <# .SYNOPSIS 按字节复制运行库，展开 ort-sys 的符号链接，避免压缩包携带构建机链接。 #>
    param([string]$Source, [string]$Destination)
    [System.IO.File]::WriteAllBytes($Destination, [System.IO.File]::ReadAllBytes($Source))
}

function Resolve-WindowsRuntime {
    <# .SYNOPSIS 递归补齐应用本地依赖或仅验证解压包；拒绝 Qt、错误架构和缺失 DLL。 #>
    param(
        [string]$PackageDirectory,
        [string]$LogPath,
        [string]$BuildDirectory,
        [switch]$VerifyOnly
    )
    if (-not (Test-Path (Join-Path $PackageDirectory 'DirectML.dll') -PathType Leaf)) {
        throw 'CPU OCR still requires the DirectML.dll shipped by ort-sys.'
    }
    $crtDirectories = @()
    if (-not $VerifyOnly) {
        if (-not $env:VCToolsRedistDir) { throw 'VCToolsRedistDir is missing; configure MSVC first.' }
        $crtDirectories = @(Get-ChildItem (Join-Path $env:VCToolsRedistDir 'x64') -Directory -Filter 'Microsoft.VC*.CRT')
        if ($crtDirectories.Count -ne 1) { throw 'Expected exactly one x64 MSVC CRT directory.' }
    }
    $queue = [System.Collections.Generic.Queue[string]]::new()
    Get-ChildItem $PackageDirectory -File | Where-Object { $_.Extension -in '.exe', '.dll' } |
        ForEach-Object { $queue.Enqueue($_.FullName) }
    $seen = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    while ($queue.Count -gt 0) {
        $image = $queue.Dequeue()
        if (-not $seen.Add($image)) { continue }
        if ([IO.Path]::GetFileName($image) -match '^Qt.*\.dll$') { throw "Qt is forbidden: $image" }
        foreach ($name in (Get-PeDependencies $image $LogPath)) {
            if ($name -match '^Qt.*\.dll$') { throw "Qt dependency is forbidden: $name" }
            $local = Join-Path $PackageDirectory $name
            if (Test-Path $local -PathType Leaf) {
                $queue.Enqueue($local)
                continue
            }
            # API Set 合约由 Windows 加载器解析，不是应随应用复制的文件。
            if ($name -match '^(api-ms-win-|ext-ms-win-)') { continue }
            $isCrt = $name -match '^(msvcp|vcruntime|concrt|vcomp)\d.*\.dll$'
            if (-not $VerifyOnly) {
                $candidate = Join-Path $BuildDirectory $name
                if ($isCrt) { $candidate = Join-Path $crtDirectories[0].FullName $name }
                if (Test-Path $candidate -PathType Leaf) {
                    Copy-RuntimeFile $candidate $local
                    $queue.Enqueue($local)
                    continue
                }
            }
            # 不把 runner 已安装的 VC++ 运行库误当作系统自带依赖。
            if ($isCrt) { throw "Missing app-local MSVC runtime: $name" }
            $system = Join-Path ([Environment]::SystemDirectory) $name
            if (-not (Test-Path $system -PathType Leaf)) { throw "Unresolved dependency: $name ($image)" }
            "SYSTEM $name -> $system" | Add-Content -LiteralPath $LogPath
        }
    }
}
