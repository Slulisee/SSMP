param([string[]]$CargoArgs = @('build','--release'))
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot
$localToolchain = Join-Path $projectRoot 'work\toolchain'
$knownToolchain = 'C:\Users\Anonym\Documents\Codex\2026-10-12\cod\work\toolchain'
$toolchainRoot = if(Test-Path -LiteralPath $localToolchain){$localToolchain}else{$knownToolchain}
$compilerRoot = Join-Path $toolchainRoot 'rustup\toolchains\stable-x86_64-pc-windows-gnullvm\bin'
if(Test-Path -LiteralPath (Join-Path $compilerRoot 'cargo.exe')){
    $linkerRoot = (Get-ChildItem -LiteralPath $toolchainRoot -Directory -Filter 'llvm-mingw-*' | Select-Object -First 1).FullName
    $env:PATH = "$compilerRoot;$(Join-Path $linkerRoot 'bin');$env:PATH"
    $env:RUSTC = Join-Path $compilerRoot 'rustc.exe'
    $env:RUSTDOC = Join-Path $compilerRoot 'rustdoc.exe'
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER = Join-Path $linkerRoot 'bin\x86_64-w64-mingw32-clang.exe'
    $env:SSMP_WINDRES = Join-Path $linkerRoot 'bin\x86_64-w64-mingw32-windres.exe'
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    $cargo = Join-Path $compilerRoot 'cargo.exe'
}else{ $cargo = (Get-Command cargo -ErrorAction Stop).Source }
$env:CARGO_HOME = Join-Path $projectRoot 'work\cargo'
$env:CARGO_TARGET_DIR = Join-Path $projectRoot 'target'
$env:CARGO_INCREMENTAL = '0'
$env:TEMP = Join-Path $projectRoot 'work\temp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
$cargoCommand = $CargoArgs[0]
$cargoTail = @()
if($CargoArgs.Length -gt 1){$cargoTail = @($CargoArgs[1..($CargoArgs.Length-1)])}
& $cargo $cargoCommand --manifest-path (Join-Path $projectRoot 'Cargo.toml') @cargoTail
exit $LASTEXITCODE
