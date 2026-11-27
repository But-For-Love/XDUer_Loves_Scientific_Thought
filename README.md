> [!CAUTION]
>
> # 本项目不保证能用。
>
> 煮啵自己在用的时候就遇到过事故。在选必修课的时候，我在配置文件里填了要抢的英语课和体育课。选课开始时我启动了程序，日志输出结果说是都成功了，可我登上网站一看发现只有英语选上了，体育没有，吓得我赶紧手动抢课去了。
>
> 个人猜测是学校有反脚本措施，短时间内抢多门课会导致后面的课无效。
>
> 但是抢选修课的时候却没有遇到这个问题（
>
> 一年就这么几次选课机会，我也没办法复现bug，也没法进行检查与修复...
>
> 总之，祝愿大家都能抢到自己喜欢的科学思维(x

# 西电选课工具

西安电子科技大学选课 / 退课工具。内核是 Rust，界面是 Flutter（终末地风格），命令行版本也还在。
自动识别验证码、自动登录、抢课失败自动重试，支持 **选课、退课、只读诊断**。

识别模型（`models/common_lite.onnx`）已随仓库提供，clone 下来即可用，无需额外下载。

> ⚠️ 仅供学习交流。请遵守学校选课规则、控制请求频率。使用产生的一切后果由使用者自行承担。

## 下载即用

到 [Releases](https://github.com/But-For-Love/XDUer_Loves_Scientific_Thought/releases) 下载
`xdxk-gui-<版本>-windows-x64.zip`，解压后双击 `xdxk-gui.exe`，不用装 Rust / Flutter。
提示缺 `vcruntime140.dll` 的话，装一下 Microsoft Visual C++ 2015-2022 运行时（x64）。

## 两套入口，一个内核

| | 图形界面 | 命令行 |
| --- | --- | --- |
| 适合 | 一边看课程列表一边勾，盯着实时日志 | 塞进脚本、远程 / 无界面环境 |
| 启动 | 双击 `xdxk-gui.exe` | `xdxk-rs.exe <子命令>` |
| 代码 | `lib/`（Dart）+ `rust/`（flutter_rust_bridge） | `crates/xdxk-cli` |

两边共用 `crates/xdxk-core`，读写同一套 `conf.json`，所以配置可以互通。

> 1.0 之前的**纯命令行版本**留在 [`cli`](https://github.com/But-For-Love/XDUer_Loves_Scientific_Thought/tree/cli) 分支，
> 想用老版本直接 `git checkout cli`。

## 图形界面怎么用

五个页面：

| 页 | 干什么 |
| --- | --- |
| 概览 | 批次 / 课程行 / 任务 / 日志四个真实计数，当前批次状态，三步引导 |
| 登录 | 填学号密码；自动识别验证码（失败自动重试三次）或手动看图输入；登录后列出可选批次，点一行进入 |
| 课程 | 按类别拉当前批次的课程，按课程号 / 课名 / 课序号筛选，勾选后直接「选课」或「退课」；长按一行看原始 JSON |
| 任务 | 把 `KCH + KXH` 写成任务列表（选 / 退都行），失败自动重试、随时停止，可存进 `conf.json`，命令行也一样读 |
| 日志 | Rust 侧每一条请求、重试、结果都实时进来，出问题能直接看出是哪一步 |

配置默认就是 **exe 旁边的 `conf.json`**，整个文件夹拷走配置跟着走。想换位置：
「概览 → 配置文件」，可以「换一份…」「另存为…」「打开所在目录」「恢复默认」。
路径解析顺序：环境变量 `XDXK_CONF` → 记住的选择 → exe 同目录（不可写时回退到 `%APPDATA%\xdxk-gui`）。

## 命令行怎么用

### 快速开始

1. **配置**：复制 `conf.example.json` 为 `conf.json`，填入学号密码和课程
2. **编译**：`cargo build -p xdxk-cli --release`（产物在 `target/release/xdxk-rs.exe`；开发时可直接用 `cargo run -p xdxk-cli -- <命令>`）
3. **运行**：
   - 先核对配置是否匹配：`xdxk-rs.exe inspect`
   - 抢必修：`xdxk-rs.exe select -c 0`
   - 抢选修（只试一次）：`xdxk-rs.exe select -c 1 --once`
   - 退课：`xdxk-rs.exe drop -c 1`

### 配置 conf.json

只需改这几项：

| 字段 | 含义 |
|---|---|
| `ocr_captcha` | `"1"` 自动识别验证码（失败自动重试）；`"0"` 手动输入 |
| `batch` | 选课批次。填系统里该轮名字的一段即可（子串匹配），如 `"第二轮通识选修课程（2025级）"`；留空自动取第一个可选批次 |
| `action` | 不带子命令运行时生效：`add` 选课 / `del` 退课 / `query` 查询 |
| `always` | `1` 失败自动重试直到成功；`0` 只试一次 |
| `bx_or_xx` | `0` 必修；`1` 选修 |
| `bx` / `xx` | 必修 / 选修课程列表。每门填 `KCH`（课程号）+ `KXH`（课序号）；`KCM` 课程名只用于日志，可不填 |
| `loginname` | 你的学号  |
| `password` | 你的密码  |


> ⚠️ `KCH` 与 `KXH` 必须和系统里**完全一致**：区分大小写、保留前导零（`02 ≠ 2`）。不确定就先跑 `inspect` 核对。

**多任务（可选）**：想一次配多种类别/动作时用 `tasks`（优先级高于 `bx`/`xx`）：

```json
"tasks": [
  { "type": "bx", "action": "add", "KCH": "24HE0003", "KXH": "94" },
  { "type": "xx", "action": "del", "KCH": "24ZH2011", "KXH": "01" }
]
```

`type`：`bx`=必修、`xx`=选修；`action`：`add` / `del` / `query`。

### 命令一览

| 命令 | 作用 |
|---|---|
| `xdxk-rs.exe inspect` | 只读诊断：查课程在哪个类别、有哪些课序号（**不会选课**，建议先跑） |
| `xdxk-rs.exe select -c 0/1` | 抢必修(0) / 选修(1)，持续重试直到成功；加 `--once` 只试一次 |
| `xdxk-rs.exe drop -c 0/1` | 退课，用法同上 |
| `xdxk-rs.exe`（无子命令） | 按 `conf.json` 的 `action` 执行 |

持续性任务（选课、退课重试）按 `Ctrl+C` 可以安全退出。

## 从源码构建

前置：Rust 1.100+、Flutter 3.47+、Visual Studio「使用 C++ 的桌面开发」、`flutter_rust_bridge_codegen` 2.13。

```bash
flutter pub get
flutter run -d windows                 # 开发界面，第一次会连带编译 Rust
flutter test                           # 界面回归测试
cargo test --workspace                 # 内核与桥接层单测
powershell -File tool/package.ps1      # 出发布包（dist/）
```

改了 Rust 侧暴露给界面的函数，要重新生成绑定：

```bash
flutter_rust_bridge_codegen generate
```

## 目录结构

```
crates/
  xdxk-core/     业务内核：配置、HTTP 接口、验证码 ONNX、AES 加密
  xdxk-cli/      命令行入口 xdxk-rs.exe
rust/            flutter_rust_bridge 桥接层（cdylib），日志走 tracing → Stream
lib/             Flutter 界面：ef/ 是 endfield-ui 套件，src/ 是五个页面与状态
models/          验证码模型 common_lite.onnx
tool/            gen_icon.dart（出图标）、package.ps1（出发布包）
windows/         Flutter Windows runner
```

## 常见问题

- **`inspect` 里课程显示 `kxh=[]` 为空**：选修课没有单独教学班列表属正常；若连课程都找不到，说明学校接口结构变了，把日志贴出来排查
- **提示「未找到课程」**：核对 `conf.json` 中 `KCH` / `KXH` 是否与系统完全一致（大小写、前导零）
- **验证码总是失败**：自动识别失败会自动重试 3 次；仍不行就把 `ocr_captcha` 设为 `"0"` 手动输入
- **模型加载失败**：确认 `models/common_lite.onnx` 存在且完整（未被改动或下载截断）
- **界面里登录成功但没有课程**：先「进入批次」；个别类别（选修）没有单独教学班列表是正常的
- **界面执行时提示「课程列表里没有这个课程号 + 课序号」**：任务里的 `KXH` 和系统不一致，改完再跑
- **双击 exe 没反应**：装 Microsoft Visual C++ 2015-2022 运行时（x64）
- **换了图标但任务栏还是旧的**：Windows 会缓存图标，重启资源管理器或重新登录一次

## 开发

- 提交和 PR 会自动跑 CI：Rust 的 `fmt` / `test`、Flutter 的 `analyze` / `test` / `build windows`（`.github/workflows/ci.yml`）
- 推 `v*` tag 会自动出包并发 Release（`.github/workflows/release.yml`），tag 名要和 `pubspec.yaml` 的版本号一致
