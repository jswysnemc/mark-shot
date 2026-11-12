# Qt 历史问题在 Rust 版本的核对

日期：2026-11-11。分支：`feat/native-barcode-scanning`。基线：`43534bd`。
GitHub 正文与评论原始记录位于 `target/legacy-issue-evidence/<issue>.json`。
本次没有修改、评论或关闭远程问题。

| 问题 | Rust 核对结果 | 修改和证据 |
| --- | --- | --- |
| [#156](https://github.com/jswysnemc/mark-shot/issues/156) 全屏选区工具栏越界 | 三屏全选未复现原问题，但发现同类边界缺陷 | 副栏原来使用主栏显示器尺寸排版，手动位置不随新尺寸收敛。现按两栏各自显示器换行并限制位置；三屏负原点、竖屏、异尺寸输出测试通过 |
| [#153](https://github.com/jswysnemc/mark-shot/issues/153) 快捷键修改需重启 | X11 未复现 | 私有 Xvfb 的 14 项桌面集成检查通过，包括 F8 改为 Ctrl+F9、旧键释放、新键立即生效、冲突恢复及禁用。Windows 原生注册路径已审阅，未在 Windows 实测 |
| [#148](https://github.com/jswysnemc/mark-shot/issues/148) X11 全屏覆盖重复/受工作区约束 | 同类机制已避免 | 按每个输出创建完整大小的窗口，使用 `override_redirect`。新增真实协议测试通过，其他 3 项 X11 后端检查通过。未使用三物理屏 Cinnamon/Muffin 实机 |
| [#139](https://github.com/jswysnemc/mark-shot/issues/139) 非主屏取色、扫码、尺子、录制错屏 | 共用选区来源保持输出身份；发现 Portal 录制同类风险 | 录制负原点、非主屏、混合 DPI、单输出筛选及跨屏拒绝测试通过。Portal 无法验证授权来源，却曾直接套用预选区域；现由原生 Portal 会话匹配来源位置、逻辑尺寸和真实像素尺寸；缺失或不匹配时拒绝预选裁剪，完整来源仍可用 |
| [#132](https://github.com/jswysnemc/mark-shot/issues/132) 设置空闲占用单核 | 已复现并修复 | 读取状态时设置目录权限，触发监听并再次读取。监听改为只关注 `status.json` 内容和名称变化，真实文件事件回归先失败后通过；20 秒实测 CPU 从 104.24% 降至 0.00% |
| [#107](https://github.com/jswysnemc/mark-shot/issues/107) Wayland 贴图拖动距离减半 | 几何检查未复现 | 贴图拖动使用固定输入表面和不接收输入的预览层。私有 Sway 的 125%/150% 双屏验证八方向缩放、连续跨屏、裁切、滚轮锚点和原图像素。尚未在 Hyprland 实测 |
| [#12](https://github.com/jswysnemc/mark-shot/issues/12) 分数缩放只截到部分画面 | 未复现 | 独立 GTK 全屏四色窗口，125%/150% 两输出各进行默认、`QT_FONT_DPI=128`、`QT_FONT_DPI=192 + QT_SCALE_FACTOR=2 + GDK_SCALE=2` 三组测试。六组均为完整 1200×900，且与独立截图逐像素一致 |

## 设置窗口占用

使用相同隔离 niri、580×650 逻辑设置窗口及 1.2 倍输出比例，静置采样 20 秒。
CPU 按单核 100% 计算；仅统计设置进程树，不包含桌面合成器。

| 版本 | CPU 平均 | RSS 平均 | PSS 平均 | SwapPSS |
| --- | ---: | ---: | ---: | ---: |
| 修复前 | 104.24% | 46.32 MiB | 33.68 MiB | 0 |
| 修复后 | 0.00% | 46.25 MiB | 34.11 MiB | 0 |

原始采样：`target/legacy-issue-evidence/settings-idle.json`、`settings-idle-fixed.json`。
PSS 受共享映射的其他进程影响，不应把两次 PSS 差异解释为分配变化。
错误反馈的系统调用跟踪在 5 秒内记录了约 2,980 次 `chmod`。

## 可重复验证

```sh
cargo test --lib --locked app::panels::toolbar::tests::geometry
cargo test --lib --locked status_reads_and_directory_permissions_do_not_wake_the_reader
cargo test --lib --locked features::recording::selection
cargo test --lib --locked pending_authorization_is_cancellable
uv run --no-project --python /usr/bin/python3 python scripts/desktop-integration-workflow.py \
  --binary target/release/mark-shot --root /tmp/mark-shot-hotkeys
uv run --no-project --python /usr/bin/python3 python scripts/legacy-capture-workflow.py \
  --binary target/release/mark-shot --artifacts target/legacy-capture-new
uv run --no-project --python /usr/bin/python3 python scripts/wayland-pin-geometry.py \
  --binary target/release/mark-shot --artifacts target/legacy-pin-new
```

图形脚本需要已有私有 Sway 工具、GTK3/Cairo、Pillow 及 Wayland 输入/截图工具。
证据目录参数必须使用新目录。X11 测试根目录需短于 Unix socket 路径上限。
快捷键脚本的独立触发之间已加入生产防抖间隔；普通贴图窗口允许一个文件事件监听线程，
仍检查线程数稳定且空闲 CPU 时钟不增长。

## 最终检查

工作区测试 1544 项通过、28 项忽略；Linux 全目标 Clippy、Windows GNU 全测试目标交叉
Clippy、Rust 格式检查、差异空白检查与 Windows 工作流语法检查通过。另在私有 Xvfb 中
执行 4 项默认忽略的后端测试，全部通过。Windows 原生 Media Foundation/GPU 测试未执行。

## 范围

这些结果覆盖真实私有协议会话、坐标回归与代码审查，不能代表所有桌面、显卡和物理显示器。
Windows 录制错误处理的修改与尚未完成的实机验收见
[Windows 视频提交失败分析](recording/windows-write-sample-failure.md)。
