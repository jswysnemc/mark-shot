#Requires -Version 7.0
<# .SYNOPSIS 验证可分发目录中的程序能够在没有开发工具 PATH 的环境下启动。 #>

function Invoke-WindowsHelp {
    <# .SYNOPSIS 在包目录运行 --help，隔离 PATH 和配置目录并要求正常退出及真实帮助内容。 #>
    param([string]$PackageDirectory, [string]$OutputPath)
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = Join-Path $PackageDirectory 'mark-shot.exe'
    $start.ArgumentList.Add('--help')
    $start.WorkingDirectory = $PackageDirectory
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.Environment['PATH'] = "$([Environment]::SystemDirectory);$env:SystemRoot"
    $private = Join-Path $PackageDirectory '../help-private'
    foreach ($variable in @('APPDATA', 'LOCALAPPDATA', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME')) {
        $directory = Join-Path $private $variable
        New-Item -ItemType Directory -Force $directory | Out-Null
        $start.Environment[$variable] = $directory
    }
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        # 超时由工作流整体期限兜底；正常 help 应立即结束，不进行任何桌面交互。
        $process.WaitForExit()
        $output = $stdout.GetAwaiter().GetResult()
        $errors = $stderr.GetAwaiter().GetResult()
        $output | Set-Content -LiteralPath $OutputPath -Encoding utf8NoBOM
        if ($errors) { $errors | Add-Content -LiteralPath $OutputPath }
        if ($process.ExitCode -ne 0 -or $output -notmatch 'Usage: mark-shot' -or $output -notmatch '--ocr') {
            throw "Packaged --help failed ($($process.ExitCode)): $errors"
        }
    }
    finally {
        $process.Dispose()
    }
}
