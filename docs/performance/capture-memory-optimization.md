# 截图内存优化与最终验证

日期：2026-11-12。最终候选为 `reclaim-interval`，保留 Windows、X11、Wayland，采用单个原生呈现表面。原始截图、编辑内容和导出语义保持一致。

## 结果与边界

4K 起始选择 RSS 从原始基线约 160.5 MiB 降至 97.0 MiB，全屏编辑从约 155.5 MiB 降至 92.2 MiB，均下降约四成。同条件 Babry 分别约为 37.3 / 37.6 MiB，尚未达到其内存水平。

同一二进制、同一私有 Niri 的六次拖选对照中，优化模式平均帧率下降约 1.6%，95 分位帧间隔略有改善，通过本轮预设的 2% 帧率与 5% 帧间隔容差。其他主要交互完成测量，但没有同样的六次配对结果。空闲回收后的首次恢复仍有代价：X11 小矩形中位时长增加约 2.0 毫秒，大范围矩形增加约 6.0 毫秒。不能将这些结果表述为全部交互性能完全相同。

静态内存、常规交互、Wayland 恢复及源码哈希保存在 [capture-memory-results.json](capture-memory-results.json)，六次拖选对照保存在 [capture-selection-pair-results.json](capture-selection-pair-results.json)，X11 / Windows 内存与测试结果保存在 [capture-platform-results.json](capture-platform-results.json)，窗口像素恢复保存在 [capture-platform-resume-results.json](capture-platform-resume-results.json)。原始基线与原因分析见 [capture-memory-comparison.md](capture-memory-comparison.md)。各项汇总记录原始 JSON 路径及文件哈希。

## 改动与性能取舍

- 透明前景只复制实际变化的 256 字节块；固定图形使用局部栅格，保留完整画布坐标。
- 软件场景按选区和候选边界切分扫描行，透明前景直接使用背景；固定遮罩按整数通道合成，保持原有两阶段预乘舍入。
- Wayland 备用帧只修复累计缺少的损坏范围；空闲 250 毫秒后回收真实释放的备用缓冲。活动缓冲始终只读，帧回调不作为缓冲释放依据。
- 缩放背景在完整重绘时直接采样，局部绘图按需缓存 256×256 设备像素块。空闲时释放已交付的不透明 CPU 画布，保留有效导航背景块；视图或选区变化仍使旧背景失效，未提交输出保持完整。
- Linux 回收透明前景时读取当前进程页面状态，跳过共享零页，只检查独占页内容；连续相交全零页合并归还，保留非零像素。页面状态无法读取时使用原回收路径。
- 每个输出约 250 毫秒至多检查一次透明区域；空闲 250 毫秒后检查完整透明画布，保留非零前景并停止重复唤醒。非原生场景取消回收期限。

共享零页不占独立像素内存。先前回收策略取消其映射，造成下一帧重复缺页；回归测试先复现问题，再验证修正。窄矩形诊断中，次要缺页约从每帧 5625 次降至 52 次，检查与回收耗时约从 8.6 毫秒降至 0.28 毫秒。这是诊断内核测量，完整程序性能以下述对照为准。

此前同时释放导航缓存，大范围空闲恢复增加约 17.3 毫秒 X11 / 8.8 毫秒 Wine，因此最终方案保留已采样背景。每帧检查页面状态的候选实际未通过配对容差；限制频率后，优化模式每 8 秒读取量均值从约 7.67 MiB 降至 0.69 MiB，最终配对通过。保留导航缓存使缩放绘图后的空闲内存有所增加，这是减少恢复成本的取舍。

测量环境允许读取 `/proc/self/pagemap` 的当前进程页面标志。限制该文件的环境仍使用原路径保持像素正确，但本报告的缺页与交互数据没有覆盖这条回退路径。

## Wayland 静态内存

使用 AMD Ryzen 7 8845H、隔离 Xvfb 与 Niri、空桌面、输出缩放 1.0。每项预热 2 秒、采样 10 秒，采样期间不编译。两个程序依次运行，不包含后台托盘、OCR 推理或录制。RSS/PSS 统计进程树，排除合成器和 X 服务器，不包含显存。全部 SwapPSS 为 0，静态 CPU 为 0–0.1%。

