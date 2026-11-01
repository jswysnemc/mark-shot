# 贴图 OCR 内存回归：一手资料核验

核验日期：2026-11-01。代码观察基线：`1e3877d`，限定为 `qt/plugins/ocr-rapid/` 内检测、识别、会话与插件实现。本机 `pkg-config --modversion libonnxruntime` 返回 `1.29.0`；这只证明开发包版本，实际测量仍须记录插件加载的运行时版本及模型校验值。

本文记录官方接口、上游源码与可验证实验，不报告未经测量的节省比例，不把任务结束后的 RSS 直接判定为泄漏。本文没有修改生产代码，没有读取用户截图或配置。

## 1. 结论

1. 当前实现已经禁用 CPU arena、禁用 memory pattern，并在每次识别结束时释放模型会话。再次提出这些开关不能解释或解决剩余回归。
2. 最直接的生命周期实验是将“同时加载检测与识别模型”改为“检测模型加载、检测、释放，然后按需加载识别模型”。它减少同时存活的会话、权重与独立线程池，但收益及总耗时必须实测。
3. 当前每会话设置 `hardware_concurrency() / 2` 个 intra-op 线程。在 16 个硬件线程的机器上，普通 ORT 线程池配置对应每会话 7 个额外工作线程；双会话可同时存在约 14 个额外工作线程。线程数不是常驻物理内存的直接换算公式。[S1][S2]
4. 输入 `CreateTensor` 已经引用调用方浮点缓冲；可见的完整复制发生在 ORT 输出向 `QVector<float>` 的复制。优先验证后处理直接消费输出，而不是再做一次“输入零复制”改造。[S2][S3]
5. `ORT_ENABLE_ALL` 到 `ORT_ENABLE_EXTENDED` 是可隔离布局优化成本的实验，不是保证省内存的设置。关闭全部图优化、关闭权重预打包、替换分配器都需要独立延迟与正确性对照。[S4][S5][S6]
6. Qt 6.12 大图缩放和部分格式转换使用独立的私有 QtGui 线程池，不是 `QThreadPool::globalInstance()`。它能解释释放 ORT 会话后仍存在的额外工作线程；公开 `QImage` 接口没有单次调用的线程数参数。[S14][S15][S16][S18]
7. 插件对象释放、动态库解除映射、线程退出、分配器把空闲页交还系统，是不同事件。Qt 明确规定加载器析构不等于插件卸载，`QThreadPool::waitForDone()` 又会主动移除工作线程；测量时不能混淆这些边界。[S8][S9][S10]

## 2. 当前代码事实

| 位置 | 已存在行为 | 对实验的约束 |
| --- | --- | --- |
| [会话加载](../qt/plugins/ocr-rapid/rapid_onnx_session.cpp) | 设置 intra-op 线程数，启用全部图优化，禁用 CPU arena 和 memory pattern | 不再将禁用 arena 当作待实施优化；参数实验一次只改变一个选项 |
| [插件加载](../qt/plugins/ocr-rapid/rapid_ocr_plugin.cpp) | `ensureLoaded()` 在检测前加载 det 和 rec | 检测阶段已承担识别模型加载后的内存与线程资源 |
| [插件识别](../qt/plugins/ocr-rapid/rapid_ocr_plugin.cpp) | 整次识别持有 `QMutex`，局部清理器在返回前释放两个模型 | 当前单插件实例内部串行；不能仅靠这一把锁推断调用方队列和图像数量有界 |
| [会话释放](../qt/plugins/ocr-rapid/rapid_onnx_session.cpp) | 依次销毁 `Session` 和 `Env` 包装对象 | 不得把释放模型的工作重复描述为新增能力 |
| [检测输入](../qt/plugins/ocr-rapid/rapid_det_model.cpp) | 最长边限制为 960，宽高对齐 32，生成局部浮点输入与输出 | 大截图不代表检测模型直接接收原始全尺寸，但原图及应用层副本仍可能存在 |
| [识别输入](../qt/plugins/ocr-rapid/rapid_rec_model.cpp) | 高度 48，宽度 16～1280，逐行推理，按时间步执行 argmax 和 CTC 解码 | 改小尺寸或裁掉文本不是无损内存优化；应保留全部字符跨度与阅读顺序 |
| [输出复制](../qt/plugins/ocr-rapid/rapid_onnx_session.cpp) | `Run()` 返回输出张量，再 `resize` 与 `std::copy` 到 `QVector` | 复制阶段存在两份输出数据；消除复制必须保住张量与分配器生命周期 |

`hardware_concurrency()` 的结果不能直接称为物理核心数。当前注释写“物理核数一半”，实际代码使用硬件并发数提示值；设置 8 个线程也不代表额外创建 8 个 ORT 线程。[S1]

## 3. 第一轮：内存选项与线程池语义

### 3.1 CPU arena 与 memory pattern 不是同一机制

ORT C 接口文档对 `DisableCpuMemArena` 的承诺是“Disable the memory arena on CPU”。它没有承诺该调用使推理结束后的进程 RSS 立即恢复初始值。[S2]

ORT C 入门文档解释，默认每会话 CPU 分配器使用 arena，arena 通常保留已分配区域以满足后续请求；同进程多个 arena 会累积空闲区域。共享 arena 是针对这一使用方式的方案。[S7]

`EnableMemPattern` 则在输入形状相同时追踪内部内存分配，为后续请求生成分配模式，使后续请求可以用大块分配满足内部需求；接口文档注明该优化用于顺序执行模式。[S2]

对本项目的判断：

- 当前两个选项均已关闭，不能把输出张量或模型权重仍占内存推导为“关闭 arena 没生效”。
- 检测每会话通常只执行一次，识别每会话执行多行且宽度可能变化。重新开启 memory pattern 是否有收益，必须分别记录形状重复率与识别耗时，不能只依据“动态形状”或“关闭缓存省内存”下结论。
- 禁用 arena 后仍有普通 CPU 分配器。上游 `CPUAllocator::Free` 调用底层对齐释放接口；仅据此不能保证操作系统立刻回收驻留页。[S11]
- `MemoryInfo::CreateCpu(OrtArenaAllocator, ...)` 用于描述张量缓冲位置与分配器类型。当前输入张量使用调用方数据，不能仅凭这个枚举判断 `DisableCpuMemArena()` 失效。[S2]

### 3.2 arena shrinkage 不适用于当前配置

官方说明 arena shrinkage 在每次 `Run()` 结束时扫描可回收区域，且仅适用于 arena 分配器。`memory.enable_memory_arena_shrinkage` 配置源码进一步明确：若包含 `cpu`，不得调用 `DisableCpuMemArena()`。[S7][S12]

因此不能在当前禁用 CPU arena 的代码上直接添加 `cpu:0`，并把它称为更积极的回收。

如单独试验重新启用 arena：

- `kNextPowerOfTwo` 收缩时保留初始分配区域，其余区域才参与回收。
- `kSameAsRequested` 收缩时所有区域都可以成为候选。
- `max_mem` 是 arena 分配上限，不是整个进程内存上限；空间不足会返回错误，不会自动降级模型。
- 每行识别后扫描或回收可能增加后续分配成本，不能只比较结束后的 RSS。[S7]

### 3.3 intra-op、inter-op 与默认顺序执行

官方线程文档明确，intra-op 数量包含调用线程。设置 3 时另外创建 2 个工作线程；显式设置数量时，不采用默认线程亲和策略。默认执行模式为 `ORT_SEQUENTIAL`；只有 `ORT_PARALLEL` 才使用 inter-op 线程池并行执行多个节点。[S1]

对当前默认顺序执行实现：

