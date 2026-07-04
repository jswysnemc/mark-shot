[CmdletBinding()]
param(
    [string]$Path = (Join-Path (Get-Location) "app"),
    [string]$CertificateBase64 = $env:WINDOWS_CODESIGN_CERTIFICATE_BASE64,
    [string]$CertificatePassword = $env:WINDOWS_CODESIGN_CERTIFICATE_PASSWORD,
    [string]$TimestampUrl = $env:WINDOWS_CODESIGN_TIMESTAMP_URL
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($CertificateBase64)) {
    Write-Host "Windows code signing certificate is not configured; skipping signing."
    exit 0
}

if ([string]::IsNullOrWhiteSpace($TimestampUrl)) {
    $TimestampUrl = "http://timestamp.digicert.com"
}

function Get-SignToolPath {
    $fromPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    $windowsKitsRoot = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (Test-Path $windowsKitsRoot) {
        $candidate = Get-ChildItem $windowsKitsRoot -Recurse -Filter signtool.exe |
            Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
            Sort-Object FullName -Descending |
            Select-Object -First 1
        if ($candidate) {
            return $candidate.FullName
        }
    }

    throw "signtool.exe was not found"
}

if (-not (Test-Path $Path)) {
    throw "Artifact path was not found: $Path"
}

$signTool = Get-SignToolPath
$certificatePath = Join-Path ([System.IO.Path]::GetTempPath()) "mark-shot-codesign.pfx"

try {
    [System.IO.File]::WriteAllBytes($certificatePath, [Convert]::FromBase64String($CertificateBase64))

    $files = Get-ChildItem $Path -Recurse -File -Include *.exe,*.dll | Sort-Object FullName
    if (-not $files) {
        throw "No Windows binaries were found under: $Path"
    }

    foreach ($file in $files) {
        $arguments = @(
            "sign",
            "/fd", "SHA256",
            "/tr", $TimestampUrl,
            "/td", "SHA256",
            "/f", $certificatePath
        )
        if (-not [string]::IsNullOrWhiteSpace($CertificatePassword)) {
            $arguments += @("/p", $CertificatePassword)
        }
        $arguments += $file.FullName

        & $signTool @arguments
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to sign $($file.FullName)"
        }
    }
}
finally {
    if (Test-Path $certificatePath) {
        Remove-Item $certificatePath -Force
    }
}