单位 MiB。原始基线为先前两轮测量均值范围；最终 Mark Shot 与 Babry 使用同一私有桌面和输出尺寸。

| 分辨率 / 状态 | 原始 Mark Shot RSS | 最终 Mark Shot RSS | Babry RSS | 最终 Mark Shot PSS | Babry PSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1270×720 / 起始选择 | 48.12–48.66 | 38.91 | 9.14 | 33.14 | 4.55 |
| 1270×720 / 全屏编辑 | 44.83–45.13 | 38.66 | 9.37 | 32.88 | 4.67 |
| 1920×1080 / 起始选择 | 62.36–62.67 | 47.81 | 13.57 | 39.87 | 6.83 |
| 1920×1080 / 全屏编辑 | 59.30–59.54 | 43.98 | 13.75 | 36.02 | 6.88 |
| 3840×2160 / 起始选择 | 160.48–160.55 | 97.00 | 37.29 | 77.19 | 18.71 |
| 3840×2160 / 全屏编辑 | 155.41–155.55 | 92.19 | 37.63 | 72.32 | 18.85 |

最终 4K 映射仍保留约 31.65 MiB 原始截图及约 31.64 MiB 当前原生呈现帧。SHM 映射虚拟长度约 63.28 MiB，空闲实际 RSS 为一帧；程序映射在起始 / 全屏状态分别约为 20.38 / 18.12 MiB，另有字体、运行状态和系统库。Babry 直接复用捕获共享文件作为冻结背景，初始场景约保留一份整屏像素，程序页也更少。剩余差额没有在本轮消除。

## Wayland 交互与配对对照

4K 私有 Niri 下发送固定轨迹输入，记录主表面帧请求、CPU、忙碌与释放鼠标 0.6 秒后的内存，每项 6 秒。CPU 按单个逻辑核心 100% 统计；帧率表示主表面绘制请求速率，不能替代显示器端呈现延迟。

| 场景 | FPS | 95 分位帧间隔 ms | CPU % | 忙碌 RSS MiB | 空闲 RSS MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 悬停 | 64.14 | 16.55 | 13.99 | 135.44 | 99.38 |
| 拖选 | 49.72 | 25.39 | 43.23 | 139.86 | 101.65 |
| 矩形 | 51.72 | 25.75 | 38.25 | 138.63 | 97.89 |
| 画笔 | 63.95 | 16.33 | 4.33 | 135.80 | 102.72 |
| 缩放 | 43.17 | 26.68 | 187.83 | 158.27 | 93.44 |
| 缩放后矩形 | 53.14 | 25.54 | 39.48 | 162.82 | 113.62 |
| 缩放后画笔 | 63.55 | 16.90 | 6.32 | 156.89 | 118.18 |

拖选另做三组配对，每次 8 秒，始终使用同一个私有合成器及同一二进制。顺序为旧 / 优化、优化 / 旧、旧 / 优化，保留全部六次结果。通过标准在测量前固定：优化 / 旧模式平均 FPS 至少 0.98，各次 95 分位帧间隔的中位值比例至多 1.05。

| 配对统计 | 旧模式 | 优化模式 |
| --- | ---: | ---: |
| 平均 FPS | 49.4756 | 48.7013 |
| 中位 FPS | 48.8497 | 48.8028 |
| 95 分位帧间隔中位值 ms | 27.535 | 27.372 |
| 平均 CPU % | 41.6107 | 40.4667 |

平均 FPS 比例为 0.98435，帧间隔比例为 0.99408，均通过。不同批次的绝对帧率存在波动，不将单次最快值作为固定保证。

## X11 与 Windows 后端

同一最终二进制切换旧模式与优化模式，使用独立 3840×2160 Xvfb。每项先测静态状态，再持续发送 60 Hz 拖动 6 秒；缩放矩形通过真实后端输入改变视图，截图确认绘出标注。Windows 使用 MSVC release 程序、独立 Wine 前缀及 Windows 原生后端，未进行 Windows 实机测量。

