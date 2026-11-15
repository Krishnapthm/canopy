# Installs Canopy, and herdr-core if there's none yet, from PowerShell:
#   irm https://github.com/Krishnapthm/canopy/releases/latest/download/install.ps1 | iex
# Installs into %LOCALAPPDATA%\Programs\canopy and adds canopy to PATH. The
# core comes from its own installer, only if none is there yet. Running this
# again updates the app. CANOPY_RELEASES_URL serves the app from elsewhere.

# Its own scope: under `irm | iex` it would otherwise set the caller's variables.
& {
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# throw, not exit: under `irm | iex`, exit would close the person's window.
function Fail($why) { throw "canopy: $why" }

$releases = if ($env:CANOPY_RELEASES_URL) { $env:CANOPY_RELEASES_URL.TrimEnd("/") } else { "https://github.com/Krishnapthm/canopy/releases/latest/download" }
$dir = Join-Path $env:LOCALAPPDATA "Programs\canopy"
$arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($arch -ne "AMD64") { Fail "no build for $arch Windows yet (x64 only)" }
$target = "windows-x86_64"
# Windows' own tar: Git Bash's, often first on PATH, takes C:\ for a remote host.
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
if (-not (Get-Command curl.exe -ErrorAction SilentlyContinue) -or -not (Test-Path $tar)) {
    Fail "needs curl.exe and tar.exe (Windows 10 1803 or newer)"
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) "canopy-install-$PID"
New-Item -ItemType Directory -Force $tmp | Out-Null
try {
    # SHA256SUMS lists every archive of the release; `sha256sum -c` format.
    $archive = "canopy-$target.tar.gz"
    curl.exe -fsSL "$releases/SHA256SUMS" -o "$tmp\SHA256SUMS"
    if ($LASTEXITCODE) { Fail "cannot reach $releases" }
    $want = Get-Content "$tmp\SHA256SUMS" | ForEach-Object { if ($_ -match "^([0-9a-f]{64})  $([regex]::Escape($archive))$") { $Matches[1] } } | Select-Object -First 1
    if (-not $want) { Fail "no build for $target yet" }

    Write-Host "Installing Canopy for $target..."
    curl.exe -fsSL "$releases/$archive" -o "$tmp\app.tar.gz"
    if ($LASTEXITCODE) { Fail "download failed: $releases/$archive" }
    # .NET, not Get-FileHash: that one fails when PowerShell 7 starts 5.1.
    $stream = [IO.File]::OpenRead("$tmp\app.tar.gz")
    try { $got = -join ([Security.Cryptography.SHA256]::Create().ComputeHash($stream) | ForEach-Object { $_.ToString("x2") }) } finally { $stream.Close() }
    if ($got -ne $want) { Fail "the download is damaged (checksum mismatch); try again" }
    & $tar -xzf "$tmp\app.tar.gz" -C $tmp
    if ($LASTEXITCODE -or -not (Test-Path "$tmp\canopy\canopy.exe")) { Fail "the download holds no complete app" }

    # A running app's folder can be renamed, not deleted: the old one goes
    # now if it can, else on the next install.
    New-Item -ItemType Directory -Force (Split-Path $dir) | Out-Null
    Remove-Item -Recurse -Force "$dir.old" -ErrorAction SilentlyContinue
    if (Test-Path $dir) { Rename-Item $dir "$dir.old" }
    Move-Item "$tmp\canopy" $dir
    Remove-Item -Recurse -Force "$dir.old" -ErrorAction SilentlyContinue

    $exe = Join-Path $dir "canopy.exe"
    $menu = Join-Path ([Environment]::GetFolderPath("Programs")) "Canopy.lnk"
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($menu)
    $link.TargetPath = $exe
    $link.WorkingDirectory = $env:USERPROFILE
    $link.Description = "Terminals and coding agents that keep running"
    $link.Save()

    # The raw user PATH, so %VARIABLES% in it stay unexpanded and keep their type.
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
    $raw = [string]$key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $listed = $raw -split ";" | Where-Object { $_.Trim().TrimEnd("\") -ieq $dir.TrimEnd("\") }
    if (-not $listed) {
        $kind = if ($key.GetValueNames() -contains "Path") { $key.GetValueKind("Path") } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
        $key.SetValue("Path", $(if ($raw) { "$($raw.TrimEnd(';'));$dir" } else { $dir }), $kind)
        # Setting any user variable tells running programs the environment changed.
        [Environment]::SetEnvironmentVariable("CANOPY_INSTALLING", "1", "User")
        [Environment]::SetEnvironmentVariable("CANOPY_INSTALLING", $null, "User")
    }
    $key.Close()
    if (($env:Path -split ";") -notcontains $dir) { $env:Path += ";$dir" }

    # The core: installed only if there's none yet. One that's already there
    # (yours, herdr's, or a package manager's) is left alone.
    $core = Join-Path $env:LOCALAPPDATA "Programs\herdr-core\herdr-core.exe"
    if (-not (Test-Path $core) -and -not (Get-Command herdr-core -ErrorAction SilentlyContinue)) {
        Write-Host "Installing herdr-core, the terminal engine Canopy runs on..."
        try {
            Invoke-RestMethod "https://releases.herdr.dev/core/install.ps1" | Invoke-Expression
        } catch {
            Write-Host "herdr-core didn't install; Canopy will show how when it opens."
        }
    }

    Write-Host "Canopy is installed. Open it from the Start menu, or run: canopy"
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
}
