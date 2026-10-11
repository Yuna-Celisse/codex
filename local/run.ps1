# Do not declare a CodexArgs parameter: PowerShell would bind CLI's -c/-C
# to that parameter and drop the flag. Unbound $args preserves every CLI flag.
$CodexArgs = @($args)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
# An owned transcript keeps the sidebar fixed while chat history scrolls.
# These are process overrides only; later user arguments can override them.
$CodexArgs = @('-c', 'tui.fullscreen_transcript=true') + @($CodexArgs)
$taskBinary = Join-Path $taskRoot 'codex-rs/target/debug/codex.exe'
if (-not (Test-Path -LiteralPath $taskBinary)) {
    $taskBinary = Join-Path $taskRoot 'codex-rs/target/release/codex.exe'
}
if (Test-Path -LiteralPath $taskBinary) {
    Write-Host 'Launching Codex built from this workspace.' -ForegroundColor DarkCyan
    & $taskBinary @CodexArgs
    exit $LASTEXITCODE
}
$taskSourceDiff = & git -C $taskRoot diff --name-only HEAD -- codex-rs
if ($LASTEXITCODE -ne 0) { throw 'Could not verify workspace source changes.' }
if ($taskSourceDiff) {
    throw 'Workspace Rust source has changed. Run local/build.ps1 first; the installed official binary cannot display these changes.'
}
$taskCommand = Get-Command codex -ErrorAction Stop
$taskVersion = (& $taskCommand.Source --version | Out-String).Trim()
if ($taskVersion -ne 'codex-cli 0.162.0') {
    throw "No workspace build exists. Expected official codex-cli 0.162.0, found '$taskVersion'. Build with local/build.ps1 first."
}
Write-Host 'No workspace build exists; launching the matching official release 0.162.0. Source edits require a build.' -ForegroundColor Yellow
& $taskCommand.Source @CodexArgs
exit $LASTEXITCODE
