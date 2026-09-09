# Rust 迁移：OCR 方案分析与结论（Agent 指南）

> 目标：把原 Python 项目 `XDxk/`（西安电子科技大学选课 / 退课脚本）迁移为 Rust + tokio 项目（本目录 `xdxk-rs/`）。
> 本文档给出**验证码 OCR 的技术选型结论**，并附迁移必须遵守的约束与验收标准。原 Python 目录保持不变。

## 一、背景

原项目用 Python 的 `ddddocr` 识别选课系统登录验证码：

```python
ocr = ddddocr.DdddOcr()
code = ocr.classification(img_bytes)  # img_bytes 是 PNG 字节
```

- 验证码图片来自登录接口返回的 `data:image/png;base64,...`，base64 解码后就是 PNG 字节。
- 另有「手动模式」：把验证码存成图片文件，让用户自己看、自己输入。

## 二、Rust OCR 生态调研

| 方案 | 类型 | 说明 | 适合本项目？ |
|------|------|------|------------|
| [`ddddocr` crate](https://docs.rs/ddddocr) | ddddocr 的 Rust 移植 | crates.io 上的 `ddddocr`（当前 0.1.0），提供 `DdddOcr::classification()`，专为验证码设计 | ✅ 首选 |
| [`ddddocr-rs` (CNWeiWei)](https://github.com/CNWeiWei/ddddocr-rs) | ddddocr 的 Rust 移植 | 高性能、低占用，支持验证码识别与检测 | ✅ 备选 |
| [`ddddocr-rs` (mzdk100)](https://github.com/mzdk100/ddddocr-rs) | ddddocr 的 Rust 实现 | 另一个独立实现，可交叉参考 | ✅ 备选 |
| [`ort`](https://crates.io/crates/ort) | onnxruntime 绑定 | 直接加载 ddddocr 的 `common.onnx` / `common_old.onnx` 自行推理 | ✅ 兜底 |
| `leptess` / `tesseract` | Tesseract C++ 绑定 | 通用 OCR，需系统库；对扭曲验证码效果差 | ❌ |
| [`ocrs`](https://crates.io/crates/ocrs) | 纯 Rust 通用 OCR | 面向自然场景文字，不针对验证码 | ❌ |
| `tch-rs` / `candle` | ML 框架 | 可跑模型但偏重，需自备模型 | ⚠️ 不推荐 |
| `captchaforge` / `captcha-engine` | 验证码识别库 | 小众，生态 / 文档弱 | ⚠️ 可调研 |

## 三、结论（最适合本项目）

**主方案：使用 `ddddocr` Rust crate（或 `ddddocr-rs`）。**

理由：

1. 与 Python 版 `ddddocr` 同源 / 同目标，专门针对扭曲验证码，准确率最接近原版。
2. API 简单（`classification(bytes) -> String`），迁移成本最低。
3. 省去自己复刻图像预处理 + onnx 推理的复杂工作。

**必须保留的兜底：手动模式（保存 `captcha.png` 让用户输入）。**

- 零 OCR 依赖、最稳，且原 Python 版已有此模式，语义可完全对齐。
- 建议把 OCR 做成可插拔（由配置 `ocr_captcha` 控制：`"1"` 自动 / `"0"` 手动），手动模式始终可用。

**若 crate 不成熟 / 效果不符的 Plan B：`ort` + ddddocr 的 onnx 模型。**

- 从原 Python 环境复制模型：`.venv/Lib/site-packages/ddddocr/common.onnx`、`common_old.onnx`（检测用还有 `common_det.onnx`、`logo.png`）。
- 用 `ort` 加载模型，复刻 ddddocr 的预处理（灰度、resize 到模型输入尺寸、归一化）与后处理（分类头 argmax）。
- 最忠实但工作量最大，作为兜底。

**不推荐**：tesseract / leptess、ocrs（对扭曲中文验证码效果差；tesseract 还需系统库）。

## 四、风险与注意事项

1. `ddddocr` crate 当前是 0.1.0，成熟度未知——接入后必须用真实验证码验证准确率；不达标就退回 `ort` 方案或手动模式。
2. 模型文件：若 crate 不自带模型，需确认其是否内嵌模型，或把 ddddocr 的 onnx 模型随项目分发（注意体积与许可）。
3. 建议开发顺序：先用手动模式跑通整条链路，再替换为自动 OCR，降低风险。

## 五、迁移硬约束（务必遵守）

1. **不改任何请求行为**：URL / 方法 / query / body / json / headers / cookies 与 Python 版逐字节一致；`add` 连续重试循环保留 1 秒间隔；`delete` 循环不加 sleep；不引入并发；不加超时 / 退避；不改分页（pageSize=300）。
2. **不使用任何环境变量**。
3. **AES 输出逐字节一致**：密钥 `MWMqg2tPcDkxcm11`，AES-128-ECB + PKCS7 + base64。

### AES 已知向量（回归测试）

```
"zyx/020305"        -> "5ZTBUxmD+OY7LL1nzUUz+g=="
"123456"            -> "OSfRhnd673K1Lp6cP4L6nA=="
""                  -> "mkpyTarWC0ro2N4QUBrjAQ=="
"a"                 -> "hj0T3YA9PCHq7PAsPke1mQ=="
"password12345678"  -> "bLI9j6Y3UbBljQs6YgYfnJpKck2q1gtK6NjeEFAa4wE="
"中文密码abc"         -> "0sLEDhp4IRDqt9Y0z1Flvw=="
```

## 六、关键请求细节（迁移时逐条核对）

1. 验证码：`POST https://xk.xidian.edu.cn/xsxk/auth/captcha`（无参数）；响应 `data.captcha` 是以 `data:image/png;base64,` 开头的 data URI，`data.uuid` 是验证码标识。
2. 登录：`POST https://xk.xidian.edu.cn/xsxk/auth/login`，表单字段走 **URL 查询串（query，不是 body）**：`loginname`、`password`（AES 加密后）、`captcha`、`uuid`；headers：`Connection: keep-alive`、`User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.5060.66 Safari/537.36 Edg/103.0.1264.44`（值必须完全一致）。
3. 批次：从登录响应 `data.student.electiveBatchList` 按 `batch` 子串匹配 `name` 取 `code`；仅 `canSelect == "1"` 可选；`batch` 为空则自动选第一个可选批次。
4. 课程列表：`POST https://xk.xidian.edu.cn/xsxk/elective/clazz/list`，**JSON body**：`{"teachingClassType":"FANKC"|"XGKC","pageNumber":1,"pageSize":300,"orderBy":"","campus":"S"}`；headers 额外带 `batchId`、`Authorization: <token>`。
5. 选课：`POST https://xk.xidian.edu.cn/xsxk/elective/clazz/add`，**查询串**：`clazzType=FANKC|XGKC`、`clazzId=<JXBID>`、`secretVal`、`chooseVolunteer=1`；headers 带 `batchId`、`Authorization`；cookies 带 `Authorization`。连续模式循环直到 `msg` 等于「该课程已在选课结果中」或「所选课程与已选课程冲突」，每轮 `tokio::time::sleep(1s)`。
6. 退课：`POST https://xk.xidian.edu.cn/xsxk/elective/clazz/del`，**查询串**：必修 `clazzType=TJKC`（无 chooseVolunteer）、选修 `clazzType=XGKC`（有 `chooseVolunteer=1`）；循环直到 `msg` 等于「所选课程与已选课程冲突」或「操作成功」，**不加 sleep**。
7. token 来自登录响应 `data.token`。

## 七、依赖与模块建议

- 依赖：`tokio`(full)、`reqwest`(json, cookies, rustls-tls)、`serde` + `serde_json`、`aes`、`base64`、`tracing` + `tracing-subscriber`、`ddddocr`（或 `ort`）。
- 模块：`main.rs`（入口）、`config.rs`（配置 + 校验）、`client.rs`（请求封装）、`encrypt.rs`（AES）、`captcha.rs`（验证码）。

## 八、验收标准

- `cargo build`、`cargo test` 通过。
- AES 已知向量测试通过（见上文）。
- 请求构造测试：每个请求的 method / URL / query / body / headers 与 Python 版一致。
- 手动模式可用；自动 OCR（ddddocr crate）可用，若 crate 不达标可暂缓并标注。
- `conf.example.json` 兼容。

## 九、参考来源

- ddddocr crate: https://docs.rs/ddddocr
- ddddocr-rs (CNWeiWei): https://github.com/CNWeiWei/ddddocr-rs
- ddddocr-rs (mzdk100): https://github.com/mzdk100/ddddocr-rs
- ort: https://crates.io/crates/ort
- ocrs: https://crates.io/crates/ocrs

---

## 十、实际实现结果（已落地）

- **最终方案**：未使用 `ddddocr` crate（其 0.1.0 的 `classification` 有 bug：直接 `try_extract_tensor::<i64>()`，但 `common_old.onnx` 输出的是 `(seq, 1, 8210)` 的 float32 logits，缺少 argmax 步骤）。
- **改为用 `ort` 直接复刻**：预处理（等高缩放到 64、灰度、归一化）→ 推理（输入 `input1` `[1,1,64,w]`）→ 输出 argmax → CTC 解码（去连续重复 + 去 blank）→ 8210 字符集映射。
- 模型：`models/common.onnx`（beta 模型，从 Python ddddocr 包 `common.onnx` 复制，54 MB，已加入 .gitignore）。
- 字符集：`charset.json`（beta 字符集，从 Python ddddocr `_get_beta_charset` 提取，8210 项）。
- 已验证：生成「1234」文字图片，识别结果正确返回 `"1234"`。
