# 录制验证记录

日期：2026-11-11。验证对象为 `feat/native-recording` 分支。

## 本地验证

- Rust 工作区 1531 项测试通过、27 项忽略；Linux 全目标 Clippy（默认和 GPU OCR 配置）、Windows GNU 全测试目标交叉 Clippy 通过
- Python 验证工具共 234 项测试通过，Ruff 导入检查通过
- 隔离 Niri 桌面使用随包 wf-recorder 和媒体工具，控制窗口启动、录制状态、命令行暂停/继续/停止、保存均通过
- 修复 Wayland 会话中不同 XWayland `DISPLAY` 导致终端无法控制桌面窗口的问题，并添加会话隔离回归
- 320×240 区域，软件 H.264，系统音频 AAC：录制约 2 秒、暂停约 2 秒、继续约 2 秒，成品 4.224 秒；视频 4.121 秒，音频 4.224 秒；暂停期间状态时长不变
- MKV：320×240 H.264/AAC，成品 2.176 秒；GIF：320×240、无音轨，成品 2.080 秒
- 上述成品均完成完整解码；MKV 解码验证保留 demuxer 时间基，避免 null 输出重采样产生时间戳舍入警告
- 倒计时取消、非法配置不创建文件、同名目标不覆盖、X11 次屏区域坐标和边界均有回归测试
- 用延迟两秒的真实 GIF 导出验证关闭行为：停止后关闭控制窗口，仍完成 GIF 保存和完整解码，随后退出窗口

## 边界

Xvfb 测试确认并修复 X11 组件区域参数错误；该环境只有 llvmpipe，GPU Screen Recorder 明确拒绝启动，不能当作真实 X11 录制通过。X11 目前需要可用的硬件 OpenGL，即使选择软件编码也需要 GPU 完成颜色处理。

隔离 Portal 测试中，系统 GSR 与随包版本均未收到视频帧；支持 wlr-screencopy 的隔离桌面已经通过直接路径验证。GNOME/KDE Portal、真实 X11、各厂商硬件编码、HDR、长期音视频漂移尚无实机验收结论。没有进行同等场景的 CPU、功耗或画质对照测试，不提供倍数性能结论。

Windows 交叉检查证明代码可以编译，不代替 Windows 运行验证。远程工作流会使用 Windows Media Foundation 编码运动色块与正弦音轨，并使用随包工具核验尺寸、时长、编码和完整解码；桌面 WGC 与真实 WASAPI 仍需要对应设备验收。

## 可重复检查

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
uv run --python 3.14 --with pillow --with numpy python -m unittest discover -s scripts -p 'test_*.py'
bash scripts/recording/build-components.sh linux
```

Windows 工作流以 `media_foundation_encodes_video_and_audio` 测试生成媒体，再执行 `scripts/recording/verify-media.py`。Linux 和 Windows 工作流分别上传完整压缩包与 SHA256 文件；包内包含组件源码、许可证和主仓库提交号。
