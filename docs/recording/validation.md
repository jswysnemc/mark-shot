# 录制验证记录

更新：2026-11-12。分支 `feat/native-barcode-scanning`，基线 `43534bd`，本地工作区改动尚未发布。
已移除旧外部录制器路径，本页只列进程内实现的证据。此前基准仍保存在 [资源基准](benchmark.md) 的历史部分。

## 自动检查

- Rust 工作区：1546 项通过、32 项默认忽略；另执行下述真实媒体和桌面测试
- Linux 全目标 Clippy、Windows GNU 全目标交叉 Clippy、Rust 格式及差异空白检查通过
- Python 工具测试：235 项通过；Ruff 导入检查及 Linux/Windows 工作流语法检查通过
- 标准和需求两路审查发现的源码包重建路径、音频间断、Portal取消、协议约束顺序和空音轨误判问题均已修复并复查
- 新媒体和采集模块均小于750行，没有加入外部媒体进程调用

## 原生媒体与实际桌面

| 场景 | 实际结果 |
| --- | --- |
| H.264/AAC、MKV、GIF | 私有共享库生成320×240运动色块及440Hz音轨；MP4约2.027秒、MKV约2.026秒、GIF约1.930秒，全部通过独立完整解码 |
| 音频间断 | 中断0.5至1.0秒，独立解码0.6至0.9秒RMS为0，前后音频有效；没有压缩掉时间间隔 |
| 异常析构 | 未显式finish仍产生可解码分片MP4；非法输入和取消导出明确失败 |
| X11 | 私有Xvfb，MIT-SHM软件录制、非零区域、暂停恢复、保存通过，无硬件OpenGL要求 |
| Wayland共享内存 | 私有双输出Sway，125%/150%比例，ext-image-copy采集和暂停恢复通过 |
| Wayland显存 | 私有Sway/GLES2、AMD Radeon780M，实际DMA-BUF/VAAPI路径通过；独立GPU色块测试确认RGB通道、方向和非零裁剪 |
| PipeWire音频 | 独立音频服务器、虚拟输出和440Hz测试音，暂停前后解码RMS符合输入，不使用真实声卡 |
| PulseAudio回退 | 强制首选PipeWire地址不可达，原生Pulse客户端连接私有兼容服务器；有效声音及暂停恢复通过 |
| Portal | 私有Portal/WLR及PipeWire，授权次屏动态画面、非零区域、暂停恢复、关闭通过；已修复授权监听析构时Tokio运行时退出顺序 |
| 暂停时停止 | X11与Wayland均通过，暂停时间不进入最终时长 |
| 实际帧率 | 软件和DMA-BUF两条720p30动态段逐帧核验均为30.0fps；早期仅13fps的调度与样本已修正、废弃 |
| 进程审计 | strace覆盖真实GPU录制，以及H.264/AAC、MKV、GIF全过程；各自只有被测程序的初始execve，没有媒体子进程 |
| Windows共享库 | 交叉构建DLL约5.20MiB，PE导入表只有系统/API Set依赖；隔离Wine实际完成GIF导出及中文路径读写 |
| 默认库位置 | 清除开发路径变量后，从可执行文件旁media目录加载并执行真实编码、导出测试通过 |
| 随包源码 | 解压约12.99MiB源码包，在独立目录按随包脚本重建成功，无需再次下载源码 |

实际桌面测试、独立解码和进程跟踪保存在 `target/native-recording-evidence/`；媒体夹具在 `target/media-evidence/`。
详细CPU/RSS/PSS口径、实际帧率及有效样本见[资源基准](benchmark.md)。

## 本地包体积

| 内容 | 大小 |
| --- | ---: |
| Linux主程序 | 约30.45MiB |
| Linux私有媒体库 | 约7.38MiB |
| Linux完整对应源码包 | 约12.99MiB |
| 完整本地tar.gz，包括源码、许可、文档 | 约29.33MiB |
| 完整解压文件总计 | 约52.69MiB |
| Windows导出DLL | 约5.20MiB |
| Windows媒体源码包 | 约11.96MiB |

这是本地工作区候选包，不是正式发布；`SOURCE-COMMIT`只标识基线，附带工作区差异摘要。
精确字节数与本地文件位置见 `target/native-recording-evidence/package.json`。
小体积不能只看主程序：此处包含可替换库、源码、许可和文档，操作系统驱动及协议库另计。

## 尚未验证的边界

Windows用户报告的 `Video: 0x80070057` 已定位到样本提交链路，代码增加首帧软件回退、异常收尾和阶段诊断；
本环境不能复现该Windows驱动错误，不能宣称实机问题已经验收。Wine导出通过也不等于WGC/WASAPI通过。
详见[Windows失败分析](windows-write-sample-failure.md)。

Portal当前使用共享内存，没有接入DMA-BUF。静态空白headless输出可能一直等首次damage，超时会明确失败；
实际动态来源已通过。没有GNOME/KDE Portal、多厂商硬件编码、HDR、长期音视频漂移或真实三物理屏完整矩阵。
直接Wayland旋转输出使用CPU兼容路径，NVENC也尚未做免CPU回读。不能宣称所有环境均支持GPU直通。

## 复现入口

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/media/build-libraries.sh linux
MARK_SHOT_MEDIA_LIBRARY="$PWD/target/media-linux/libmarkshot-media.so" \
MARK_SHOT_MEDIA_EVIDENCE="$PWD/target/media-evidence" \
cargo test -p shot-media --locked -- --ignored --nocapture
```

先用 `cargo test --lib --no-run --message-format=json` 确认当前测试可执行文件，然后运行：

```sh
uv run --with pillow python scripts/recording/native-workflow.py \
  --mode x11 --test-binary /absolute/path/to/current/test-binary \
  --media-library "$PWD/target/media-linux/libmarkshot-media.so" \
  --directory /tmp/mark-shot-native-recording-new
```

模式还包括 `wayland`、`gpu`、`audio`、`pulse`；目录必须未存在。
GPU模式需要可用的render节点，可显式设置 `WLR_RENDER_DRM_DEVICE`。
音频模式启动私有D-Bus、PipeWire、WirePlumber和虚拟输出；Pulse模式另启动私有兼容服务。
Windows工作流使用系统Media Foundation夹具，并以仅供CI的独立解码器校验MP4、MKV与GIF；测试工具不进入发布包。
