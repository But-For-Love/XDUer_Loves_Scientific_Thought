西安电子科技大学选课工具的桌面界面：**Rust 内核 + Flutter 界面**，用 [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) 直连，不起本地服务、不复制业务逻辑。界面风格来自 [endfield-ui](https://github.com/hhikr/endfield-ui-skill)。

## 下载

| 文件 | 说明 |
| --- | --- |
| `xdxk-gui-1.0.0-windows-x64.zip` | 解压即用（26.9 MB），不用装 Rust / Flutter |
| `SHA256SUMS.txt` | 校验值 |

解压后双击 `xdxk-gui.exe`。如果提示缺少 `vcruntime140.dll`，装一下 Microsoft Visual C++ 2015-2022 运行时（x64）。

```
SHA256  3DA61759347C828A6B3070D8850D8AC7B1AB4CE3B195E9C578F0010EEAF3BD0D
```

## 能做什么

- **登录**：学号密码 + 验证码。自动模式走本地 ONNX 模型（`models/common_lite.onnx`），识别失败自动重试三次；也可以关掉自动识别，手动看图输入。
- **课程**：按类别拉取当前批次的课程，按课程号 / 课名 / 课序号筛选，勾选后直接「选课」或「退课」；长按一行看原始 JSON。
- **任务**：把 `KCH + KXH` 写成任务列表，支持选 / 退两种动作；失败自动重试直到成功，随时可以「停止」；可以存进配置，命令行版也能读。
- **日志**：Rust 侧每一条请求、重试、结果都实时进「日志」页，出问题能直接定位是哪一步。
- **配置**：默认就是 exe 旁边的 `conf.json`，整个文件夹拷走配置跟着走；也可以在「概览 → 配置文件」里换位置、另存、恢复默认。

## 界面

概览（真实计数 + 批次状态）、登录、课程、任务、日志五页；宽屏顶部深色命令栏，窄屏底部标签栏；浅色 / 深色两套主题跟着系统。

## 从源码构建

```bash
flutter pub get
flutter run -d windows                      # 开发
powershell -File tool/package.ps1           # 出发布包
```

需要 Rust 1.100+、Flutter 3.47+、Visual Studio「使用 C++ 的桌面开发」。

## 说明

- `crates/xdxk-core` 是唯一的业务实现，命令行版 `crates/xdxk-cli` 和界面共用它。
- 模型与选课逻辑沿用[原项目](https://github.com/But-For-Love/XDUer_Loves_Scientific_Thought)，界面与桥接层是本仓库新增的。
- ⚠️ 仅供学习交流。选课规则以学校为准，请求频率自己控制，使用产生的一切后果由使用者自行承担。
