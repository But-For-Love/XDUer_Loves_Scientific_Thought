# xdxk-rs 操作说明

西安电子科技大学选课脚本的 Rust 版。自动识别验证码、自动登录，支持**选课 / 退课 / 查询 / 容量检查 / 捡漏 / 诊断**。识别用的模型文件 models/common_lite.onnx 已随项目提供，无需额外准备。

## 一、上手三步

1. 写配置文件 conf.json（见「二」）
2. 编译（见「三」）
3. 运行命令（见「三」「四」）

## 二、配置 conf.json

复制 conf.example.json 为 conf.json，改下面几处（// 是注释，实际文件里不要写）：

    {
      "ocr_captcha": "1",          // "1"=自动识别验证码，"0"=手动输入
      "batch": "2020级",            // 选课批次（见下方字段说明）
      "interval_ms": 0,             // 轮询间隔毫秒，0=默认
      "always": 1,                  // 1=失败自动重试，0=只试一次
      "bx_or_xx": 0,                // 0=必修，1=选修
      "bx": [ { "KCH": "TE204003", "KXH": "02", "KCM": "大学语文" } ],
      "xx": [ { "KCH": "FL006066", "KXH": "01", "KCM": "英语选修" } ],
      "data": { "loginname": "你的学号", "password": "你的密码" }
    }

字段说明：

| 字段 | 含义 |
|---|---|
| ocr_captcha | "1" 自动识别验证码（识别失败或验证码错误会自动重试，最多 3 次）；"0" 手动输入 |
| batch | 选课批次。填系统里那一轮名字中的一段即可（子串匹配），比如 "2025级" 或 "第一轮方案内课程"；留空则取第一个可选批次 |
| action | add 选课 / del 退课 / query 查询 / check 容量检查 / snipe 捡漏。只在「不带子命令运行」时生效 |
| interval_ms | 检查/捡漏的轮询间隔，毫秒；0=默认（容量检查 500ms，捡漏 5 秒） |
| always | 1=失败自动重试，0=只试一次 |
| bx_or_xx | 0=必修，1=选修（用 bx/xx 列表时生效） |
| bx / xx | 课程列表。KCH（课程号）和 KXH（课序号）都要填；KCM（课程名）只用于日志显示，可不填 |
| data.loginname / password | 学号 / 密码 |

**关键：KCH 和 KXH 是严格精确匹配**——区分大小写，02 和 2 算不同（只忽略首尾空格）。填错任何一个都会匹配不到课程。

### 高级：tasks 任务列表

想一次配多种动作/多类别时，用 tasks（优先级高于 bx/xx）：

    "tasks": [
      { "type": "bx", "action": "add", "KCH": "TE204003", "KXH": "02" },
      { "type": "xx", "action": "del", "KCH": "FL006066", "KXH": "01" }
    ]

type 取 bx / xx / 0 / 1；action 取 add / del / query。

## 三、编译与命令

先编译出可执行文件（开发时也可以直接用 cargo run -- 命令 代替）：

    cargo build --release

产物在 target\release\xdxk-rs.exe。

命令一览：

| 命令 | 作用 |
|---|---|
| xdxk-rs.exe inspect | 只读诊断：查课程在哪个类别、KXH 有哪些（不会选课，建议先跑这个核对配置） |
| xdxk-rs.exe select -c 0 | 抢必修（连续重试直到成功） |
| xdxk-rs.exe select -c 1 --once | 抢选修，只试一次 |
| xdxk-rs.exe drop -c 1 | 退选修 |
| xdxk-rs.exe check | 容量检查：盯住配置里的课，有空位就抢 |
| xdxk-rs.exe check --snipe | 捡漏：扫描全部课程，见空位就抢 |
| xdxk-rs.exe | 不带子命令，按 conf.json 的 action 执行 |

参数：-c 0 = 必修，-c 1 = 选修；--once = 只试一次。

## 四、常见场景

1. 第一次用，先核对配置对不对：

    xdxk-rs.exe inspect

2. 抢必修课（持续重试直到成功）：

    xdxk-rs.exe select -c 0

3. 抢选修课，只试一次：

    xdxk-rs.exe select -c 1 --once

4. 退课：

    xdxk-rs.exe drop -c 1

5. 蹲一门课的空位（容量检查）：把要蹲的课填进 conf.json 的 bx/xx，然后：

    xdxk-rs.exe check

6. 全场捡漏（扫所有课等空位）：

    xdxk-rs.exe check --snipe

## 五、重要说明

- KCH + KXH 严格匹配：区分大小写，02 ≠ 2，填错匹配不到课程。
- 验证码固定 4 位字母数字。自动识别失败会重试（最多 3 次）；想手动输入就把 ocr_captcha 设为 "0"。
- 持续任务（选课重试 / 容量检查 / 捡漏）按 Ctrl+C 可安全退出。
- 请求节奏与原版一致，没做频控退避；选课高峰服务器可能限流，属正常现象。
- 登录、选课接口依赖学校系统，网站维护或改规则时可能失效。

## 六、常见问题

**Q：inspect 里课程显示 kxh=[] 是空的？**
选修课没有教学班列表，KXH 在课程本身。若显示为空，说明服务器返回结构变了，把日志贴出来排查。

**Q：报「未找到课程」？**
检查 conf.json 里 KCH / KXH 是否和系统里完全一致（大小写、前导零都要一致）。

**Q：登录报验证码错误？**
识别有一定失败率，会自动重试；连续失败可把 ocr_captcha 设为 "0" 手动输入。

**Q：报模型加载失败 / 找不到 common_lite.onnx？**
模型随项目提供，确认 models/common_lite.onnx 存在且完整（未被删改或下载不完整）。
