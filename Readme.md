# 西电选课脚本（XDxk）

一个基于 `conf.json` 配置驱动的西安电子科技大学选课 / 退课自动化脚本，支持自动识别验证码。

> 本项目基于 [syityx/XDxk](https://github.com/syityx/XDxk) 改进而来。

> ⚠️ 仅供学习交流使用。请遵守学校选课规则，合理控制请求频率，勿滥用造成服务器压力或影响他人公平选课。使用本脚本产生的一切后果由使用者自行承担。

## 环境要求

- Python 3.8 及以上（开发环境为 3.14）
- 依赖：`requests`、`ddddocr`、`pycryptodome`

## 安装

```bash
pip install -i https://pypi.tuna.tsinghua.edu.cn/simple -r requirements.txt
```

## 配置

打开 `conf.json`，按需修改（可先复制 `conf.example.json` 为 `conf.json`，再填入你的学号密码）：

```json
{
  "ocr_captcha": "1",
  "debug": "0",
  "batch": "2020级",
  "action": "add",
  "always": 1,
  "bx_or_xx": 0,
  "bx": [
    { "KCH": "TE204004", "KXH": "06" },
    { "KCH": "TE204004", "KXH": "07" }
  ],
  "xx": [
    { "KCH": "FL006066" }
  ],
  "data": {
    "loginname": "你的学号",
    "password": "你的密码",
    "captcha": "xxxx",
    "uuid": "xxxx"
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `ocr_captcha` | string | `"1"` 自动识别验证码；`"0"` 手动输入 |
| `debug` | string | `"1"` 输出调试文件（`captcha_pac.json` / `login_pac.json` / `classlist.json`）；`"0"` 关闭 |
| `batch` | string | 选课批次名称（**子串匹配**）；留空 `""` 自动选择第一个可选批次 |
| `action` | string | `"add"` 选课；`"del"` 退课；`"query"` 只查询、不提交 |
| `always` | number | `1` 连续选/退课（一直尝试到成功或终止）；`0` 仅执行一次 |
| `tasks` | array | 可选。任务列表，非空时覆盖 `bx_or_xx` / `action` / `bx` / `xx`（见下文「任务列表」） |
| `bx_or_xx` | number | `0` 必修；`1` 选修 |
| `bx` | array | 必修课程，每项需 `KCH`（课程号）和 `KXH`（课序号，见小卡片左上角 `[01]`） |
| `xx` | array | 选修课程，每项只需 `KCH`（课程号） |
| `data` | object | `loginname` 学号；`password` 密码（可留空，运行时手动输入）；`captcha` / `uuid` 无需改动 |

### 注意事项

- 必修课必填课序号 `KXH`，选修课可不填。
- 不配置 `tasks` 时，单次运行只能选一种类别（必修 / 选修）、执行一种操作（选课 / 退课 / 查询）。如需同时完成「必修 + 选修」或「选课 + 退课」，请使用下文的「任务列表」。
- `batch` 是子串匹配：填 `"2020级"` 会匹配名称里包含「2020级」的批次；控制台会打印所有可选批次供核对。
- 将 `action` 设为 `"query"` 可进入只查询模式：只打印匹配到的课程，不真正选课 / 退课。
- 安全提示：`conf.json` 以明文保存密码，且 `encrypt.py` 密钥硬编码、使用 ECB 模式（属前端可逆加密），请勿将 `conf.json` 提交到公开仓库。

### 高级：任务列表

在 `conf.json` 中配置可选的 `tasks` 字段，可以一次完成多种类别、多种操作的组合（例如同时必修选课 + 选修退课）。当 `tasks` 非空时，会忽略 `bx_or_xx` / `action` / `bx` / `xx`。

```json
"tasks": [
  { "type": "bx", "action": "add", "KCH": "TE204004", "KXH": "06" },
  { "type": "xx", "action": "del", "KCH": "FL006066" }
]
```

字段：`type`（`"bx"` 必修 / `"xx"` 选修，也可用 `0` / `1`）、`action`（`add` / `del` / `query`）、`KCH`（课程号）、`KXH`（课序号，仅必修必填）。

## 运行

配置好 `conf.json` 后：

```bash
python xk_main.py
```

也可直接用任意 IDE 运行 `xk_main.py`。

程序流程：

1. 登录（验证码自动识别或手动输入）；
2. 打印姓名、专业、班级与可选批次；
3. 按 `batch` 选中批次；
4. 拉取课程列表，按课程号（必修再按课序号）匹配；
5. 按 `action` 执行选课或退课。

## 常见问题

| 现象 | 处理 |
|------|------|
| 提示「验证码错误」 | 重新运行；或将 `ocr_captcha` 改为 `"0"` 手动识别（不建议） |
| 手动验证码 | 脚本会把验证码保存为 `captcha.png`，打开查看后回到控制台输入验证码 |
| 提示未找到可用批次 | 核对 `batch` 是否写对，或留空自动选择；控制台会打印所有可选批次 |
| 忘记在配置里填学号密码 | 运行时控制台会提示手动输入 |

## 文件结构

```
XDxk/
├── xk_main.py       # 入口：读取配置并执行选课/退课
├── func.py          # 登录、验证码、批次选择、选课/退课逻辑
├── encrypt.py       # 密码 AES 加密
├── conf.json        # 配置文件
├── requirements.txt # 依赖
└── Readme.md
```