- 测量 `1 / 2 / 4 / 当前值` 个 intra-op 线程有意义，应同步记录真实线程数、检测耗时、识别总耗时及 UI 响应。
- 调低 inter-op 数量不是当前双会话内存问题的有效首选实验，因为当前没有启用图节点并行。
- 减少线程可能减少线程栈、线程本地状态与部分分配压力，但官方没有提供“每少一个线程节省固定 MiB”的保证。
- C API 特别注明：使用 OpenMP 构建时，`SetIntraOpNumThreads` 不控制 OpenMP 线程数，需要使用相应环境变量。发行包和本机包可能不同，必须检查实际线程数。[S2]

关闭 `session.intra_op.allow_spinning` 主要减少工作线程自旋的 CPU、资源和功耗成本。官方没有将其描述为销毁工作线程或释放模型权重的方法，不能把它作为主要内存回归修复。[S1]

### 3.4 共享 Env 不等于共享线程池

官方默认行为是每会话拥有独立线程池。全局共享线程池需要同时满足：使用 `CreateEnvWithGlobalThreadPools` 创建环境，并在会话选项调用 `DisablePerSessionThreads`。[S7]

C API 还注明，后续环境创建调用会返回先前环境实例并忽略参数。因此：

- 当前两个 `Ort::Env` 包装对象不等于两个完全独立运行时环境。
- 只把两个环境成员改成一个静态对象，不会自动合并会话线程池。
- 必须在环境首次创建时决定全局线程池策略；不能假定后续配置覆盖已有环境。
- 如果全局环境常驻，线程池也可能比本次 OCR 活得更久，这与当前任务后回收策略有取舍。[S2][S7]

分阶段释放 det 后再创建 rec，可能让最后一个环境引用在两阶段之间消失，增加环境重建成本。是否保留一个任务级环境引用，应作为独立对照；不要把全局常驻环境与分阶段会话优化混成一次实验。

## 4. 第二轮：模型生命周期与张量复制

### 4.1 分阶段加载是优先验证的候选

当前 det 与 rec 是两个不同模型，并按顺序使用。官方共享权重功能面向多个会话实际共享相同初始化器的情况，不能据此断言检测和识别权重可以合并。[S7]

更直接的候选生命周期为：

```text
当前：加载 det -> 加载 rec -> 检测 -> 逐行识别 -> 释放 det 与 rec
候选：加载 det -> 检测 -> 释放 det -> 加载 rec -> 逐行识别 -> 释放 rec
```

预期改变的是同一时刻的存活集合，不改变检测输入、检测阈值、识别模型、字符解码或异步交互。正常数据的输出应与基线一致，实际 RSS 与延迟变化仍需测量。

应验证这些边界：

1. 检测失败、识别加载失败、字典读取失败和空结果路径都释放已经加载的资源。
2. 检测产生的框必须脱离 ORT 输出张量有效期，才能销毁检测会话。
3. 保留现有互斥范围或等价同步，不能在另一个请求使用模型时释放会话。
4. 若无检测框时跳过 rec 加载，需要明确错误语义：原实现即使图像无文本，也会发现“文件存在但 rec 模型损坏或字典不可读”。延迟加载会把这种情况改为本次空结果成功。这是错误路径差异，不是所有行为完全等价。
5. 分阶段释放不是取消。仍需由调用方管理取消标记、队列等待与图像持有，不能在 `Run()` 还未返回时销毁会话。

### 4.2 输入已引用调用方数据

`CreateTensorWithDataAsOrtValue` 的官方定义是使用调用方缓冲；`p_data` 仍由调用方拥有，`ReleaseValue` 不释放它。当前 C++ `CreateTensor<float>(memoryInfo, input.data, ...)` 使用这一类接口。[S2][S3]

因此输入向 ORT 的包装本身不是又一份完整浮点输入复制。输入向量仍必须覆盖同步 `Run()` 的整个有效期，不能为了“提前释放”制造悬空指针。

### 4.3 输出复制可以单独消除

当前 `Run()` 返回后，ORT 输出张量与新分配 `QVector<float>` 在 `std::copy` 期间并存。复制数据量可精确计算：

```text
检测输出复制字节数 = mapHeight * mapWidth * sizeof(float)
识别输出复制字节数 = steps * classes * sizeof(float)
```

不要将模型文件大小当作输出复制量，也不要未经检查就固定假设类别数。

官方 `GetTensorData<T>()` 返回张量数据指针，不进行类型检查，调用方必须保证实际元素类型匹配。[S3]

可测候选是同步后处理回调或拥有 `Ort::Value` 的小型结果对象：后处理直接读取输出，完成后释放张量，再释放会话。边界包括：

- 检测连通域读取只读概率图，继续保留独立的 `visited` 与栈空间。
- 识别逐时间步 argmax 后，只需保留索引与置信度，再执行现有 CTC 解码。
- 输出类型、维数、非正尺寸与元素总数检查应在读取前完成；不能因改用指针而丢失检查。
- 数据指针不能逃出持有张量的生命周期；若输出分配依赖会话分配器，应先销毁输出再释放会话。
- 不改变浮点数据、比较顺序、blank 处理、字符跨度、置信度和阅读顺序，因此可以进行逐 token 对照。

若采用“复制精简后的 argmax 结果”，需验证最高概率并列时仍保留原实现的第一个类别，避免字符结果变化。

### 4.4 保持 ORT 类型留在插件内部

若采用拥有输出张量的结果对象，建议只在插件内部增加一个职责单一的小模块：

```text
qt/plugins/ocr-rapid/
  rapid_onnx_output.h    仅暴露只读浮点数据、形状、元素数及移动所有权
  rapid_onnx_output.cpp  通过 PImpl 持有 Ort::Value，负责类型检查和释放
  rapid_onnx_session.*   执行推理，将输出所有权移交给结果对象
  rapid_det_model.*      在结果对象有效期间读取概率图
  rapid_rec_model.*      在结果对象有效期间执行原有 argmax
```

这个方案不需要修改插件 SDK，也不需要向应用层公开 ORT 头文件或张量 ABI。实现时应保证：

1. 结果对象只移动、不隐式复制。移交的是 `Ort::Value` 所有权，不是指向 `Run()` 局部输出容器的裸指针。
2. 公开的 `const float *` 只在结果对象有效期间使用，任何 token、信号或异步回调都不能持有该指针。
3. 检测和识别函数的局部输出对象在函数返回前销毁，外层插件清理器随后才销毁会话。这样不需要假定输出所用分配器独立于会话生存。
4. 推理失败时清空结果对象，避免调用方误读上一次输出；使用新输出前验证浮点类型和维数，转换到 `int` 或 Qt 容器长度之前检查边界。
5. 后处理只修改自身小缓冲，不修改 ORT 输出。若识别完成 argmax 后不再需要完整概率图，可缩短输出对象作用域，但不要同时改变 CTC 解码逻辑。

检测概率图通常只有单通道；识别输出包含全部类别分数。先记录实际形状和复制字节数，再决定此改动在总峰值中的优先级。

## 5. 第三轮：图优化、预打包与低优先级选项

### 5.1 先比较 ALL 与 EXTENDED

官方定义：[S4]

| 选项 | 范围 |
| --- | --- |
| `ORT_ENABLE_BASIC` | 基础消除、常量折叠及基础融合 |
| `ORT_ENABLE_EXTENDED` | 包含基础优化，再增加扩展节点融合 |
| `ORT_ENABLE_ALL` | 包含上述优化及布局优化 |

官方列举的 CPU 布局优化为 NCHWc。`ALL -> EXTENDED` 因此比关闭全部图优化更适合隔离布局相关成本，但文档并未保证布局优化必然增加内存。它可能减少计算，也可能改变中间张量布局与工作区；这是需要实测的原因，不是已经证明的根因。

建议分别对 det 和 rec 做选项对照，记录会话构造峰值、首轮推理峰值、同尺寸多次推理延迟与不同尺寸输出。浮点运算组织变化时，不能只检查程序没有异常，应比较框、文本、字符跨度和置信度误差。

