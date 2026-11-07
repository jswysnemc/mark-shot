# 活动窗口截图兼容范围

`--window` 使用独立窗口后端工厂。KDE 继续通过 ScreenShot2 捕获活动窗口；niri 从 IPC
读取稳定窗口 ID、工作区及所属输出比例，再对这个 ID 执行精确窗口截图。实现没有使用
整屏裁切，窗口图片不会包含其他窗口、合成器背景或 niri 焦点环。

## niri 剪贴板边界

本机 niri 为 `26.04 (v26.04-69-g3d2a6738)`。私有注册表证实它提供窗口列表及
wlr-screencopy，但没有 `ext_foreign_toplevel_image_capture_source_manager_v1` 和
`ext_image_copy_capture_manager_v1`。不能把当前上游的窗口来源实现视为本机可用协议。

`ScreenshotWindow` 动作始终复制 PNG。本机新增的 `stdout` 请求返回
`Screenshot { png_base64 }`，真实探针确认它仍然覆盖剪贴板，关闭磁盘写入也不能避免。
因此兼容后端只有在本次输出包含复制时才执行截图，例如：

```sh
mark-shot --window --to c
mark-shot --window --to co --include-cursor
mark-shot --window --to cs --output /absolute/output-directory
```

仅保存、贴图或标准输出均返回明确错误，原剪贴板保持不变。后端不先复制旧剪贴板再恢复，
避免覆盖期间产生额外记录或干扰并发复制。没有相关窗口后端、无焦点、错误 IPC 或窗口
关闭时也返回错误，不返回显示器图片。

兼容动作可能触发 niri 自身的截图通知；`--nonotify` 只控制应用通知。无剪贴板副作用、
由应用统一控制通知的窗口来源协议仍需接入。

## 模块与生命周期

```text
src/backend.rs                         窗口与输出捕获分别选择后端
src/app/instant.rs                     延迟之后获取活动窗口，再进入统一交付
src/backend/wayland/capture/niri_window/
  mod.rs                               复制授权与五秒总期限
  target.rs                            固定窗口身份、工作区和输出比例
  target/tests.rs                      活动窗口与输出边界回归
  legacy.rs                            显式窗口 ID、完成事件和私有临时文件
  frame.rs                             有界 PNG 解码及预乘颜色导入
  tests.rs                             原始尺寸、透明度与损坏图片回归
src/backend/wayland/niri/ipc.rs         共用有界行读取、服务身份和可取消异步连接
src/backend/wayland/niri/ipc/tests.rs   传输、取消、关闭与身份边界回归
examples/window-capture-target.rs       独立颜色和透明度来源
scripts/wayland-window-capture.py       私有原生工作流
```

IPC 与 Wayland 对端的进程 ID、用户 ID 都来自内核。两者必须一致，套接字名称或窗口标题
不能替代这个检查。事件连接沿用已校验路径和身份，不再次读取环境变量。

每行回复上限为 1 MiB，合并到达的多行事件保留在同一个缓冲读取器中。先建立事件订阅，
再请求固定窗口；仅接受本次随机私有目录内的唯一 PNG 路径。整个请求包含连接、焦点、
事件和图片导入，最长五秒。等待采用异步套接字；取消后释放连接和临时目录，不保留
截图辅助进程或周期等待线程。有限图片解码使用短期后台任务。

图片文件及解码分配上限为 256 MiB，单边上限为 32,768。niri 截图编码直接写入预乘
渲染颜色；导入保持这些颜色，统一 PNG 导出再转换为正常透明颜色，避免半透明窗口变暗。
像素尺寸来自窗口截图，缩放来自窗口所属输出，不根据鼠标位置或图片宽高比例猜测。

## 验证

原始回归首先在私有 niri 中确认旧候选的 `--window --to co` 失败。确定性窗口提供
不透明、128、64 和零透明度四个区域，并用两种身份颜色区分窗口。验证比较全部 RGBA
预乘像素及原生尺寸，同时检查最终剪贴板图片。

私有一倍与 1.5 倍流程覆盖两个活动窗口、延迟期间切换焦点、浮动窗口、指针包含与排除、
没有复制授权、缺失 IPC、无焦点及临时文件回收。协议测试另外检查错误进程在请求前拒绝、
窗口关闭立即结束、合并多行事件、过大与半行回复、取消后服务端断连、工作区所属输出和
非法缩放。完整构建与结果清单位于 `target/verification-niri-window-r1/result.json`；
独立候选为 `target/niri-window-candidate-r2/mark-shot`，现有发布文件保持单独记录。

验证必须断开私有 niri 的通知总线，避免截图动作向用户会话发送通知：

```sh
env DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/mark-shot-window-test \
  uv run --no-project --with pillow python scripts/wayland_test_runtime.py \
  --root target/window-runtime --scale 1.5 -- \
  python scripts/wayland-window-capture.py \
  --binary target/niri-window-candidate-r2/mark-shot \
  --target target/niri-window-candidate-r2/window-capture-target \
  --root target/window-checks
```

## 未完成范围

当前验证仅覆盖本机版本和私有单输出，混合输出、其他 niri 版本、带客户端装饰或子表面的
复杂窗口仍需实际验证。上游窗口图像来源协议尚未接入，当前 niri 兼容入口不支持没有
复制动作的交付。窗口及 UI 悬停框选已由独立区域识别模块接入，进度见
[region-detection-compatibility.md](region-detection-compatibility.md)。完整 Qt 光栅细节和
整体性能审计继续保留在重写目标中。
