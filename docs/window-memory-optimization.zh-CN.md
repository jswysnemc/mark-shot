# 截图与贴图编辑窗口内存优化

## 范围与结论

本轮在 `23e2142` 加上尚未提交的历史预览和 OCR 输出解码优化基础上对照测量，
不使用历史记录页面的数据代替截图或贴图窗口的数据。

- 修复共享高分屏图片进入画布时，清除 DPR 元数据引发的整图复制。
- 修复全屏裁剪与原图范围相同时，仍然复制整张截图的问题。
- 不减少原图分辨率，不关闭后台 OCR，不修改文字选择手势，不强制回收工作集。
- 高分屏贴图加编辑器场景减少约一张原图的存活像素；普通截图窗口的持续占用没有显著下降。
- 不能据此声称用户之前观察到的约 224 MiB 已经解决，也不能把不同场景的节省量累加。

## 文件职责

```text
qt/src/canvas/canvas_image_input.h/.cpp  画布输入格式、DPR 与像素共享
qt/src/shot_window_setup.cpp            调用画布输入模块
qt/src/capture_geometry.cpp             整帧共享与局部独立裁剪
qt/tests/canvas_image_input_test.cpp    元数据、行跨度、寿命和写入隔离
qt/tests/canvas_image_memory_test.cpp   真实窗口与贴图编辑入口
qt/tests/capture_geometry_test.cpp      裁剪像素共享与局部所有权
qt/tests/gui_memory_scenarios.cpp       高分屏贴图编辑测量场景
qt/tests/gui_memory_benchmark.cpp       测量参数与结构化结果
```

## 根因与实现

### 高分屏共享图片

贴图窗口使用 `QPixmap` 保留原图，`openImageEditor()` 通过 `toImage()` 和
`openImageForAnnotation()` 把同一份像素交给 `ShotWindow`。
画布按原图像素计算坐标，因此需要把自身图像的 DPR 设为 1。
旧实现直接修改共享 `QImage` 的 DPR，触发写时复制；源图的 DPR 为 2 时，
一张 3840 × 2160、每像素 4 字节的图片多分配 31.64 MiB。

新实现保持以下边界：

1. RGB32 和预乘 ARGB32 直接复用；需要预乘的其他格式仍执行正确转换。
2. DPR 已为 1 或像素独占时沿用简单路径。
3. 共享高分屏输入使用独立元数据的只读图像视图，保留源图引用直到最后一个视图释放。
4. 保留色彩空间、分辨率、偏移和文本元数据；行跨度沿用源图，不假设每行紧密排列。
5. 对源图或视图写入时仍然复制，不能相互修改像素。

### 全屏裁剪

实际 grim 截图链路会对已经覆盖目标输出的图片调用 `cropFrameToRequest()`。
旧实现不区分整帧和局部选区，始终使用 `frame.copy(crop)`。
新实现仅在最终像素裁剪矩形等于 `frame.rect()` 时返回共享图像。
局部选区仍然创建独立小图，避免小选区长期保留整张多屏截图。

这消除一次确定的临时分配，不代表整个应用的 RSS 峰值一定下降：
解码和显示缓冲区等其他阶段仍可能决定总峰值。

## 对照测量

### 高分屏贴图与编辑器

- 两侧都是同一机器、相同 Release 配置和分配策略。
- Qt offscreen、`QT_SCALE_FACTOR=1`，原图本身的 DPR 为 2；不要把显示环境 DPR 与原图 DPR 混为一谈。
- 使用生成的 3840 × 2160 RGB32 图案，调用真实贴图窗口及编辑入口，两个窗口同时保留。
- 使用独立配置，测量目录不放置 OCR 运行库和模型；这些数字不包含实际 OCR 推理。
- 每组交替测量三次，表中数值是中位数，单位 MiB。
- 读取本进程 RSS、系统维护的 HWM 和 glibc 存活分配量；不调用内存整理函数。

| 场景 / 指标 | 优化前 | 优化后 |
| --- | ---: | ---: |
| DPR 2 贴图加编辑器，RSS | 202.52 | 171.02 |
| DPR 2 贴图加编辑器，存活堆分配 | 143.93 | 112.29 |
| DPR 2 场景，进程峰值 | 202.52 | 171.02 |
| DPR 2 场景，全部窗口关闭后 RSS | 76.02 | 76.12 |
| DPR 1 贴图加编辑器，RSS | 170.87 | 170.90 |
| 生成图截图窗口显示后 RSS | 145.75 | 145.95 |
| 生成图截图场景，进程峰值 | 153.23 | 153.36 |

