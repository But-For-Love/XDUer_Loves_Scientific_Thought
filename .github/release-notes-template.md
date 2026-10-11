# __TAG__ — 西电选课 / 退课工具（Windows x64）

西安电子科技大学选课 / 退课脚本，解压即用，无需安装 Python 或 ONNX Runtime。

## 下载

| 文件 | 说明 |
|---|---|
| `__ZIP_NAME__` | 解压即用（约 __ZIP_SIZE__ MB） |
| `SHA256SUMS.txt` | 校验值 |

解压后目录：

```
xdxk-rs-__TAG__-windows-x64/
├─ xdxk-rs.exe            # 主程序
├─ conf.example.json      # 配置模板
├─ models/
│  └─ common_lite.onnx    # 验证码识别模型（ddddocr common_old，8210 类）
└─ README.md              # 完整文档
```

## 功能

- **自动登录**：复刻网页端加密逻辑（RSA + AES）
- **验证码自动识别**：本地 ONNX 模型推理，识别失败自动重试 3 次；也可切换手动输入
- **三个子命令**：`select` 抢课、`drop` 退课、`inspect` 只读诊断
- **持续重试**：失败自动重试直到成功，`Ctrl+C` 可安全退出
- **多任务**：`conf.json` 的 `tasks` 可一次配置多种类别/动作

## 快速开始

1. 解压；把 `conf.example.json` 复制为 `conf.json`，填入学号、密码、课程（`KCH` + `KXH`）
2. 先核对课程信息（不会选课，可放心跑）：`xdxk-rs.exe inspect`
3. 开始抢课：
   - 必修：`xdxk-rs.exe select -c 0`
   - 选修：`xdxk-rs.exe select -c 1`
   - 只试一次：加 `--once`
   - 退课：`xdxk-rs.exe drop -c 1`

详细配置与常见问题见压缩包内 `README.md`。

## 环境要求

- Windows 10 / 11 x64
- Microsoft Visual C++ 运行库 2015–2022 x64（绝大多数系统已预装，缺失时装上即可）
- 无需 Python、无需单独安装 ONNX Runtime：运行时已静态链接进 `xdxk-rs.exe`，模型随包提供
- 程序在**当前工作目录**读取 `conf.json` 与 `models/common_lite.onnx`，请在解压出的目录里运行

## 已知问题 / 注意事项

- ⚠️ **可能抢课失败**：学校疑似有反脚本措施，短时间内抢多门课会出现「日志显示成功、实际只选上部分」的情况，抢完请登录教务系统确认（详见 README 顶部提醒）
- `KCH` / `KXH` 必须与系统**完全一致**：区分大小写、保留前导零（`02 ≠ 2`）
- `inspect` 中选修课 `kxh=[]` 为空属正常（选修课无独立教学班列表）

## 文件校验

```
SHA256 (__ZIP_NAME__)
__SHA256_ZIP__

SHA256 (xdxk-rs.exe)
__SHA256_EXE__
```

## 免责声明

仅供学习交流。请遵守学校选课规则、控制请求频率。使用产生的一切后果由使用者自行承担。

---

构建日期：__DATE__
