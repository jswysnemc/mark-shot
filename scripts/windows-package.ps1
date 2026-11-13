#Requires -Version 7.0
<# .SYNOPSIS 打包默认 CPU OCR 的 Windows x64 程序、实际运行依赖、源码标识和校验和。 #>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$SourceSha,
    [switch]$Sign
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Packaging requires Windows and the MSVC x64 tools.' }
. (Join-Path $PSScriptRoot 'windows-runtime.ps1')
. (Join-Path $PSScriptRoot 'windows-smoke.ps1')

$root = Split-Path $PSScriptRoot -Parent
$build = Join-Path $root 'target/x86_64-pc-windows-msvc/release'
$evidence = Join-Path $root 'target/windows-evidence'
$name = 'mark-shot-native-windows-x86_64-msvc'
# 使用全新目录，不清除或覆盖调用者的旧暂存内容。
$staging = Join-Path $root "target/windows-package-$([guid]::NewGuid().ToString('N'))"
$package = Join-Path $staging $name
New-Item -ItemType Directory -Force $package, $evidence | Out-Null
New-Item -ItemType Directory (Join-Path $package 'licenses'), (Join-Path $package 'docs') | Out-Null

Push-Location $root
try {
    $actualSha = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actualSha -ne $SourceSha) { throw 'SOURCE-COMMIT must identify the checked-out source.' }
    $dirty = @(& git status --porcelain --untracked-files=no)
    if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) { throw 'Tracked source files changed after checkout.' }
    Copy-Item (Join-Path $build 'mark-shot.exe') $package
    Copy-Item (Join-Path $root 'target/media-windows') (Join-Path $package 'media') -Recurse
    $linkage = Join-Path $package 'windows-linkage.txt'
    Resolve-WindowsRuntime -PackageDirectory $package -BuildDirectory $build -LogPath $linkage
    Copy-Item LICENSE-MIT, LICENSE-APACHE, README.md $package
    Copy-Item docs/*.md (Join-Path $package 'docs')
    Copy-Item docs/recording (Join-Path $package 'docs/recording') -Recurse
    Copy-Item assets/fonts/LICENSE-OFL.txt, assets/fonts/LICENSE-NUMERALS.txt (Join-Path $package 'licenses')
    Copy-Item assets/icons/mark-shot/LICENSE (Join-Path $package 'licenses/LICENSE-ICONS.txt')
    Copy-Item Cargo.lock (Join-Path $package 'DEPENDENCIES-Cargo.lock')
    $SourceSha.ToLowerInvariant() | Set-Content (Join-Path $package 'SOURCE-COMMIT') -Encoding utf8NoBOM
    @'
Windows x86_64 MSVC / default features / CPU OCR
Keep mark-shot.exe and every DLL in this directory together.
Choose Windows system OCR or local model OCR on first use.
The default package contains neither ONNX Runtime nor DirectML.
Local OCR requires separate manual downloads of its runtime and language models.
Recording uses Windows Graphics Capture and Media Foundation, with a software fallback.
The media subdirectory includes the in-process GIF/MKV shared library, notices and corresponding sources.
Windows system DLLs are supplied by the OS; required MSVC CRT DLLs are app-local.
This is a CI build artifact, not a release or proof of full Windows desktop compatibility.
'@ | Set-Content (Join-Path $package 'BUILD-NOTES.txt') -Encoding utf8NoBOM
    if ($Sign) { & (Join-Path $PSScriptRoot 'windows-sign-artifact.ps1') -Path $package }
    Invoke-WindowsHelp -PackageDirectory $package -OutputPath (Join-Path $package 'windows-help.txt')
    # 清单覆盖包内每个文件（不包含清单本身），另对最终 ZIP 计算外部 SHA256。
    $hashes = foreach ($file in (Get-ChildItem $package -File -Recurse | Sort-Object FullName)) {
        $relative = [IO.Path]::GetRelativePath($package, $file.FullName).Replace('\', '/')
        '{0}  {1}' -f (Get-FileHash $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $relative
    }
    $hashes | Set-Content (Join-Path $package 'SHA256SUMS') -Encoding utf8NoBOM
    $archive = Join-Path $root "target/$name.zip"
    Compress-Archive -LiteralPath $package -DestinationPath $archive -CompressionLevel Optimal -Force
    $unpacked = Join-Path $staging 'unpacked'
    Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
    $extracted = Join-Path $unpacked $name
    foreach ($line in (Get-Content (Join-Path $extracted 'SHA256SUMS'))) {
        if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw "Invalid SHA256SUMS entry: $line" }
        $expected = $Matches[1]
        $file = Join-Path $extracted $Matches[2]
        if ((Get-FileHash $file -Algorithm SHA256).Hash -ne $expected) { throw "Archive checksum mismatch: $file" }
    }
    Resolve-WindowsRuntime -PackageDirectory $extracted -VerifyOnly -LogPath (Join-Path $evidence 'unpacked-linkage.txt')
    Invoke-WindowsHelp -PackageDirectory $extracted -OutputPath (Join-Path $evidence 'unpacked-help.txt')
    '{0}  {1}.zip' -f (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant(), $name |
        Set-Content "$archive.sha256" -Encoding ascii
    Copy-Item (Join-Path $package 'SOURCE-COMMIT'), (Join-Path $package 'SHA256SUMS'), $linkage $evidence
}
finally {
    Pop-Location
}
