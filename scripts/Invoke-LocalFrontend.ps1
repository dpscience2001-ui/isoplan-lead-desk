$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$env:npm_config_cache = Join-Path $projectRoot ".npm-cache"
$env:TEMP = Join-Path $projectRoot ".tmp"
$env:TMP = $env:TEMP

New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
Set-Location -LiteralPath $projectRoot

& npm.cmd run dev
