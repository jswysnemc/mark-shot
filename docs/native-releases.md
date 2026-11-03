# 原生预览版发布

原生重写尚未完成，只发布 `native-vX.Y.Z-preview.N` 预览版，不覆盖 Qt 稳定版。
预览版仅提供 Linux Wayland x86_64 归档，既不更新 Latest，也不进入 AUR、PPA、
AppImage、Flatpak 等原版发布渠道。

## 固定产物

先等待 `.github/workflows/native-build.yml` 成功，再下载该任务的产物。校验归档摘要、
`SOURCE-COMMIT`、可执行文件摘要、帮助输出、动态链接和许可。私有 niri、Sway 会话
使用下载的可执行文件重复验证，不能借用本地编译或其他指纹的界面证据。

`release/native-preview.json` 固定源提交、任务编号、产物编号、归档和二进制摘要。
`release/native-preview-verification.json` 记录同一指纹的界面场景，以及仍未完成的范围。
发布清单同时固定验证记录和发布说明摘要，拒绝缺项、未知字段和不同构建的证据。
第二版使用 `release/native-preview-2.json`、`release/native-preview-2-verification.json`
及 `release/native-preview-2-notes.md`，保留第一版文件不变。第二版还要求 niri、Sway
两组独立窗口原生输入法证据。公开证据资产仍统一命名为 `native-preview-verification.json`。

## 发布隔离

提交并推送发布清单后，`Native Preview Release` 校验清单与任务来源，直接复用已验证
产物，不重新编译发布二进制。发布先创建草稿，上传并重新下载核验全部资产，然后公开。
网络失败留下的草稿可以重试；已公开版本只进行摘要核验，禁止覆盖资产或移动标签。

发布权限仅授予本仓库原生分支的任务。发布使用 GitHub Actions 自带的 `GITHUB_TOKEN`，
不使用个人令牌；由该令牌产生的 release 与 push 事件不会启动其他工作流。因此原版
监听 `release: published` 的稳定渠道不受本次预览发布影响。规则依据：
https://docs.github.com/en/actions/concepts/security/github_token

发布后还需从公开地址下载归档，核对清单与标签，确认稳定版 Latest 未变化，并确认
没有启动原版渠道任务。完整重写目标与预览版发布是两个独立状态，发版不代表重写完成。
