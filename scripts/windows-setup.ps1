#Requires -Version 7.0
<# .SYNOPSIS 【Windows安装】【打包入口】从已验证便携包生成 Setup，保留版本、源码和签名并验证安装卸载。 #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$SourceSha,
    [ValidatePattern('^$|^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$')]
    [string]$Version,
    [string]$Compiler,
    [switch]$Sign
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Setup packaging requires Windows.' }
$root = Split-Path $PSScriptRoot -Parent
$name = 'mark-shot-native-windows-x86_64-msvc'
$archive = Join-Path $root "target/$name.zip"
$temporary = Join-Path $root "target/windows-setup-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory $temporary | Out-Null
try {
    # 1. 【Windows安装】【来源校验】只接受摘要及源码标识与当前构建一致的便携包
    $sidecar = (Get-Content "$archive.sha256" -Raw).Trim()
    $expected = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($sidecar -ne "$expected  $name.zip") { throw 'Portable archive checksum mismatch.' }
    Expand-Archive -LiteralPath $archive -DestinationPath $temporary
    $package = Join-Path $temporary $name
    if ((Get-Content (Join-Path $package 'SOURCE-COMMIT') -Raw).Trim() -ne $SourceSha) {
        throw 'Setup source does not match the verified portable build.'
    }
    $metadata = & cargo metadata --no-deps --format-version 1 --locked --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed.' }
    $cargoVersion = ($metadata.packages | Where-Object { $_.name -eq 'mark-shot' }).version
    if ([string]::IsNullOrWhiteSpace($Version)) { $Version = $cargoVersion }
    if (($Version -split '[-+]')[0] -ne $cargoVersion) { throw 'Setup version does not match Cargo.toml.' }
    $fileVersion = ($Version -split '[-+]')[0] + '.0'

    # 2. 【Windows安装】【安装编译】安装器使用已验证目录，不重新收集运行依赖
    if ([string]::IsNullOrWhiteSpace($Compiler)) {
        $tool = Get-Command ISCC.exe -ErrorAction SilentlyContinue
        $Compiler = if ($tool) { $tool.Source } else { "${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe" }
    }
    if (-not (Test-Path $Compiler -PathType Leaf)) { throw 'Inno Setup 6.4 or later is required.' }
    & $Compiler "/DPackageDirectory=$package" "/DPackageVersion=$Version" "/DFileVersion=$fileVersion" "/DOutputDirectory=$temporary" (Join-Path $root 'packaging/windows/mark-shot.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Setup compilation failed.' }
    $compiled = Join-Path $temporary "$name-setup.exe"
    if ($Sign) { & (Join-Path $PSScriptRoot 'windows-sign-artifact.ps1') -Path $compiled }

    # 3. 【Windows安装】【安装验证】签名后执行静默安装、文件核验、程序启动与卸载，再生成最终摘要
    & (Join-Path $PSScriptRoot 'windows-setup-smoke.ps1') -Installer $compiled -PackageDirectory $package
    $destination = Join-Path $root "target/$name-setup.exe"
    Copy-Item -LiteralPath $compiled -Destination $destination -Force
    '{0}  {1}' -f (Get-FileHash $destination -Algorithm SHA256).Hash.ToLowerInvariant(), "$name-setup.exe" |
        Set-Content "$destination.sha256" -Encoding ascii
}
finally {
    if (Test-Path $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