### 5.2 关闭预打包不一定省内存

`session.disable_prepacking=1` 可以禁用预打包；默认开启。[S5]

官方说明预打包用于改善部分算子的推理性能。上游 `session_state.cc` 又明确指出，推理场景会在预处理后释放部分原始初始化器，因此不把全部初始化器放入无法逐个释放的大缓冲。[S6][S7]

所以“预打包一定保留原始权重加一份打包权重，关闭就省一份”并不成立。关闭可能降低加载开销，也可能保留更多原始表示、增加每次推理成本。它只应作为单独的诊断实验，不能默认投入使用。

### 5.3 离线优化与 ORT 格式不是本轮必要条件

官方说明每次初始化会话都会执行在线图优化；离线保存优化模型可以减少启动时间，但应使用相同执行提供程序、选项与目标硬件。启用布局优化时还存在 CPU 指令集兼容要求。[S4]

当前每次 OCR 重建会话，离线优化有潜在价值，但同时引入缓存失效、模型版本、运行时版本、平台和硬件兼容性管理，不适合作为第一项内存修复。

v1.29.0 配置头提供 ORT 格式专用的模型字节直接使用和内存映射配置，且对初始化器引用原始字节规定了完整会话生命周期约束。当前加载的是 `.onnx`，不能直接加这些配置就宣称消除 ONNX 权重复制。[S5]

### 5.4 分配器替换只保留为独立候选

官方内存文档介绍共享 arena 与 mimalloc，但性能变化依赖模型和使用方式。Windows 可以构建时启用 mimalloc；Linux 文档建议独立构建并通过 `LD_PRELOAD` 使用。[S13]

这不是本项目已经验证的收益，也不是仅影响 OCR 的局部改动。它会影响进程分配行为、部署方式与回收测量；应在消除重叠会话和输出复制之后再独立评估，不与首轮改动捆绑。

## 6. Qt 生命周期与测量边界

### 6.1 加载器销毁、插件卸载和动态库常驻不同

`QPluginLoader` 官方文档指出：[S8]

- 没有显式调用 `unload()` 时，加载器析构后插件仍可保留到程序终止。
- 同一插件存在多个加载器时，只有每个实例都请求卸载才可能完成卸载。
- 根插件对象不会随着加载器析构自动删除；应遵循加载器卸载机制，不能随意删除根对象。
- 自 Qt 5.7 起，默认包含 `QLibrary::PreventUnloadHint`。

`QLibrary` 对 `PreventUnloadHint` 的解释是防止动态库从地址空间卸载，后续打开时静态变量不重新初始化。[S9]

因此，模型会话释放后仍有 ORT 或插件库映射，并不能单独证明模型权重仍然存在。测量应将动态库文件映射与匿名私有内存分开。

清除 `PreventUnloadHint` 不是低风险回收开关。只有确认没有运行中的插件调用、线程入口、回调、插件类型对象与待执行任务，才可单独验证卸载。应检查 `unload()` 返回值，并区分“请求卸载成功”和“所有相关映射消失”；不要为降低截图读数破坏后台 OCR。

### 6.2 `waitForDone()` 会改变被测状态

Qt 文档明确，`QThreadPool::waitForDone()` 等待线程退出并移除线程池中的所有线程。默认空闲线程过期时间为 30000 毫秒，线程池本来用于复用线程、减少创建成本。[S10]

内存实验应区分：

| 时点 | 应记录的状态 |
| --- | --- |
| OCR 提交前 | 插件是否已加载，是否有前序任务，实际线程数 |
| det 加载后与检测峰值 | 活跃模型、输入尺寸、匿名内存、线程数 |
| rec 加载后与识别峰值 | 文本行数、最长行输入宽度、输出形状 |
| OCR 返回后 | 会话已释放，但应用工作线程可能仍存活 |
| 固定空闲时间后 | 正常过期机制是否已经触发，是否调用主动回收 |
| 进程退出前 | 仅用于退出收尾检查，不替代交互中的常驻内存 |

不能把调用 `waitForDone()` 后的结果与未清空线程池的旧版结果直接比较。若主动回收是生产行为，应两版同样启用并报告回收前后；若只用于归因，则应单列为诊断数据。

### 6.3 Qt 6.12 图像预处理使用独立私有线程池

补充核验针对主测量中出现的线程序列：独立 OCR 进程开始时 1 个线程，检测会话加载后 8 个，图像预处理结束且第一次 `Run()` 尚未调用时 16 个，检测会话释放后 9 个，识别会话加载后 16 个，最终释放后 9 个。该序列由主测量提供，本文没有重复执行基准。

Qt v6.12.0 官方源码给出了直接匹配的调用链：[S14][S15][S16][S17]

```text
QImage::scaled(..., SmoothTransformation)
  -> QImage::transformed / smoothScaled
  -> qSmoothScaleImage
  -> multithread_pixels_function
  -> QGuiApplicationPrivate::qtGuiThreadPool()

QImage::convertToFormat
  -> 对应格式转换器；部分格式使用 convert_generic 等通用转换器
  -> QGuiApplicationPrivate::qtGuiThreadPool()
```

不能笼统认为每次 `convertToFormat` 都创建工作线程：转换器按格式和平台选择，同格式转换可以直接返回共享图像。大图平滑缩放及通用转换路径已足以触发上述私有池。

`qtGuiThreadPool()` 的已核验实现：[S14]

- 保存一个受互斥量保护的静态 `QPointer<QThreadPool>`，按需 `new QThreadPool()`；没有调用 `globalInstance()`。
- 初始最大线程数若大于 4，则设置为 `qBound(4, maxThreadCount() / 2, 8)`。初始为 16 时上限为 8，而不是再创建 16 个线程。
- 创建池对象与实际启动线程不同；分段任务调用 `start()` 时，工作线程才按需求建立。
- 该函数没有覆盖默认空闲过期时间，因此沿用 `QThreadPool` 的 30000 毫秒默认值。`QGuiApplication` 析构时显式等待并删除这个池。[S10][S14]

缩放分段数为 `min(sourceWidth * sourceHeight / 65536, destinationHeight)`。只有分段数大于 1、池存在，而且当前线程不属于该池时才分派任务；否则在当前线程直接执行同一缩放区间函数。通用格式转换采用类似分段与防嵌套条件。[S15][S16]

这解释了为何全局 `QtConcurrent` 工作线程或应用专用 OCR 工作线程不能自动消除这 8 个线程：它们不属于 QtGui 私有池，无法满足 `threadPool->contains(QThread::currentThread())`。

在普通 ORT 线程池构建下，观测值可分解为：`1 + ORT 7`，预处理后为 `1 + ORT 7 + QtGui 8`，释放会话后为 `1 + QtGui 8`。它与源码吻合，但仍应通过独立进程对照或线程创建栈确认具体二进制，不能仅靠数字命名线程来源。

### 6.4 环境开关是一次缓存的进程级诊断条件

Qt v6.12.0 源码包含：[S14]

```cpp
const static bool runtime_disable = qEnvironmentVariableIsSet("QT_NO_GUI_THREADPOOL");
```

因此：

- `QT_NO_GUI_THREADPOOL` 只检查变量是否存在，设置为 `0` 同样禁用。
- 结果在首次调用时缓存；OCR 开始前临时设置、结束后恢复不能可靠地限制单个任务，也不是线程局部配置。
- 小图操作也可能先调用池获取函数，再判断是否需要并行，所以“此前没有额外线程”不代表该环境条件尚未缓存。
- 该开关控制 QtGui 私有池，影响其他图像转换和绘制辅助路径，不能当作仅限制 OCR 的产品设置。