| 后端 / 场景 | 静态 RSS 旧→优化 MiB | 忙碌 RSS 旧→优化 MiB | CPU 旧→优化 % | 绘图后空闲 RSS 旧→优化 MiB |
| --- | ---: | ---: | ---: | ---: |
| X11 / 拖选 | 128.36→97.84 | 128.60→110.38 | 73.67→45.67 | 129.52→109.71 |
| X11 / 缩放矩形 | 159.05→94.25 | 159.29→140.68 | 51.33→53.17 | 159.18→119.80 |
| Windows/Wine / 拖选 | 208.33→176.92 | 208.73→197.28 | 79.83→75.83 | 208.64→199.35 |
| Windows/Wine / 缩放矩形 | 238.23→175.84 | 246.96→228.64 | 80.00→82.17 | 238.28→202.18 |

这四项静态、忙碌和绘图后空闲内存均下降。X11 拖选 CPU 明显下降；缩放矩形 CPU 在 X11 / Wine 分别增加约 1.8 / 2.2 个百分点。脚本没有记录两个后端的实际呈现 FPS，不能将 CPU 和输入速率作为实机帧率验证。

## X11 / Windows 窗口像素恢复

使用私有 4K Xvfb、中灰背景窗口及 XTest 输入，轮询根窗口可见的 8×8 像素，测量输入至服务器像素改变的观察时长。观察位置远离鼠标图案；矩形样本要求新像素包含红色边框。未发生重绘的对照实际超时失败。每组预热一次，再保留 12 次样本。指标包括 Wine、X 服务器及约 1 毫秒轮询开销，排除真实显示器呈现延迟。

拖选始终按住鼠标，暂停后继续改变角点；缩放矩形在释放鼠标后等待，再开始新图形。局部矩形第一次移动为 150×115 设备像素，准备矩形与测量位置分开，避免已有边框提前触发观察。下面列出中位时长，单位毫秒，顺序为旧模式→优化模式。

| 后端 / 场景 | 等待 0.05 秒 | 等待 0.7 秒 |
| --- | ---: | ---: |
| X11 / 拖选 | 22.54→12.23 | 22.45→16.51 |
| X11 / 缩放后小矩形 | 8.36→8.79 | 9.48→11.51 |
| Windows/Wine / 拖选 | 23.95→21.16 | 24.14→21.54 |
| Windows/Wine / 缩放后小矩形 | 18.86→18.96 | 21.88→22.86 |

0.7 秒等待触发不透明 CPU 画布回收，同时保留导航缓存。X11 小矩形组的优化模式 RSS 为 116.76–117.33 MiB，Wine 为 206.57–206.63 MiB。小矩形空闲恢复中位代价约为 2.0 / 1.0 毫秒，连续输入组分别约为 0.4 / 0.1 毫秒。

另将缩放后第一次移动扩大至屏幕大部分区域。两个模式采用相同轨迹及标注数量，每组 12 次；等待组之间继续累积矩形，按每组旧 / 优化对照评估。

| 后端 / 大范围矩形 | 等待 0.05 秒中位 ms | 等待 0.7 秒中位 ms | 等待 0.7 秒最大 ms |
| --- | ---: | ---: | ---: |
| X11 | 43.99→44.62 | 69.41→75.38 | 79.84→85.36 |
| Windows/Wine | 52.17→52.16 | 83.30→83.15 | 101.57→111.94 |

大范围空闲恢复在 X11 增加约 6.0 毫秒。Wine 中位值接近旧模式，但优化模式最大样本更长，不能据此宣称恢复没有额外代价。该组优化模式空闲 RSS 分别为 124.37–124.43 / 206.21–206.25 MiB。

## Wayland 空闲恢复与正确性

同一最终 Linux 二进制分别等待 0.05 / 0.7 秒后开始局部矩形绘图，每组 12 次。优化模式与旧模式分别启用和关闭原生场景。指标是输入至下一个主表面帧请求的父进程观察时长，包含观察开销，排除显示器呈现延迟。

| 等待 | 中位时长 旧→优化 ms | 最大时长 旧→优化 ms |
| --- | ---: | ---: |
| 0.05 秒 | 14.88→14.68 | 18.73→22.39 |
| 0.7 秒 | 27.35→27.95 | 30.26→30.66 |

空闲回收后中位观察时长增加约 0.6 毫秒。首次恢复仍需要重新构建已回收的备用原生帧。

