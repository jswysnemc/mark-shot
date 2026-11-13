#Requires -Version 7.0
<# .SYNOPSIS 【原生发布】【Windows签名】签名应用程序与私有媒体库，不修改系统运行库；缺少证书时跳过。 #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Path,
    [string]$CertificateBase64 = $env:WINDOWS_CODESIGN_CERTIFICATE_BASE64,
    [string]$CertificatePassword = $env:WINDOWS_CODESIGN_CERTIFICATE_PASSWORD,
    [string]$TimestampUrl = $env:WINDOWS_CODESIGN_TIMESTAMP_URL
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($CertificateBase64)) {
    Write-Host '【原生发布】【Windows签名】未配置证书，生成未签名产物'
    return
}
if ([string]::IsNullOrWhiteSpace($TimestampUrl)) { $TimestampUrl = 'http://timestamp.digicert.com' }
if (-not (Test-Path $Path -PathType Container)) { throw "Artifact directory not found: $Path" }
$tool = Get-Command signtool.exe -ErrorAction SilentlyContinue
$signTool = if ($tool) { $tool.Source } else {
    $kits = "${env:ProgramFiles(x86)}/Windows Kits/10/bin"
    (Get-ChildItem $kits -Recurse -Filter signtool.exe |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending | Select-Object -First 1).FullName
}
if (-not $signTool) { throw 'signtool.exe was not found' }
$certificate = Join-Path ([IO.Path]::GetTempPath()) "mark-shot-$([guid]::NewGuid().ToString('N')).pfx"
try {
    [IO.File]::WriteAllBytes($certificate, [Convert]::FromBase64String($CertificateBase64))
    foreach ($relative in @('mark-shot.exe', 'media/markshot-media.dll')) {
        $file = Join-Path $Path $relative
        if (-not (Test-Path $file -PathType Leaf)) { throw "Signing input missing: $relative" }
        $arguments = @('sign', '/fd', 'SHA256', '/tr', $TimestampUrl, '/td', 'SHA256', '/f', $certificate)
        if (-not [string]::IsNullOrWhiteSpace($CertificatePassword)) { $arguments += @('/p', $CertificatePassword) }
        & $signTool @arguments $file
        if ($LASTEXITCODE -ne 0) { throw "Signing failed: $relative" }
        & $signTool verify /pa $file
        if ($LASTEXITCODE -ne 0) { throw "Signature verification failed: $relative" }
    }
    # 1. 【原生发布】【媒体摘要】签名会改变动态库字节，重新写入媒体包摘要
    $media = Join-Path $Path 'media'
    $hashes = foreach ($name in @('markshot-media.dll', 'MEDIA-SOURCES.tar.xz')) {
        '{0}  {1}' -f (Get-FileHash (Join-Path $media $name) -Algorithm SHA256).Hash.ToLowerInvariant(), $name
    }
    $hashes | Set-Content (Join-Path $media 'SHA256SUMS') -Encoding ascii
}
finally {
    if (Test-Path $certificate) { Remove-Item $certificate -Force }
}
