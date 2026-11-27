**1.0.0：西电选课工具现在有界面了。**

Rust 内核 + Flutter 界面（终末地风格），用 [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) 直连，不起本地服务、不复制业务逻辑。
命令行版仍然保留，和界面共用同一个内核 `crates/xdxk-core`。

> ⚠️ 本项目不保证能用，选课规则以学校为准。**请先读 [README 顶部的警告](https://github.com/But-For-Love/XDUer_Loves_Scientific_Thought#readme)**：
> 短时间内抢多门课，后几门可能被学校反脚本机制作废。仅供学习交流，使用产生的一切后果由使用者自行承担。

## 下载

| 文件 | 说明 |
| --- | --- |
| `xdxk-gui-1.0.0-windows-x64.zip` | 解压即用，不用装 Rust / Flutter |
| `SHA256SUMS.txt` | 校验值（`certutil -hashfile <包名> SHA256` 对一下） |

解压后双击 `xdxk-gui.exe`。提示缺少 `vcruntime140.dll` 的话，装一下 Microsoft Visual C++ 2015-2022 运行时（x64）。

## 这一版新增

- **图形界面**：概览 / 登录 / 课程 / 任务 / 日志 五页；宽屏顶部命令栏，窄屏底部标签栏；浅色 / 深色跟随系统
- **登录**：本地 ONNX 模型自动识别验证码，失败自动重试三次；也可以关掉识别、手动看图输入
- **课程**：按类别拉当前批次的课程，按课程号 / 课名 / 课序号筛选，勾选后直接「选课」或「退课」，长按看原始 JSON
- **任务**：把 `KCH + KXH` 写成任务列表（选 / 退都行），失败自动重试直到成功，随时可以停
- **日志**：Rust 侧每一条请求、重试、结果实时进「日志」页，出问题能直接看出是哪一步
- **配置**：默认就是 exe 旁边的 `conf.json`，整个文件夹拷走配置跟着走；也能在「概览 → 配置文件」里换位置

## 没变的部分

- 命令行 `xdxk-rs.exe`：`inspect` / `select` / `drop`，行为和 0.1.2 一致
- `conf.json` 格式完全兼容，界面和命令行可以共用同一份
- 验证码模型 `models/common_lite.onnx` 仍随包分发

1.0 之前的纯命令行版本在 [`cli` 分支](https://github.com/But-For-Love/XDUer_Loves_Scientific_Thought/tree/cli)，`git checkout cli` 就能拿到。

## 从源码构建

```bash
flutter pub get
flutter run -d windows                 # 开发界面
flutter test && cargo test --workspace # 测试
powershell -File tool/package.ps1      # 出发布包
```

需要 Rust 1.100+、Flutter 3.47+、Visual Studio「使用 C++ 的桌面开发」。
