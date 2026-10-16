$ErrorActionPreference = 'Stop'
$packageRoot = Split-Path $PSScriptRoot
python -m pip install -r (Join-Path $PSScriptRoot 'requirements-build.txt')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
python (Join-Path $PSScriptRoot 'build.py')
exit $LASTEXITCODE