在独立、同条件的测量进程启动前设置 `QT_NO_GUI_THREADPOOL=1`，可检验预处理是否不再增加 8 个线程；保持同一 `SmoothTransformation` 调用及模型，并继续比较像素、token、延迟和内存。此实验验证来源，不等于建议生产应用全局关闭该池。

构建配置还定义 `qtgui-threadpool` 特性，标为 `PRIVATE`，用途为多线程图像变换及 `QPainter` 填充。[S19] 改变 Qt 构建特性同样不是插件单次调用控制。上述环境变量已在固定版本源码核验，但未在所核验的公开 `QImage` API 中作为稳定配置接口列出。[S18]

### 6.5 没有满足全部约束的公开单次调用开关

公开 `QImage::scaled` 参数控制目标尺寸、纵横比和变换模式；`convertToFormat` 参数控制目标格式及转换标志。它们没有调用方线程池或每次调用线程数参数。[S18]

在“不修改应用或 QtGui 全局池、不依赖私有 ABI、不改变重采样结果”三个约束下，不能声称存在一个现成的 Qt 6.12 调用选项。候选及边界如下：

| 候选 | 作用与边界 |
| --- | --- |
| 将 OCR 放入应用专用单工作线程队列 | 可以限制任务和解码并发；不能阻止 QtGui 私有池为一次预处理分派任务 |
| 通过私有接口把预处理投递到 QtGui 池 | `contains()` 可阻止嵌套并行，同一缩放算法在该线程执行；但依赖 `GuiPrivate` ABI，任务仍与其他 QtGui 操作共享池，不作为发行默认方案 |
| 分条带只做格式转换 | 可在限定格式、没有跨像素或跨行状态的转换中减少并行及临时空间；必须逐像素对照，不能推广到全部抖动和调色板转换 |
| 裁成条带后分别平滑缩放 | 会改变取样坐标、边缘和舍入条件，不能宣称与全图缩放等价 |
| 先逐行缩放、再逐列缩放 | 可能避免大图分段，但中间量化与两次舍入不一定等于原二维实现；还可能增加中间图像和调用开销 |
| 插件内部同步预处理实现 | 可以局部控制线程与缓冲，但需要保留原采样和 alpha 语义，并通过固定像素参考及 OCR 对照；这是算法实现工作，不是配置开关 |

`QImage` 的大图缩小源码使用面积采样路径；改成 `FastTransformation`、普通双线性绘制或缩小模型输入不能视为只改线程、不改质量。[S17] 如要引入本地实现，应独立评估，不与模型生命周期修复同时改变。

### 6.6 为什么不应先归因于 ORT 懒创建第二个线程池

ORT v1.29.0 `InferenceSession::ConstructorCommon` 根据会话选项创建 intra-op 池，仅在 `ORT_PARALLEL` 模式创建 inter-op 池。[S20] 当前探针在首次 `Session::Run()` 前就观察到新增线程，而这段调用链执行的是 Qt 转换、缩放和应用自己的输入填充。

该时序不支持“首次 `Run()` 懒创建第二池”的解释；Qt 源码又提供了数量和生命周期匹配的来源。仍不能仅凭源码排除任意执行提供程序或发行版依赖的后台行为。最小证据闭环是：相同 ORT 配置、模型和预处理，在新进程中只改变 Qt 环境条件，检查线程数差异；必要时记录线程创建栈。无需先修改全局 ORT 或应用线程参数。

## 7. 实验矩阵与验收

所有实验保持相同程序构建类型、同一插件、相同运行时与模型、相同合成图片、相同窗口尺寸及等待时点。保留自动后台识别、拖动选区和文字选择，不通过关闭功能或更换小模型制造收益。

| 优先级 | 独立实验 | 必须观察 | 主要风险或取舍 |
| --- | --- | --- | --- |
| 1 | det 与 rec 分阶段加载和释放 | 各阶段峰值、总时长、加载失败路径、空白图 | 环境重建耗时；延迟加载改变损坏 rec 的错误时机 |
| 1 | intra-op 线程数 1、2、4、当前值 | 实际线程数、检测和识别分段时长、UI 延迟 | 低线程数可能降低吞吐；不同运行时构建控制方式不同 |
| 2 | 直接消费 ORT 输出，取消完整浮点输出复制 | 输出字节数、复制期间峰值、完整 token 对照 | 张量生命周期、类型和越界检查 |
| 2 | det 或 rec 单独从 ALL 改为 EXTENDED | 构造与推理分段耗时、峰值、数值及文本结果 | 不保证省内存，可能降低计算效率 |
| 3 | 共享任务级环境，必要时单独比较共享线程池 | 环境初始化成本、结束后线程寿命 | 首次环境创建规则；全局池可能延长常驻时间 |
| 3 | 单独关闭预打包 | 原始及打包权重、加载耗时、每行识别耗时 | 可能保留更多原始权重，不能假定节省 |
| 4 | arena 加收缩、离线优化或更换分配器 | 完整多轮工作负载与部署约束 | 配置复杂、版本敏感，不应与前述改动同时引入 |

正确性样本至少包含：无文本、单行、密集多行、长行、中英混合、不同截图尺寸、透明图像，以及检测或识别加载失败。线程与队列验证包含连续提交、取消、关闭贴图后任务完成和应用退出。

对线程数、图优化和预打包改动，除文本外还应对比置信度误差和字符框；微小概率变化可能在阈值或最高类别接近时放大成可见行为变化。

本文没有执行上述基准。减少同时存活对象和复制字节数是代码层可验证事实；整个应用的 RSS、私有内存及响应时间收益，仍以相同条件下的重复测量为准。

## 8. 证据来源与复现命令

所有关键网页均通过 `smart-search fetch` 获取正文，再形成本文结论。Context7 用于库与源码发现，不以其生成摘要代替官方页面。取证时主搜索接口出现 TLS 错误；Context7、Exa、Tavily 检查通过，本轮没有使用主搜索回答或其他搜索工具替代证据。

主要命令：

```sh
smart-search doctor --format json
smart-search deep \
  'Deep research: ONNX Runtime CPU arena, memory pattern, graph optimization and per-session thread pools; Qt plugin unload lifetime. Need evidence-backed safe memory optimizations for two serial PP-OCR det/rec CPU sessions recreated per OCR request without disabling asynchronous OCR.' \
  --format json --output /tmp/mark-shot-memory-research-plan.json
smart-search context7-library 'ONNX Runtime' --format json
smart-search context7-docs '/microsoft/onnxruntime' \
  'DisableCpuMemArena DisableMemPattern memory pattern cache fixed input shape CreateTensorWithDataAsOrtValue buffer lifetime SetIntraOpNumThreads DisablePerSessionThreads' \
  --format json --output /tmp/mark-shot-ort-context7-docs.json
smart-search fetch 'https://onnxruntime.ai/docs/performance/tune-performance/threading.html' \
  --format markdown
smart-search fetch 'https://onnxruntime.ai/docs/get-started/with-c.html' --format markdown
smart-search fetch 'https://onnxruntime.ai/docs/api/c/struct_ort_api.html' --format markdown
smart-search fetch 'https://doc.qt.io/qt-6/qpluginloader.html' --format markdown
smart-search fetch 'https://doc.qt.io/qt-6/qthreadpool.html' --format markdown
```

