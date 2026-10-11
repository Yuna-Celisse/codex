param(
    [switch] $Release,
    [string] $Toolchain = '1.95.0'
)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (Test-Path -LiteralPath $taskVsWhere) {
    $taskVs = & $taskVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($taskVs) {
        $taskDevCmd = Join-Path $taskVs 'Common7/Tools/VsDevCmd.bat'
        $taskEnvCommand = ('"{0}" -no_logo -arch=x64 -host_arch=x64 & set' -f $taskDevCmd)
        $taskEnvironment = & $env:ComSpec /c $taskEnvCommand
        if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the Visual Studio build environment.' }
        foreach ($taskLine in $taskEnvironment) {
            if ($taskLine -match '^([^=]+)=(.*)$') {
                [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
            }
        }
    }
}
Push-Location (Join-Path $taskRoot 'codex-rs')
try {
    $taskArgs = @(('+' + $Toolchain), 'build', '--locked', '-p', 'codex-cli', '--bin', 'codex')
    if ($Release) { $taskArgs += '--release' }
    & cargo @taskArgs
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed ($LASTEXITCODE)." }
} finally { Pop-Location }
