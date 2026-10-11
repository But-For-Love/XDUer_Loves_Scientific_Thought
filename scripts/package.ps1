# 打 Release 包：编译 → 自检 → 组装 → 压缩 → 校验值 → 渲染发布说明。
# 本地：pwsh scripts/package.ps1 -Tag v0.1.0[ -SkipBuild]
# CI  ：.github/workflows/release.yml 在 tag 推送时调用。
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$OutDir = 'dist',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Read-Utf8([string]$Path) {
    # PS 5.1 的 Get-Content 对无 BOM 的 UTF-8 会按 ANSI 解码，中文会乱码。
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
}

function Write-Utf8([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding($false)))
}

# ---- 版本一致性 ----
if ($Tag -notmatch '^v\d+\.\d+\.\d+$') {
    throw "tag 格式应为 vX.Y.Z（如 v0.1.0），当前为：$Tag"
}
$version = $Tag.Substring(1)

$m = [regex]::Match((Read-Utf8 'Cargo.toml'), '(?m)^version\s*=\s*"([^"]+)"')
if (-not $m.Success) { throw 'Cargo.toml 中未找到 package.version' }
$cargoVersion = $m.Groups[1].Value
if ($cargoVersion -ne $version) {
    throw "Cargo.toml version=$cargoVersion 与 tag $Tag 不一致（先改版本号再打 tag）"
}

# ---- 编译 ----
if (-not $SkipBuild) {
    Write-Host "==> cargo build --release --locked"
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo build 失败' }
}

$exe = Join-Path $root 'target/release/xdxk-rs.exe'
if (-not (Test-Path $exe)) { throw "找不到编译产物：$exe" }
foreach ($f in @('conf.example.json', 'README.md', 'models/common_lite.onnx')) {
    if (-not (Test-Path (Join-Path $root $f))) { throw "缺少打包所需文件：$f" }
}

# ---- 自检（确保打出来的 exe 真能跑）----
$selfTest = & $exe --version
if ($LASTEXITCODE -ne 0) { throw "自检失败：$exe --version 退出码 $LASTEXITCODE" }
if ($selfTest -notmatch [regex]::Escape($version)) {
    throw "自检失败：--version 输出 `"$selfTest`" 不含 $version"
}
Write-Host "==> 自检通过：$selfTest"

# ---- 组装 ----
$name = "xdxk-rs-$Tag-windows-x64"
$stage = Join-Path (Join-Path $root $OutDir) $name
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $stage 'models') -Force | Out-Null
Copy-Item $exe -Destination $stage
Copy-Item (Join-Path $root 'conf.example.json') -Destination $stage
Copy-Item (Join-Path $root 'README.md') -Destination $stage
Copy-Item (Join-Path $root 'models/common_lite.onnx') -Destination (Join-Path $stage 'models')

# ---- 压缩 ----
$zip = Join-Path (Join-Path $root $OutDir) "$name.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
$sevenZip = Get-Command 7z -ErrorAction SilentlyContinue
Push-Location (Split-Path -Parent $zip)
try {
    if ($sevenZip) {
        & 7z a -tzip -mx=9 "$name.zip" "$name" | Out-Null
        if ($LASTEXITCODE -ne 0) { throw '7z 打包失败' }
    } else {
        Compress-Archive -Path $name -DestinationPath "$name.zip" -CompressionLevel Optimal
    }
} finally {
    Pop-Location
}

# ---- 校验值 ----
$zipHash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
$exeHash = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
$zipSize = [math]::Round((Get-Item $zip).Length / 1MB, 1)
$sumsPath = Join-Path (Split-Path -Parent $zip) 'SHA256SUMS.txt'
Write-Utf8 $sumsPath "$zipHash  $name.zip`n$exeHash  xdxk-rs.exe`n"

# ---- 发布说明 ----
$notesPath = Join-Path (Split-Path -Parent $zip) 'release-notes.md'
$body = Read-Utf8 (Join-Path $root '.github/release-notes-template.md')
foreach ($pair in @(
        @('__TAG__', $Tag),
        @('__VERSION__', $version),
        @('__ZIP_NAME__', "$name.zip"),
        @('__ZIP_SIZE__', "$zipSize"),
        @('__SHA256_ZIP__', $zipHash),
        @('__SHA256_EXE__', $exeHash),
        @('__DATE__', (Get-Date -Format 'yyyy-MM-dd'))
    )) {
    $body = $body.Replace($pair[0], $pair[1])
}
if ($body -match '__[A-Z_]+__') { throw "发布说明模板有未替换的占位符：$($Matches[0])" }
Write-Utf8 $notesPath $body

Write-Host "==> 产物："
Write-Host "    $zip  ($zipSize MB)"
Write-Host "    $sumsPath"
Write-Host "    $notesPath"
