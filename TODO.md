# 开发待办 / 已知问题（TODO）

> 本文档面向后续维护者或 AI agent。当前各项改进已完成，保留范围约束供后续参考。
> 最后更新：2026-08-26

## 范围约束（务必遵守）

进行任何改动前，请遵守以下两条硬性约束，不要越界：

1. **不做任何涉及「环境变量」的改动**：例如不要新增「从环境变量读取密码 / 密钥」之类的方案。

2. **不做任何涉及「请求行为」的改动**：
   - 不新增 / 修改 `timeout`、`retry` 与退避；
   - 不改变 `requests.post(...)` 的 `params` / `data` / `json` / `headers` 参数；
   - 不调整请求频率（`time.sleep`）、不做并发；
   - 不修改分页参数（如 `pageSize`）。
   即：所有 HTTP 请求「发什么、怎么发、多久发一次」保持不变。

## 完成情况

- [x] 代码清理：删除死代码、清理注释、清理 `encrypt.py`。
- [x] 可维护性：显式导入、命名统一（`delete` / `aes_encrypt` / `pkcs7_padding`）、参数语义化。
- [x] 类型标注：`func.py` / `xk_main.py` / `encrypt.py` 全部函数已加类型标注，并补齐 docstring。
- [x] 安全：`.gitignore`、`conf.example.json`、debug 脱敏、README 风险提示。
- [x] 功能：配置校验、任务列表（`tasks`）、dry-run（`query`）、logging。
- [x] 性能：ddddocr 单例。
- [x] 测试：`test_xdxxk.py` 14 个测试（encrypt / show_msg / _build_tasks / 请求构造 mock）。
- [x] 工程化：`pyproject.toml`（ruff + mypy 配置）、`.github/workflows/ci.yml`（CI：lint + typecheck + 测试）。

## 备注

- ruff / mypy 未在本地运行：当前环境无法访问 PyPI 镜像安装它们；配置已就绪，推送到 GitHub 后由 CI 自动执行。
- 请求构造由 `TestRequestConstruction` 的 mock 测试锁定，改动请求前请先确认不违反上方范围约束。