- [S1] [ORT Thread management](https://onnxruntime.ai/docs/performance/tune-performance/threading.html)：线程数量含义、默认顺序执行、显式数量与亲和策略、自旋及共享池。
- [S2] [ORT C API：OrtApi](https://onnxruntime.ai/docs/api/c/struct_ort_api.html)：`CreateEnv`、`EnableMemPattern`、`DisableCpuMemArena`、`CreateTensorWithDataAsOrtValue`、`SetIntraOpNumThreads`、`DisablePerSessionThreads`。
- [S3] [ORT C++ API：Ort::Value](https://onnxruntime.ai/docs/api/c/struct_ort_1_1_value.html)：调用方缓冲张量构造与 `GetTensorData` 指针语义。
- [S4] [ORT Graph Optimizations](https://onnxruntime.ai/docs/performance/model-optimizations/graph-optimizations.html)：各优化级别、NCHWc、在线与离线优化及兼容要求。
- [S5] [ORT v1.29.0 会话配置键](https://github.com/microsoft/onnxruntime/blob/v1.29.0/include/onnxruntime/core/session/onnxruntime_session_options_config_keys.h)：预打包、共享分配器与 ORT 格式字节生命周期；已获取对应 raw 正文。
- [S6] [ORT v1.29.0 session_state.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/framework/session_state.cc)：初始化器分配、预打包及原始初始化器释放；已获取对应 raw 正文。
- [S7] [ORT C/C++ 入门：Features](https://onnxruntime.ai/docs/get-started/with-c.html#features)：独立线程池、共享分配器、arena 配置与收缩、共享初始化器及预打包权重。
- [S8] [Qt QPluginLoader](https://doc.qt.io/qt-6/qpluginloader.html)：加载器析构、根对象、卸载引用与默认加载提示。
- [S9] [Qt QLibrary::LoadHint](https://doc.qt.io/qt-6/qlibrary.html#LoadHint-enum)：`PreventUnloadHint` 的动态库映射与静态变量语义。
- [S10] [Qt QThreadPool](https://doc.qt.io/qt-6/qthreadpool.html)：线程复用、默认空闲过期与 `waitForDone()` 移除线程。
- [S11] [ORT v1.29.0 allocator.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/framework/allocator.cc)：普通 CPU 分配器使用底层对齐分配和释放；已获取对应 raw 正文。
- [S12] [ORT v1.29.0 运行配置键](https://github.com/microsoft/onnxruntime/blob/v1.29.0/include/onnxruntime/core/session/onnxruntime_run_options_config_keys.h)：CPU arena 收缩要求保持 arena 启用；已获取对应 raw 正文。
- [S13] [ORT Reduce memory consumption](https://onnxruntime.ai/docs/performance/tune-performance/memory.html)：共享 arena 与 mimalloc 的使用范围。
- [S14] [Qt v6.12.0 qguiapplication.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/kernel/qguiapplication.cpp)：`qtGuiThreadPool()` 的独立池、线程上限、环境条件及应用析构清理；已获取对应 raw 正文。
- [S15] [Qt v6.12.0 qimagescale_p.h](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/painting/qimagescale_p.h)：`multithread_pixels_function` 的分段数、私有池和防嵌套条件；已获取对应 raw 正文，并与本机安装头文件对照。
- [S16] [Qt v6.12.0 qimage_conversions.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/image/qimage_conversions.cpp)：通用格式转换使用 QtGui 私有池，并按格式选择转换器；已获取对应 raw 正文。
- [S17] [Qt v6.12.0 qimagescale.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/painting/qimagescale.cpp) 与 [qimage.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/image/qimage.cpp)：`smoothScaled` 调用、格式规范化及面积采样；已获取对应 raw 正文。
- [S18] [Qt QImage](https://doc.qt.io/qt-6/qimage.html)：公开缩放、格式转换参数与语义；已获取正文。
- [S19] [Qt v6.12.0 GUI configure.cmake](https://github.com/qt/qtbase/blob/v6.12.0/src/gui/configure.cmake)：私有 `qtgui-threadpool` 构建特性；已获取对应 raw 正文。
- [S20] [ORT v1.29.0 inference_session.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/session/inference_session.cc)：会话构造中的线程池创建及顺序、并行模式条件；已获取对应 raw 正文。

## 9. 追加核验：会话释放、动态库卸载与子进程隔离

本节补充后续主测量结果，修正线程来源与内存主因之间的区别；不改变前述固定版本源码事实。数值来自主测量，本文没有独立重复执行基准或实施进程架构。

### 9.1 45 秒结果否定“QtGui 空闲线程是主要差额”

| 完整入口场景 | 固定空闲阶段线程数 | 空闲 RSS |
| --- | --- | ---: |
| 旧版、不执行自动 OCR | 对照入口记录 | 58.54 MiB |
| 分阶段加载模型 | 起初 10 个，约 30 秒后降为 2 个 | 90.95 MiB，线程退出前后没有明显变化 |
| 分阶段加载，进程启动前设置 `QT_NO_GUI_THREADPOOL=1` | 2 个 | 90.46 MiB |

关闭 QtGui 私有池的空闲 RSS 差约为 0.49 MiB，远小于新版相对旧版约 32 MiB 的差额。因此，第 6.3 节的线程数量归因成立，但它不是这次空闲内存主要差额的解释，不能继续将全局禁用 QtGui 池当作主要修复。

同次 `smaps` 分类显示，当前 ORT 文件映射驻留约 15.39 MiB，大部分记为 `Private_Clean`；匿名驻留约 29.06 MiB，旧版约 15.16 MiB，匿名差约 13.90 MiB。这些结果把剩余调查分成了文件映射与匿名资源两部分，不能把全部差额笼统称为“模型还没有释放”。

`Private_Clean` 是页的共享和脏页状态分类，不等于 malloc 堆，也不代表文件映射无法回收。一个暂时只有此进程使用的干净文件页可以记入此项；必须同时查看映射路径、RSS、PSS 与匿名标记。[S26]

旧版没有自动 OCR，新版保留自动 OCR，所以两版不是等量推理工作负载。目标应分别表述为降低推理峰值、减少推理后额外常驻，以及保持交互与识别质量，而不是要求推理期间无条件回到旧版 58 MiB。

### 9.2 峰值计量修正

此前完整入口工具每 50 毫秒采样 RSS；其最大值是采样所见最大值，不能保证捕获短暂峰值。主测量已增加 `peak_root_hwm`，读取内核维护的 `VmHWM`，并与采样值分列。

| 完整入口记录 | 根进程 `VmHWM` |
| --- | ---: |
| 当前版本，第一轮 | 207.26 MiB |
| 当前版本，第二轮 | 206.97 MiB |
| 当前版本，第三轮 | 210.85 MiB |
| 分阶段模型加载，45 秒观察 | 185.73 MiB |

这些值替代“采样约 166 MiB 就是完整入口真实峰值”的说法。`VmHWM` 是内核维护的进程 RSS 高水位，而不是全部进程、全部内存类型或 cgroup 的总峰值；Linux 手册还提示 `/proc/PID/status` 中相关 RSS 统计存在精度限制。[S32]

独立插件接口实验使用其系统高水位；其中 ARGB 图像视图实验报告约 138 MiB，并且完整 token 对照一致。它与完整应用入口的进程基线、窗口和资源生命周期不同，不能直接拿 138 与完整入口 185.73 或 210.85 比较算法收益。

### 9.3 ORT 中几个名字相近的对象具有不同寿命

| 对象或状态 | 已核验的生命周期 | 能否解释 Session 释放后仍有资源 |
| --- | --- | --- |
| 会话自己的 intra-op、inter-op 池 | `InferenceSession` 以 `unique_ptr` 持有默认池；POSIX 线程对象析构会 `pthread_join` | 不支持把正常销毁后的全部剩余线程归为该会话线程池；外部池和全局池另论 |
| C API 的 `OrtEnv` 单例 | `p_instance_` 配合引用计数；正常运行中最后一次 `Release` 置空并删除实例 | 仍有引用时可以存活，但不是无条件永久驻留 |
| `onnxruntime::Env::Default()` | POSIX 返回函数局部静态 `PosixEnv`；它不是 `Ort::Env` 包装的同一个概念 | 可跨多个 C API 环境创建、销毁周期存在 |
| 算子 schema 注册状态 | `environment.cc` 使用全局 once flag 和 `std::call_once` 注册 `OpSchemaRegistry` | 普通 `Environment` 析构没有反注册 schema，不能预期销毁会话等于撤销整个库初始化 |
| C++ API 表指针 | 头文件中的函数局部静态 `OrtApi *` 缓存 | 不是模型权重，也不是动态库所有权；真卸载后的旧指针具有生命周期风险 |

这些行为分别由 ORT v1.29.0 环境源码、会话头、POSIX 环境和 C++ API 头确认。[S21][S22][S30]

正常 `OrtEnv::Release` 在引用计数归零时删除实例。源码只在进程关闭标志已设置时存在避免正常清理的分支；不能把该关闭分支当作日常 OCR 后空闲内存的既定原因。[S21]

启用 CPUINFO 的构建中，静态 `PosixEnv` 构造调用 `cpuinfo_initialize()`，静态析构才调用 `cpuinfo_deinitialize()`。这证明部分平台状态跨会话存在，但不证明它占用了多少 MiB。schema 注册和平台单例具体贡献仍需分配追踪。[S30]

此外，代码与只读数据页、普通分配器尚未交还系统的空闲页、外部执行提供程序和应用图像都可能与会话寿命不同。不能仅凭“调用过 reset”认定剩余内存一定泄漏，也不能仅凭这些机制存在就认定差额全部正常。

### 9.4 Qt 的卸载提示必须在关联文件之前处理

Qt v6.12.0 默认 `QPluginLoader` 使用 `PreventUnloadHint`。Unix 实现将它映射为 `RTLD_NODELETE`；对应动态库即使经历成功的 `dlclose()` 也可能继续映射，静态变量不会因为再次打开就重新初始化。[S23][S24]

本轮查到一个影响诊断有效性的约束：Qt 只允许在加载器尚未关联文件时清除提示；设置文件名后，新提示与已有提示进行 OR 合并，已经加载后修改无效。同一动态库的加载提示还在加载器之间共享。[S9][S23]

因此，`QPluginLoader(path)` 之后再清空提示，不是可靠的取消默认防卸载方式。串行卸载实验应在独立新进程中按以下顺序初始化，并确认此前无人默认加载同一个库：

```cpp
QPluginLoader loader;
loader.setLoadHints({});
loader.setFileName(pluginPath);
```

这只是确保实验没有主动设置该提示，不保证 ELF 加载器最终移除全部依赖库。

Linux `dlclose` 文档明确，返回成功不保证符号离开地址空间；其他直接或依赖引用仍会保留库。[S24] glibc 2.42 的卸载判断还检查 `l_nodelete_active`、未完成的 TLS 析构计数以及依赖关系。[S31]

GCC 官方说明，`STB_GNU_UNIQUE` 用于保证模板静态数据和 inline 函数局部静态变量唯一，但会导致受影响 DSO 的 `dlclose` 被忽略。`-fno-gnu-unique` 是编译选项，不是对已安装 ORT 的运行时补丁；也不能未经符号检查就认定本机库一定受它影响。[S25]

应同时记录插件与 ORT 各自的映射是否消失，而不是只记录 `unload()` 返回值。检查时优先读取进程映射；若使用 `dlopen(..., RTLD_NOLOAD)` 探测，也要正确配对关闭，避免探针本身额外持有引用。[S24]

### 9.5 并发任务下，“会话已释放”不足以证明可以卸载

Qt `QLibraryPrivate::unload` 在最后一个卸载引用归零时直接删除根插件实例，随后调用系统卸载函数。它的内部互斥用于管理加载器状态，不会等待应用的 `recognize()` 调用，也不会自动获取插件自己的业务互斥量。[S23]

因此即使 `PreventUnloadHint` 让代码继续映射，根对象仍可能已经销毁；工作线程继续访问成员、等待插件互斥量或从方法返回，都可能使用失效对象。仅取消 watcher 连接不等于这些调用已经结束。

正式卸载至少需要应用证明这些条件，而不是从 RSS 或活动线程数猜测：

1. 已停止向该插件接纳新任务，所有已接纳任务都有明确归属；排队任务不会再拿旧接口指针进入插件。
2. 所有进行中调用实际返回，包括正在等待插件内部互斥量的调用；取消 UI 通知不能代替完成确认。
3. 输出张量、会话、环境、插件定义的结果对象和有插件析构函数的容器均已按顺序释放。
4. 没有未来还会执行的插件函数指针、信号回调、线程入口、异步任务或插件类型元对象使用。
5. 遵循根 QObject 的线程与事件生命周期；不能先卸载代码，再等待 `deleteLater` 或排队事件完成。Qt 文档要求跨线程销毁对象时避免并发事件处理。[S29]
6. 卸载完成后清除所有旧接口地址；重新加载必须重新获取实例和接口，不依赖上次指针碰巧有效。

当前“取消后旧任务可能仍等待或执行”的风险尚未解除前，不推荐启用产品级空闲卸载。把加载器析构移到工作线程、给卸载加一个定时器或只观察 watcher 数量，都不能完成上述证明。

### 9.6 `Ort::Env` 重建不等于动态库重新加载安全

当 ORT 共享库一直映射、所有对象按合法顺序释放时，正常 `OrtEnv` 引用归零后重新创建，与真正 `dlclose` 后重载，是不同实验。

ORT v1.29.0 C++ API 头缓存 `OrtApi *`，并明确提醒延迟初始化解决的是动态库和静态初始化顺序问题，不意味着任意静态初始化期间调用 API 都安全。[S22] 若保留该指针的宿主或其他模块仍然加载，而 ORT 真正卸载，旧 API 表地址及依赖它的对象析构可能失效。

不能把“手动重新调用 `InitApi`”当作通用修复：它不能补救已存活对象、旧回调或其他模块里的缓存，也不能替代加载依赖关系管理。最安全的进程内实验边界是插件及相关 ORT 对象都不逃出一次受控加载周期，并检查实际映射，再验证下一次加载的全量结果。

### 9.7 短生命周期 OCR 子进程的收益与代价

以下是方案评估，不是已经实施或实测的优化。

| 维度 | 短生命周期子进程 | 进程内安全卸载插件 |
| --- | --- | --- |
| 结束后的私有资源 | 子进程结束后不再保留其活跃地址空间，不依赖第三方库是否支持反复卸载 | 只有生命周期正确且实际卸载有效才回收相应映射；主分配器和其他引用可能保留内存 |
| 同时峰值 | 主窗口图像仍在，额外存在子进程、输入传输或解码、运行时和模型；可能高于进程内 | 无进程与 IPC 固定开销，但必须管理全部活动调用 |
| 冷启动与连续任务 | 每次启动、动态链接、初始化、模型加载和 IPC 都有成本；连续任务需要测量整体吞吐 | 已有应用环境可复用，但频繁库重载也可能增加初始化成本 |
| 故障和取消 | 推理异常可限制在子进程；父进程仍必须异步回收、处理超时和失败 | 强制终止执行中的线程或卸载其代码不安全，只能等待或采用受支持的协作取消 |
| 部署 | 需要可定位的辅助入口、匹配的运行时和模型、协议及平台打包验证 | 不新增进程协议，但插件和依赖库的卸载行为可能随工具链、平台变化 |
| 对主应用的影响 | 可只在子进程启动环境中控制 QtGui 预处理池，不改变主应用池 | 私有 QtGui 池仍属于主应用，不随 OCR 插件卸载消失 |

子进程确实可以缩短 OCR 私有资源在主应用中的驻留，但这是隔离与资源寿命变化，不能自动称为总内存优化。文件页也可能仍在系统页缓存中，进程消失不等于整台机器对应缓存立即归零。

若主进程为查询可用性仍加载链接 ORT 的插件，随后又启动包含 ORT 的子进程，就可能同时承担两份初始化状态。评估必须测量真实宿主加载路径，而不是假设辅助程序启动后宿主自然不加载 ORT。

输入方案同样影响峰值：传递原始像素可能新增父进程发送缓冲和子进程接收缓冲；传递现有文件会在子进程再次解码；共享内存可以减少部分复制，但需要明确所有权和释放时机。不得使用无界输入队列，或把全部截图编码、缓存后再依次识别。

Qt `QProcess` 提供异步启动和结束信号。同步等待可能冻结主线程，`startDetached()` 又让进程脱离父进程生命周期，不适合以“请求完成或取消后必须回收”为目标的试验。[S28] 在多线程进程内自行 `fork` 后直接执行复杂 Qt/ORT 逻辑也不应作为捷径；应使用正常启动独立程序的路径，而不是绕过受支持的进程创建流程。

### 9.8 总量指标与可证伪实验

多进程评估使用同步采样的主进程加全部子进程指标。相加 RSS 会重复计算共享库页；相加各进程不同时间出现的高水位也不是同一时刻的总峰值。`PSS` 按共享页的使用进程数分摊，适合观察进程组比例驻留，但外部进程共享同一页也会影响分摊结果。[S26]

条件允许时，将父进程与子进程放在同一独立 cgroup：`memory.current` 包含组和后代当前使用量，`memory.peak` 记录其峰值；同时查看匿名、文件缓存和内核内存分类。cgroup 数值不等于 PSS，不能混用口径，也不能只记录根进程 `VmHWM`。[S27]

| 实验 | 保持不变 | 能支持或否定的判断 |
| --- | --- | --- |
| 同一进程创建、释放 Session 与最后 Env 引用，重复多轮 | 不卸载插件，同一图片、模型和线程数 | 若首轮增加后稳定，支持存在初始化或缓存基线；持续增长则继续定位，不能提前判定全是正常缓存 |
| 只卸载、重载一个已排空的串行插件，至少 10 轮 | 新进程内正确清提示；每轮确认无调用及无借用对象 | `unload=true` 但 ORT 映射不消失，直接否定“清提示即可回收全部 ORT”；映射消失但匿名差不降，否定库映射是全部原因 |
| 同步记录映射分类、匿名差、线程数和分配器回收前后 | 不把主动回收结果混入正常交互值 | 区分只读文件页、仍存活对象与分配器空闲页；45 秒线程退出试验已经否定空闲 QtGui 线程是主要差额 |
| 卸载后重新加载并完整识别，检查下一轮结果和内存 | 固定模型和输入，重取全部接口 | 检验合法重载周期；需要同时覆盖失败和取消后的排空，单次成功不证明并发安全 |
| 新建最小辅助进程的独立试验，与进程内版本比较 | 同一预处理、模型、输出检查和任务序列 | 只有同步总 PSS、总私有内存或 cgroup 峰值下降，才能支持总量优化；主进程 RSS 下降不足以成立 |
| 单次、短时间连续和多窗口快速提交三类负载 | 后台 OCR、拖选和文本完整性均保留 | 检查子进程启动成本、队列积压与取消回收；不能只挑长时间空闲场景报告收益 |

串行卸载试验可配合符号检查与加载器诊断；查到 `STB_GNU_UNIQUE` 或仍存活依赖后，应先解释为什么映射保留，而不是在应用中继续增加 `dlclose` 次数或强制解除映射。

### 9.9 最小风险推荐

先保留经过同入口、全 token 对照验证的局部内存优化；完成解码前任务接纳限制和真实取消、排空语义，再讨论插件卸载。旧版无自动 OCR 与新版有自动 OCR 的差额应如实列出，不能通过移除后台识别恢复旧版数字。

不推荐把清除 `PreventUnloadHint`、切换全局线程池或强行卸载 ORT 当作当前产品修复。可以继续执行独立进程内的安全串行卸载实验，用映射和匿名变化验证收益上限；该实验不应绕过生产生命周期问题。

如果动态库确实难以可靠卸载，而目标明确要求 OCR 完成后不再长时间保留其私有资源，短生命周期子进程比强制卸载活动插件更容易建立清晰的资源边界。但仅建议进入独立原型测量，不直接替换现有架构；验收必须同时覆盖总峰值、空闲总量、识别延迟、连续任务和跨平台打包。

## 10. 追加证据

本轮继续使用 `smart-search fetch` 获取官方文档或固定版本源码；没有修改生产代码。主要新增取证命令示例：

```sh
smart-search fetch \
  'https://raw.githubusercontent.com/microsoft/onnxruntime/v1.29.0/onnxruntime/core/session/ort_env.cc' \
  --format markdown
smart-search fetch \
  'https://raw.githubusercontent.com/qt/qtbase/v6.12.0/src/corelib/plugin/qlibrary.cpp' \
  --format markdown
smart-search fetch 'https://man7.org/linux/man-pages/man3/dlopen.3.html' --format markdown
smart-search fetch 'https://gcc.gnu.org/onlinedocs/gcc/Code-Gen-Options.html' --format markdown
smart-search fetch 'https://docs.kernel.org/admin-guide/cgroup-v2.html' --format markdown
```

- [S21] [ORT v1.29.0 ort_env.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/session/ort_env.cc)：`OrtEnv` 引用计数、正常销毁与进程关闭分支；已获取 raw 正文。
- [S22] [ORT v1.29.0 inference_session.h](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/session/inference_session.h) 与 [onnxruntime_cxx_api.h](https://github.com/microsoft/onnxruntime/blob/v1.29.0/include/onnxruntime/core/session/onnxruntime_cxx_api.h)：会话线程池所有权、缓存 API 表及静态初始化限制；已获取 raw 正文。
- [S23] [Qt v6.12.0 qlibrary.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qlibrary.cpp)、[qlibrary_unix.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qlibrary_unix.cpp) 与 [qpluginloader.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qpluginloader.cpp)：提示合并、根实例删除、系统卸载和 `RTLD_NODELETE`；已获取 raw 正文。
- [S24] [Linux man-pages：dlopen、dlclose](https://man7.org/linux/man-pages/man3/dlopen.3.html)：引用计数、依赖关系、`RTLD_NODELETE` 与成功关闭不保证解除映射。
- [S25] [GCC Code Generation Options](https://gcc.gnu.org/onlinedocs/gcc/Code-Gen-Options.html)：`-fno-gnu-unique`、`STB_GNU_UNIQUE` 与 DSO 卸载影响。
- [S26] [Linux 内核 proc 文档](https://docs.kernel.org/filesystems/proc.html)：映射 RSS、PSS、共享与私有页分类及分摊定义。
- [S27] [Linux 内核 cgroup v2 文档](https://docs.kernel.org/admin-guide/cgroup-v2.html)：`memory.current`、`memory.peak` 与后代进程计量范围。
- [S28] [Qt QProcess](https://doc.qt.io/qt-6/qprocess.html)：异步进程生命周期、阻塞等待和脱离式启动语义。
- [S29] [Qt Threads and QObjects](https://doc.qt.io/qt-6/threads-qobject.html)：对象线程归属、跨线程访问、销毁与事件循环约束。
- [S30] [ORT v1.29.0 environment.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/session/environment.cc) 与 [POSIX env.cc](https://github.com/microsoft/onnxruntime/blob/v1.29.0/onnxruntime/core/platform/posix/env.cc)：schema 一次注册、平台静态实例、CPUINFO 生命周期及线程 join；已获取 raw 正文。
- [S31] [glibc 2.42 dl-close.c](https://github.com/bminor/glibc/blob/glibc-2.42/elf/dl-close.c)：NODELETE、TLS 析构计数及依赖保留判断；已获取 raw 正文，不假定本机恰好使用此 glibc 版本。
- [S32] [Linux man-pages：proc_pid_status](https://man7.org/linux/man-pages/man5/proc_pid_status.5.html)：`VmHWM` 定义及相关 RSS 计量精度提示。

## 11. 追加诊断：调用线程退出后可以实际卸载 ORT

### 11.1 两条受控路径的十轮结果

本轮读取并核对了以下原始记录：

- `build/memory-regression/results/ocr-unload.json`：主线程执行识别。
- `build/memory-regression/results/ocr-unload-thread.json`：主线程创建插件实例，短生命周期 `std::thread` 持有本次 image、tokens 并执行识别，`join()` 后由主线程卸载插件。

两条路径均使用非 GUI 的独立 `ocr_unload_probe`、相同 1080p 输入，每条执行十轮。加载器按“空构造、清提示、设置路径”顺序初始化；插件和 `libonnxruntime` 动态符号表中的 `UNIQUE` 数量均为 0。符号计数来自本轮主诊断，不将它推广到其他发行包。

| 指标 | 主线程识别 | 短线程识别，join 后卸载 |
| --- | ---: | ---: |
| 十轮 `unload()` 返回值 | 全部 `true` | 全部 `true` |
| 卸载后 `ort_mapped` | 十轮全部 `true` | 十轮全部 `false` |
| 每轮 token 数量 | 全部 262 | 全部 262 |
| 首轮卸载并 trim 后 RSS | 52.93 MiB | 26.81 MiB |
| 第十轮卸载并 trim 后 RSS | 53.58 MiB | 28.08 MiB |
| 首轮卸载并 trim 后 `heap_in_use` | 11.75 MiB | 0.64 MiB |
| 第十轮卸载并 trim 后 `heap_in_use` | 11.83 MiB | 0.78 MiB |

表中首末值按 JSON 字节数除以 `1024 * 1024`，保留两位小数。`heap_in_use` 是该探针记录的分配器指标，不等于全部匿名驻留，更不是整个进程内存。

这些是卸载并主动 trim 后的结果，不是完整应用正常空闲值。非 GUI 探针没有完整窗口和应用生命周期，不能将 26.81 MiB 当作贴图程序可以达到的总常驻内存，也不能把与主线程探针的差直接当成生产优化幅度。

**本轮只验证 token 数量，没有比较完整文本、坐标、字符跨度及置信度。** 两条路径均为 262 tokens 不足以证明结果完全一致，也不替代错误、取消和重载正确性测试。

`ocr-unload-debug.json` 是另一次加载器诊断记录，当前记录中的 token 数量为 0，不并入这两条 262-token 对照。

### 11.2 可以得出的结论与尚不能得出的结论

本轮支持以下结论：

1. 本机这一组二进制在受控条件下能够实际解除 ORT 映射；“ORT 无论如何都不能卸载”不成立。
2. 调用线程及相关对象的生存范围会影响本次卸载结果。会话返回与调用线程真正退出不是同一个资源边界。
3. `unload=true` 不能证明 ORT 已解除映射，两条路径再次直接验证了这一点。
4. 仅凭主线程路径的结果就把问题归为 `STB_GNU_UNIQUE` 不成立：本轮插件和 ORT 动态符号检查均为 0。

但这仍不能定位某一个 TLS 对象、TLS 析构注册者或具体分配点。调用线程、局部对象范围及分配器上下文随实验一起变化；要区分这些因素，需要进一步受控验证。glibc 的未完成 TLS 析构计数是可解释这一现象的候选机制，不是已经通过此次 RSS 和映射记录证明的唯一原因。[S31]

`LD_DEBUG` 显示 `libprotobuf-lite` 存在 NODELETE 状态，但不能据此推出依赖它的 ORT 必须常驻。glibc 保留仍需要对象的依赖，不等于沿相反方向保留所有使用该依赖的库；短线程路径中 ORT 映射消失也直接否定了这一必然推论。[S31]

### 11.3 当前注册器的生产边界仍不支持直接卸载

主代码审阅发现，现有注册器按默认防卸载方式保留插件，并向任务提供借用的裸接口指针；状态和元信息中还可能保存来自插件的 `QStringLiteral`。受控探针没有覆盖这些长期存活引用，不能直接把其 `unload()` 调用移入生产注册器。

Qt 官方说明，`QStringLiteral` 在编译期生成数据，并将数据放入目标文件只读段；`QString` 按值复制通常采用隐式共享，不保证复制字符数据。[S33] 因此，“状态结构已经复制了一个 QString”不证明其中字符脱离插件映射。插件真卸载后，后续界面读取或比较这种字符串仍可能访问失效地址。

若未来引入卸载边界，需要在库仍有效时把确实跨边界保留的数据复制到宿主拥有的存储。例如，Qt 文档明确说明由 `QChar` 数组构造的普通 QString 会深复制字符；不能用 `QStringView`、`fromRawData` 或单纯赋值冒充所有权转移。[S33] 这一点同样适用于其他借用视图和包含插件定义析构函数的对象。

生产层还必须完成第 9.5 节的任务停止接纳、真实排空、旧接口清除、对象线程归属与回调生命周期检查。对某个工作线程执行 `join()`，只能证明该线程结束，不能证明注册器、其他任务或界面状态不再引用插件。

### 11.4 后续可证伪实验，不代表已实现方案

| 实验方向 | 隔离条件 | 必须观察 |
| --- | --- | --- |
| 长期工作线程与短线程对照 | 相同线程入口与对象作用域；前者保持空闲存活，后者完成后 join；卸载都在主线程执行 | 映射保留是否随线程退出改变；将线程退出与仅释放 image、tokens 区分 |
| 查明 TLS 注册来源 | 在独立诊断进程追踪 TLS 析构注册、执行和对应 DSO，不将调试器开销混入性能数据 | 能否将保留 ORT 的条件对应到具体注册栈；没有此证据前不命名“泄漏对象” |
| 宿主状态所有权回归 | 在卸载前清除借用接口、深复制必要元信息，覆盖状态显示与错误信息路径 | 卸载后宿主仍可安全读取自身数据；不能以文本恰好仍可读取作为内存所有权证明 |
| 独立推理模块生命周期 | 宿主 Qt provider 保持加载，模型与 ORT 对象限制在内部可卸载模块及短线程范围 | 内部模块是否真正卸载，是否还有直接链接依赖和跨模块数据，完整 token 是否一致 |
| 显式动态加载 ORT C API | 不让长期宿主直接链接并持有 ORT；按模块句柄获取 API，全部对象释放和线程结束后再关闭 | 每轮重新获取有效 API，清除旧地址；对比实际映射、总峰值、耗时及错误路径 |

独立推理模块可能避免反复销毁注册器持有的 Qt 插件对象，但它不是仅移动几个类：需要明确跨模块接口、结果所有权、分配与释放归属，以及 Windows、Linux 等平台的加载和打包行为。

显式加载 ORT 也不等于全局调用一次 `Ort::InitApi` 就安全。若继续使用会缓存 API 表的 C++ 包装，必须证明缓存与库句柄寿命一致；复制 API 表本身也不能延长其中函数地址的有效期。[S22] 这些方向均未实现，不能计入本次已完成优化。

现阶段最低风险结论仍是：保留经过完整正确性对照的局部优化，先建立生产任务与数据所有权边界。本轮成功卸载结果值得继续调查，但尚不足以替代全量 token 对照、并发取消、重复重载和完整应用内存测量。

### 11.5 补充来源

- [S33] [Qt QString](https://doc.qt.io/qt-6/qstring.html#QStringLiteral)：`QStringLiteral` 的只读段存储、QString 隐式共享和从 `QChar` 数组构造时的深复制语义；已通过 `smart-search fetch` 获取正文。
- 本地受控记录：`build/memory-regression/results/ocr-unload.json`、`build/memory-regression/results/ocr-unload-thread.json`；本轮逐项检查十轮记录中的 token 数、卸载返回值、ORT 映射以及首末 RSS 和 `heap_in_use`，没有重新执行识别。
