# 工程交接与易错点

## 实时曲线显隐（2026-11-10）

- 曲线数据的存在与是否显示是独立状态：隐藏只影响绘制，不停止接收数据。
- `PlotState::push_line` 只能在新建曲线时设置默认显示，不能在每个数据点到达时重新写入 `visible`，否则会覆盖用户取消勾选的选择。
- 不要用数据队列是否为空判断是否是新曲线；应使用 `series.entry` 的插入路径初始化显示状态。手动清除后重新创建的曲线仍默认显示。
- 清空全部数据、切换稳定数据格式及导入历史文件沿用已有的曲线重建行为。
- 实时更新路径不增加额外扫描或显隐状态副本；只在创建曲线时更新显示集合，保留原有历史点数上限。
- 回归验证覆盖 CSV 和 key=value 数据持续追加、隐藏全部曲线后重新勾选，以及清除单条曲线后重新接收。
- 验证命令：`cargo fmt --check`、`cargo test --locked`、`cargo clippy --all-targets --all-features --locked`。当前工具链会对既有 UI 代码报告 `float_literal_f32_fallback` 告警；`src/update.rs` 还存在两处既有的 `useless_format` 告警，避免在绘图状态修复中混入无关清理。

## v0.2.17 发布阻塞（2026-11-10）

- 修复提交为 `9316b94`，发布准备提交为 `85bdd22`；后者已推送到 `main` 并打上远端标签 `v0.2.17`。本地格式检查、57 项测试及 Clippy 已完成。
- 发布任务：<https://github.com/Nitmi/serial-scope/actions/runs/42789773383>。校验任务没有执行任何步骤，GitHub 注解明确说明：`The job was not started because your account is locked due to a billing issue.`
- 发布及上传步骤均跳过，最新正式版仍为 `v0.2.16`。本机仅具备 Windows 桌面目标，没有可用的 Linux/macOS 交叉编译工具，无法替代规范要求的三平台发布。
- 账户账单锁定解除后，运行 `gh run rerun 42789773383` 重跑原标签的发布任务，再跟进到三平台构建、发布和缓存刷新完成。无需重建或移动 `v0.2.17` 标签。
- 恢复后按 `docs/release-checklist.md` 验证五个发布资产、中文更新说明、`latest.json` 的版本与主代理优先顺序，以及 GitHub 和主代理的更新端点。
