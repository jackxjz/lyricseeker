# 提交规范

本仓库从 2026-10-23 起使用 Git 管理。**每一次功能/修复改动都必须单独提交一次**，
不要把多个不相关的改动混在一个 commit 里，也不要攒着不提交。

## 提交信息格式

```
<type>: <一句话说明>

<可选的正文：为什么改、影响面>
```

`type` 取值：

| type | 用途 |
| --- | --- |
| `feat` | 新功能 |
| `fix` | 修 bug |
| `refactor` | 重构（行为不变） |
| `perf` | 性能优化 |
| `style` | 样式/格式（不影响逻辑） |
| `docs` | 文档 |
| `test` | 测试 |
| `chore` | 构建、依赖、脚本等杂项 |

## 提交流程

```powershell
cd "E:\Code 编程\DeepSeek Harness\LyricSeeker"

git status --short          # 确认改动范围
git add -A
git commit -F <说明文件>     # 见下方「中文提交信息」
```

### 中文提交信息

PowerShell 5.1 下用 `git commit -m "中文"` 会因为控制台代码页（GBK）把中文写坏，
表现为提交信息乱码或命令解析失败。**统一走文件**：

```powershell
# 1. 用 write 工具或记事本写一个 UTF-8 无 BOM 的说明文件，比如 %TEMP%\msg.txt
# 2. 提交
git -c i18n.commitEncoding=utf-8 commit -F "$env:TEMP\msg.txt"
# 3. 删掉临时文件
Remove-Item "$env:TEMP\msg.txt"
```

不要用 heredoc（`<<'EOF'`）—— PowerShell 不支持，会报
`缺少终止符，或文件说明符缺失`。

## 提交前自检

- [ ] `npm run build` 通过（前端类型 + 打包）
- [ ] 改到 Rust 时：`cd src-tauri; cargo test` 通过
- [ ] 改到上游接口时：`cargo test --test live -- --ignored --test-threads=1` 通过
- [ ] `git status --short` 只剩预期的改动，没有 `target/`、`node_modules/`、`dist/` 混进来