高分屏编辑窗口仍显示时，RSS 减少 31.50 MiB，约 15.6%；
存活堆分配减少 31.64 MiB，与一张原图一致。
关闭窗口后没有显著变化，普通 DPR 截图或编辑也没有显著变化。

复现单个场景：

```sh
cmake --build build/memory-regression/build-fixed \
  --target mark-shot-gui-memory-benchmark --parallel 4
QT_QPA_PLATFORM=offscreen QT_SCALE_FACTOR=1 \
  build/optimization-round3/candidate/mark-shot-gui-memory-benchmark \
  --scenario pin-edit-hidpi --width 3840 --height 2160
```

`build/optimization-round3/baseline/` 与 `candidate/` 是本机保存的对照二进制，
后者从上述 Release 构建目录复制。复测时必须保证两侧运行库、配置和插件一致，
不能把携带 OCR 插件的安装目录与空测量目录直接比较。
原始 JSON 和日志保存在 `build/optimization-round3/results/`。

同一环境下再加 `--max-peak-mib 185` 验证可失败的内存预算：
旧二进制峰值 202.31 MiB，退出码 1；新二进制峰值 170.80 MiB，退出码 0。
这个门槛仅用于当前受控测量，不作为不同平台和字体环境的通用 CI 上限。

### 原生 Wayland 截图

通过已有 `recording_wayland_session_test.py` 包装器创建独立 D-Bus、Xvfb、niri，
仅截取隔离桌面，调用实际 `mark-shot --capture`，两侧携带相同 layer-shell 插件。
本地诊断脚本 `build/optimization-round3/capture_probe.py` 采样 10 秒，
保存应用自身的 `/proc/<pid>/status` 和 `smaps`，不按进程名称模糊匹配。

初步测量已经复现约 204 MiB 的 4K 截图窗口。其中一种稳定映射组成是：

| 映射类别 | 占用 |
| --- | ---: |
| 匿名驻留页，包括原图和应用堆 | 约 53.02 MiB |
| 库、字体等文件驻留页 | 约 56.15 MiB |
| 三张 `wayland-shm` 显示缓冲区 | 约 94.94 MiB |

每张 4K 显示缓冲区约 31.64 MiB。另一次测量只有两张缓冲区，RSS 约 172 MiB；
缓冲区数量会影响总量，不能把这种差异归功于源图优化。
这些数字不等同于 PSS，也不代表全部内存都由应用私有持有。
本轮不通过强制丢弃合成器仍可能使用的缓冲区换取更低的监控数字。

完成构建和回归测试后，再串行交替测量三轮，结束时 RSS 中位数为
204.11 → 203.90 MiB，系统记录峰值的中位数也为 204.11 → 203.90 MiB，
没有显著改善。原始结果编号为 `native-4k-{baseline,candidate}-{4,5,6}`。
个别运行还出现文件驻留页减少或显示缓冲区数量变化；这些变化不能解释为存活对象减少。

## 回归与限制

- 修复前运行新增断言：整帧裁剪像素地址不同，共享 DPR 2 输入进入画布后像素地址不同，两组测试失败。
- 修复后，裁剪与画布相关四个测试目标通过，包括显示缩放 2 倍的窗口测试。
- 真实贴图编辑入口验证像素共享；关闭贴图后，编辑器仍能读取原图。
- 输入模块覆盖两种共享格式、独占和共享输入、DPR 1 / 1.5 / 2、透明像素、带行填充的只读外部缓冲区。
- 使用源图和视图分别写入、`QPainter` 写入、多个视图顺序释放，验证写入隔离和一次性释放。
- ASan/UBSan 输入模块测试通过；关闭 LeakSanitizer，系统 Qt 未插桩，不声称整个应用不存在泄漏。
- Debug 完整构建成功；全量测试 192/196 通过。
- 四个失败与修改前相同：`annotation-cursor` 的空白光标断言，以及三个在应用启动前失败的 Hyprland 环境初始化测试。
- 截图裁剪、画布、视口渲染、贴图文字选择、历史窗口、GUI 场景及 wlroots 录制的 11 个针对性测试全部通过。
- `git diff --check` 与源码行数限制检查通过。

本轮没有验证真实 GNOME 桌面的显存、Windows 工作集或携带完整 OCR 模型的全部交互峰值。
截图显示缓冲区仍是明确的大块占用，本轮结果只证明去除了特定路径的重复像素。
