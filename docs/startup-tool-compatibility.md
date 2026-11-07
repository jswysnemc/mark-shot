# 文件默认工具与原版启动别名

本次补齐 `--file-default-tool`，并接受原版 `--all-output`、`--full-screen`。文件工具
只影响打开已有图片后的初始标注，屏幕截图继续使用普通和全屏工具设置。

## 原版顺序

依据 `../mark-shot/qt/src/main.cpp:284`：

1. `--default-tool` 为普通、全屏和文件入口提供共同默认值
2. `--fullscreen-default-tool` 覆盖全屏和文件入口
3. `--file-default-tool` 最后只覆盖文件入口

原生实现由 `src/app/startup_tool.rs` 统一选择全屏或文件初始工具。
`src/cli/arguments.rs` 解析文件选项和两个范围别名；同时提供新旧范围拼写时全部消费，
仍拒绝全部输出与 `--monitor` 的冲突。非法工具在参数阶段返回错误。

```sh
mark-shot --default-tool pen --fullscreen-default-tool ruler --file-default-tool rectangle picture.png
mark-shot --full-screen --all-output --fullscreen-default-tool pen
```

## 固定候选与验证

候选为 `target/startup-cli-candidate-r1/mark-shot`，大小 49,447,512 字节，SHA-256：

```text
78324cfa8c725493693d98f5f934ffb7d7129c313624283a92bca68593ed4492
```

1,062 个构建输入前后相同。新增六项启动测试覆盖参数、错误、重复别名及实际工具优先级；
全特性工作区 1,293 项通过、19 项忽略，严格 Clippy、格式和相关脚本 Ruff F 类检查通过。
三个参数回归在修复前失败，日志保留为
`target/startup-cli-verification-r1/before-cli-tests.log`。

同一候选在私有 Sway/niri 的一点五倍单输出各验证三例，不发送工具切换快捷键：

- 文件矩形选项覆盖全屏尺子和普通画笔，实际导出矩形
- 未提供文件选项时继承全屏矩形，实际导出矩形
- 屏幕全屏入口忽略文件矩形选项，实际导出画笔；此场景使用两个原版范围别名

两种文件场景都保留 256×160 原图尺寸，像素检查区分矩形边缘与画笔连续笔迹。
保存的 60 个空闲样本均为单线程，同组内 CPU、提交数和描述符数稳定，RSS 为
55,508–66,916 KiB。证据为 `target/startup-cli-verification-r1/result.json`。

本阶段未改写发布路径；核对时其指纹保持此前单实例构建的 `4852decc…c24c53`。
此前单实例与区域矩阵分别绑定该构建，见
[单实例对照](single-instance-compatibility.md)，不把此前矩阵替换为本次启动工具结果。
本次原生检查确认参数接入、直接绘制及原图尺寸，不声称完成全部 Qt 光栅对照或冷启动审计。
