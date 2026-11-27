# 打 Windows 发布包：编译 → 组装 → 压缩 → 算 SHA256。
#
#   powershell -ExecutionPolicy Bypass -File tool/package.ps1
#   powershell -ExecutionPolicy Bypass -File tool/package.ps1 -SkipBuild   # 复用已有产物
#
# 产物在 dist/：
#   xdxk-gui-<版本>-windows-x64/        解压即用的目录
#   xdxk-gui-<版本>-windows-x64.zip     发给别人就是这个
#   SHA256SUMS.txt

param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root

# 版本号取自 pubspec.yaml 的 version: 0.2.0+1
$versionLine = Select-String -Path 'pubspec.yaml' -Pattern '^version:\s*([^+\s]+)' | Select-Object -First 1
if (-not $versionLine) { throw 'pubspec.yaml 里没有 version 字段' }
$version = $versionLine.Matches[0].Groups[1].Value

$name = "xdxk-gui-$version-windows-x64"
$releaseDir = 'build/windows/x64/runner/Release'
$distDir = 'dist'
$stage = Join-Path $distDir $name
$zip = Join-Path $distDir "$name.zip"

if (-not $SkipBuild) {
    Write-Host "编译 Release（首次会连带编译 Rust，几分钟）…"
    flutter build windows --release
    if ($LASTEXITCODE -ne 0) { throw 'flutter build 失败' }
}

if (-not (Test-Path (Join-Path $releaseDir 'xdxk_gui.exe'))) {
    throw "找不到 $releaseDir/xdxk_gui.exe"
}

if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null

# 运行时文件：exe、桌面 embedder、插件 dll、data/
Copy-Item (Join-Path $releaseDir 'xdxk_gui.exe') (Join-Path $stage 'xdxk-gui.exe')
Copy-Item (Join-Path $releaseDir 'flutter_windows.dll') $stage
Copy-Item (Join-Path $releaseDir 'data') $stage -Recurse
Get-ChildItem $releaseDir -Filter '*.dll' |
    Where-Object { $_.Name -ne 'flutter_windows.dll' } |
    ForEach-Object { Copy-Item $_.FullName $stage }

# 验证码模型必须和 exe 在一起：程序会先看 exe 同目录
Copy-Item 'models' $stage -Recurse
# 自带一份默认配置，双击就能用；配置默认读写 exe 旁边这一份
Copy-Item 'conf.default.json' (Join-Path $stage 'conf.json')
Copy-Item 'README.md' $stage

# 版本说明
$notes = @"
西电选课 $version（Windows x64）
构建时间：$(Get-Date -Format 'yyyy-MM-dd HH:mm')

怎么用
  1. 双击 xdxk-gui.exe
  2. 在「登录」页填学号密码，登录后选批次
  3. 在「课程」页拉列表挑课，或在「任务」页填课程号 + 课序号
  4. 「任务」页可以保存配置、只读诊断、开始执行

配置文件就是这个文件夹里的 conf.json，界面和命令行读的是同一份。
不想让它待在这儿：「概览 → 配置文件 → 另存为…」换个位置即可。
conf.json 里存着学号密码，把文件夹发给别人前先清空。
models/ 目录必须和 exe 放在一起，删了就只能手动输验证码。
打不开的话先装 Microsoft Visual C++ 2015-2022 运行时（x64）。
"@
$notes | Out-File -FilePath (Join-Path $stage '使用说明.txt') -Encoding utf8

if (Test-Path $zip) { Remove-Item $zip -Force }
Write-Host "压缩 $zip …"
$sevenZip = Get-Command 7z -ErrorAction SilentlyContinue
if ($sevenZip) {
    Push-Location $distDir
    & 7z a -tzip -mx=7 "$name.zip" "$name" | Out-Null
    Pop-Location
    if ($LASTEXITCODE -ne 0) { throw '7z 压缩失败' }
} else {
    Compress-Archive -Path $stage -DestinationPath $zip -CompressionLevel Optimal
}

# 校验值
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
$line = "$hash  $(Split-Path $zip -Leaf)  $([math]::Round((Get-Item $zip).Length / 1MB, 1)) MB"
# 不带 BOM，sha256sum / certutil 都能直接读
[System.IO.File]::WriteAllText(
    (Join-Path $distDir 'SHA256SUMS.txt'),
    $line + [Environment]::NewLine,
    (New-Object System.Text.UTF8Encoding($false)))

Write-Host ''
Write-Host "完成：$zip"
Write-Host $line
Write-Host "目录：$stage"