- 完整工作区：1598 项通过、34 项按环境要求忽略，87 套测试，零失败；Linux 全目标 Clippy `-D warnings` 通过，Windows MSVC 全目标检查与两平台 release 构建通过。
- Windows/Wine 当前测试程序：52 项通过、7 项环境测试忽略，包含原生后端、两种通道顺序的软件场景、渲染、透明画布与驻留调度。
- X11 真实部分上传与曝光验证通过；Wayland 真实共享槽位验证活动来源只读、独立目标、累计损坏修复及回收后完整重建。
- Sway 两个输出 125% / 150% 的实际矩形像素与旧模式最大通道差异为 0，分别确认 12809 / 20848 个标注像素。
- 编辑器生命周期测试覆盖放大、缩小、分数比例、混合输出、再次改变视图、空闲替换后局部绘图、有效导航缓存保留及尚未提交输出；原生像素与保留完整 CPU 画布的路径一致。
- Linux 真实页标志测试确认共享零页继续映射、独占全零页实际归还、非零和区域外页面保留。回收许可测试验证每个输出独立限制频率，空闲完成后停止重复唤醒。
- 最终 Rust 格式、Python Ruff 与编译检查、`git diff --check` 均通过；本轮记录的源码和结果文件少于 750 行，原始证据与源码哈希一致，最终 Linux 构建后没有 Rust 源码修改。

## 二进制与复现

最终 Linux SHA256：`7da9fd436cafdfabc3f01c9fa38694567a517ed9fb840771813699c1e47677b6`。

最终 Windows MSVC SHA256：`ef5d9a92c97167f769bd822f87b262274b9a75a113054fc017cd95ef196b8049`。

Babry SHA256：`835d241609cf19152ee821ef6e86303e40a1471425d932e4310c49c6d3ddbbe0`，源码提交 `f10a1c669f2f2fd90ed1eaeab34d27a01474a749`。最终程序保存在 `target/performance-resume/mark-shot-reclaim-interval` 与对应 `.exe`；默认 Linux / Windows release 文件哈希与被测候选相同。

```sh
MARK_SHOT_CAPTURE_SCENE=1 uv run --no-project --python /usr/bin/python3 python scripts/performance/compare_capture_memory.py \
  --mark-shot target/release/mark-shot --babry /home/snemc/workspace/babry/target/release/babry \
  --root /home/snemc/.cache/ms-memory-reproduce-new --seconds 10 --label reclaim-interval

MARK_SHOT_CAPTURE_SCENE=1 uv run --no-project --python /usr/bin/python3 python scripts/performance/capture_interaction.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-interaction-reproduce-new --seconds 6

uv run --no-project --python /usr/bin/python3 python scripts/performance/capture_selection_pairs.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-selection-pairs-new

uv run --no-project --python /usr/bin/python3 --with pillow python scripts/performance/x11_capture.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-x11-reproduce-new --seconds 6 --scenario zoomed_rectangle

uv run --no-project --python /usr/bin/python3 --with pillow python scripts/performance/x11_capture.py \
  --binary target/x86_64-pc-windows-msvc/release/mark-shot.exe --runner wine \
  --root /home/snemc/.cache/ms-windows-reproduce-new --seconds 6 --scenario zoomed_rectangle

MARK_SHOT_CAPTURE_SCENE=1 uv run --no-project --python /usr/bin/python3 python scripts/performance/capture_resume.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-wayland-resume-new --samples 12

MARK_SHOT_CAPTURE_SCENE=0 uv run --no-project --python /usr/bin/python3 python scripts/performance/capture_resume.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-wayland-legacy-resume-new --samples 12

uv run --no-project --python /usr/bin/python3 --with python-xlib python scripts/performance/x11_resume.py \
  --binary target/release/mark-shot --root /home/snemc/.cache/ms-x11-pixel-resume-new --samples 12

uv run --no-project --python /usr/bin/python3 --with python-xlib python scripts/performance/x11_resume.py \
  --binary target/x86_64-pc-windows-msvc/release/mark-shot.exe --runner wine \
  --root /home/snemc/.cache/ms-windows-pixel-resume-new --samples 12
```

大范围首动对照在 `x11_resume.py` 命令后增加 `--scenarios zoomed_rectangle --rectangle-span large`。像素脚本依赖系统 Xvfb、xdotool 和 ImageMagick，Windows 对照另需 Wine。所有根目录必须使用新路径。脚本只创建和回收自己的私有桌面或 Wine 会话，不操作用户当前桌面。
