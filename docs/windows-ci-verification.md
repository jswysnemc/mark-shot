# Windows 原生构建工作流验证

2026-11-07 按用户授权提交并推送到 `rewrite/native-wayland`。没有创建标签或发布 Release。

## 已通过的远端构建

- 提交：`07e429f94bf414ca4ee2b98829bc7e0c78fc4bf7`
- 运行：https://github.com/jswysnemc/mark-shot/actions/runs/42383919619
- 环境：GitHub `windows-2022`，x86_64 MSVC，默认功能，CPU OCR
- 工作流：`.github/workflows/windows-build.yml`
- 通过格式、全目标严格 Clippy、原生单元测试、Release 编译、运行库收集、导入表验证、独立目录启动、压缩包解压及校验、构建产物上传
- 主库指定分组 64 项通过、2 项忽略；图像与拼接库 21 项通过。忽略项是已有原生桌面集成测试，不能视为实际 GUI 通过

相同提交的 Linux 工作流也完整成功：
https://github.com/jswysnemc/mark-shot/actions/runs/42383919620 。
包含工作区测试、全特性严格 Clippy、Python 检查、Release、链接检查、许可证和产物上传。

打包脚本按职责拆成 `windows-test.ps1`、`windows-package.ps1`、`windows-runtime.ps1`、
`windows-smoke.ps1`。压缩包附带 DirectML 与所需 MSVC 运行库，不依赖开发工具 PATH，
递归检查导入表且拒绝 Qt DLL。默认 OCR 仍使用 CPU，DirectML 是链接依赖。

## 可运行文件与摘要

下载目录：`target/windows-ci-07e429f-artifact/`。

| 文件 | SHA-256 |
| --- | --- |
| `mark-shot-native-windows-x86_64-msvc.zip` | `484c7d740e66536e7f9235fd8936d57fdcba01c2f7154836e1e1db1c989f5103` |
| 解压后的 `mark-shot.exe` | `50666db07cfb1a3a964ee8496d0125bd421dbd680c5838be715bd5e0f0082c0b` |

ZIP 为 26,900,191 字节。解压位置为
`target/windows-ci-07e429f-artifact/unpacked/mark-shot-native-windows-x86_64-msvc/`。
必须保留该目录内全部 DLL，不能只复制 EXE。包内 `SOURCE-COMMIT` 和 `SHA256SUMS`
提供源码关联及逐文件校验。产物是构建附件，不是公开 Release。

## 同一远端产物的实际回归

直接运行下载的 EXE，没有用本地构建替代。私有 Wine/Xvfb 三类流程通过：

- `target/windows-ci-final-basic-r1/workflow.json`：显示器发现、区域捕获精确像素、
  编辑器明暗恢复、首次 OCR 引导、画笔、保存与剪贴板一致、基本贴图复制
- `target/windows-ci-final-pin-r1/workflow.json`：菜单按下和释放事务、无损旋转、
  缩放保留原件、外部配置刷新、另存为取消与保存、编辑交接和窗口回收
- `target/windows-ci-final-ocr-r2/workflow.json`：空模型目录首次引导、正式地址下载三项模型并验证
  完整 SHA-256、下载后自动推理与独立结果交接、Unicode 剪贴板、撤销重做、原图页签、
  最小窗口及二次模型复用；独立结果窗口在外部原子保存主题后自动刷新，浅色标题栏精确恢复，正文不变

这些实际 GUI 证据仍属于 Wine，不代表真实 Windows 11 多显示器、输入法、帧率或内存验收。
OCR 首轮完整流程为同目录 `windows-ci-final-ocr-r1`；第二轮增加独立主题断言后重新下载、推理和编辑，
没有复用旧 R3 证据。MiSans VF 与 MiSans L3 仅安装在测试前缀，摘要记录于报告，未加入产品包。

## CI 中实际发现并修复的问题

1. Windows 检出模板包含 CRLF，旧配置迁移按 LF 查找失败；现在规范化模板换行，保留配置语义
2. 历史文件名及翻译盐值仍读取 `/dev/urandom`；Wine 掩盖了路径依赖，现改用系统随机接口
3. Windows 目录句柄路径失败曾静默返回空路径；现在打开时解析有效 DOS、卷 GUID 或 NT 路径，失败直接返回错误
4. Linux 对照字体版本不同；CI 下载并校验固定 Noto Sans 2.015，私有 Fontconfig 保留系统中文回退
5. 共享页测试原先依赖临时磁盘文件系统；现在使用与 Wayland 同类的匿名共享内存，保留原有像素与驻留页断言

后续 `07e429f` 修复 CPU 近似倒数导致的透明通道差异，以及 Inter 缺少六百字重时被其他字体回退替换。
本地工作区 1,367 项通过、19 项忽略，严格 Clippy 通过。该提交 Windows 远端构建与
下载产物的两组实际回归均已通过；此前 `ce48cb8` 证据保留在独立目录，不替代本次结果。

同一最新 Linux 候选 `target/determinism-candidate-r1/mark-shot` 的 SHA-256 为
`69f5ffd47c3e43bfe2b328960cb18734f9f00d8059cb5b2a9f89b7f97490ddb8`。
该文件重新通过字体完整流程及一点二五倍画布明暗、导航和透明导出流程，结果分别为
`target/final-font-r1/result.json` 与 `target/final-canvas-r1/result.json`；未复用旧构建结果。

## 未完成

用户实测长截图内存仍不如 Qt、普通截图帧率下降继续列为问题；历史 R20 固定场景数字不外推到用户场景。
图片编辑画布黑底已复现并修复，正常及一点二五倍输出、缩放视图与透明导出通过，见 `theme-compatibility.md`。
用户未附图的具体黑色区域仍未确认。Windows 完整平台目标、全场景性能及交互审计继续进行。
