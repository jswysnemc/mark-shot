#Requires -Version 7.0
<#
.SYNOPSIS
执行不依赖交互桌面或模型下载的 Windows 单元测试，并保存真实执行日志。
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Windows tests require a Windows runner.' }
$root = Split-Path $PSScriptRoot -Parent
$evidence = Join-Path $root 'target/windows-evidence'
New-Item -ItemType Directory -Force $evidence | Out-Null

function Invoke-UnitTests {
    <# .SYNOPSIS 执行一组测试；失败或零项通过均阻止流水线，避免过滤器失效造成假通过。 #>
    param([string]$Name, [string[]]$CargoArguments)
    $log = @(& cargo test --target x86_64-pc-windows-msvc --locked -j 2 @CargoArguments 2>&1)
    $code = $LASTEXITCODE
    $log | Tee-Object -FilePath (Join-Path $evidence "$Name.txt") | Write-Host
    if ($code -ne 0) { throw "Test group $Name failed ($code)." }
    if (($log -join "`n") -notmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed;') {
        throw "Test group $Name did not execute any passing tests."
    }
}

Push-Location $root
$oldPath = $env:PATH
$isolatedVariables = @('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'APPDATA', 'LOCALAPPDATA', 'LANG')
$oldEnvironment = @{}
foreach ($variable in $isolatedVariables) {
    $oldEnvironment[$variable] = [Environment]::GetEnvironmentVariable($variable)
}
try {
    foreach ($variable in $isolatedVariables | Where-Object { $_ -ne 'LANG' }) {
        $directory = Join-Path $root "target/windows-test-private/$variable"
        New-Item -ItemType Directory -Force $directory | Out-Null
        [Environment]::SetEnvironmentVariable($variable, $directory)
    }
    $env:LANG = 'en_US.UTF-8'
    # ort-sys 在无符号链接权限时只向 debug 复制 DLL；测试可从此目录解析依赖。
    $env:PATH = "$(Join-Path $root 'target/x86_64-pc-windows-msvc/debug');$oldPath"
    Invoke-UnitTests 'image-stitch' @('-p', 'shot-image', '-p', 'shot-stitch')
    $groups = @(
        'features::desktop::hotkeys::windows::tests::',
        'features::desktop::status::tests::',
        'features::recording::',
        'backend::window_detection::',
        'app::region_detection::',
        'app::input::tests::cursor_magnifier::',
        'backend::windows::native::',
        'backend::windows::clipboard::image_tests::',
        'config::tests::',
        'features::capture_history::tests::storage::',
        'ui::file_picker::save::tests::',
        'features::ocr_result::control_input::tests::',
        'features::ocr_result::input_method::tests::'
    )
    foreach ($group in $groups) {
        $name = $group.Replace('::', '-').TrimEnd('-')
        Invoke-UnitTests $name @('-p', 'mark-shot', '--lib', $group, '--', '--test-threads=1')
    }
    Invoke-UnitTests 'windows-ui-automation' @('-p', 'mark-shot', '--lib', 'backend::window_detection::windows::tests::', '--', '--ignored', '--test-threads=1', '--nocapture')
    Invoke-UnitTests 'cursor-magnifier-render' @('-p', 'mark-shot', '--test', 'cursor_magnifier_render')
    Invoke-UnitTests 'recording-arguments' @('-p', 'mark-shot', '--test', 'recording_arguments')
}
finally {
    $env:PATH = $oldPath
    foreach ($variable in $isolatedVariables) {
        [Environment]::SetEnvironmentVariable($variable, $oldEnvironment[$variable])
    }
    Pop-Location
}
