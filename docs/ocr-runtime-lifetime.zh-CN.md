# OCR 运行库生命周期与打包边界

本文核验将动态 ONNX Runtime 原型整理为生产实现时的安全条件。最终推荐采用**作用域 C API 表、短生命周期推理线程、常驻 Qt 插件对象**，不采用全局 C++ API 表初始化与清除方案。依据为 ORT v1.29.0、Qt v6.12.0 源码、官方 API 文档和当前打包脚本。

本文只记录设计、证据与验收条件，没有修改生产代码。历史内存归因和串行卸载实验见 [内存回归一手资料](memory-regression-references.zh-CN.md)。

## 1. 结论与已知结果

1. 常驻插件持有注册器接口、Qt 元对象及元信息；一次调用单独持有 ORT 动态库和 API 表。这个边界避免销毁注册器仍在使用的插件对象。
2. 所有 ORT 原生对象必须在推理回调返回前销毁，工作线程必须真正结束并成功 join，之后才能释放运行库句柄。
3. `isAvailable()` 可以只加载库、验证入口和 API 版本，不建立模型或修改全局 API 表。它与推理重叠时，各自持有独立加载器引用。
4. Windows 打包必须将动态 ORT DLL 加入显式依赖根；仅扫描 PE 导入不能发现已经取消静态导入的 ORT。Linux 自动共享库依赖生成也需单独检查。
5. 受控 Linux 测试可以验证每轮解除映射，但正式功能不能要求所有平台或所有并发场景中 `unload()` 都立即移除库。
6. 旧提案中的 `Ort::InitApi(nullptr)` 在 v1.29.0 不会清空缓存；最终采用 C API 后应删除对该行为的依赖。

主测量报告：先前动态加载原型完成三张图片、九次完整 token 对照；完整应用空闲内存约 64.77 MiB；同一插件实例连续十次调用后，每次均观察到 ORT 映射消失。这些是原型证据，不能自动证明本次改写后的 C API 包装、全部异常分支及各平台安装包已经通过验收。

## 2. 建议的职责边界

生产文件名由实现确定，建议按以下职责组织，不把加载、模型算法和平台部署塞入一个文件：

```text
qt/plugins/ocr-rapid/
  rapid_onnx_runtime.*       运行库候选、入口验证、持有 QLibrary、短线程执行
  rapid_onnx_handles.*       绑定本次 OrtApi 的原生句柄与状态释放
  rapid_onnx_session.*       单模型会话、输入输出名称、张量推理
  rapid_det_model.*          检测预处理、后处理，接收本次 API 借用
  rapid_rec_model.*          识别预处理、CTC 解码，接收本次 API 借用
  rapid_ocr_plugin.*         常驻 provider、任务结果与模型组合，不保存原生会话
qt/scripts/
  windows-deploy-runtime.sh  动态 ORT 根与原有递归依赖复制
docs/
  ocr-runtime-lifetime.zh-CN.md
```

作用域关系必须满足：

| 资源 | 所有者 | 最迟释放时点 |
| --- | --- | --- |
| Qt 插件根对象、provider 接口及其字面量 | 现有注册器 | 不参与单次 OCR 运行库卸载 |
| `QLibrary` 与获取到的 API 表借用 | 本次 `RapidOnnxRuntime` | 回调退出、原生对象销毁、工作线程 join 之后 |
| `OrtEnv`、`OrtSession`、张量、状态及其他原生句柄 | 工作线程内的局部对象 | 回调返回之前 |
| ORT 返回的字符串和数据视图 | 对应原生对象或分配器 | 使用完成即释放或复制，不能借用到运行库卸载之后 |
| 输出 token、错误文字、状态值 | 常驻插件或宿主拥有的 Qt 数据 | 可以跨回调返回，不包含 ORT 指针、deleter 或异常对象 |

复制 `OrtApi` 结构或函数指针不会延长函数代码寿命。API 表只能作为持有运行库期间的借用，不能独立充当资源所有者。

## 3. 为什么最终不采用 C++ 全局 API 表

ORT v1.29.0 C++ 头文件的内部逻辑只在参数非空时更新缓存：

```cpp
if (newValue) {
    api = newValue;
}
```

`InitApi(const OrtApi*)` 只是调用该逻辑。因此 `InitApi(nullptr)` 不清除旧地址；它不能用于证明卸载后已经没有 API 缓存。[R1]

如果继续采用 C++ 包装，还必须保证所有翻译单元对 `ORT_API_MANUAL_INIT` 定义一致；头文件对 MSVC 提供了不一致检测，并明确要求全编译单元一致。部分文件启用、部分文件自动初始化可能产生定义不一致或意外直接依赖。[R1]

最终 C API 路线没有上述缓存，不需要 `ORT_API_MANUAL_INIT`，也不需要为初始化、清除这个全局表而添加全局互斥量。但这不等于可以删除任务接纳、取消或插件业务状态所需的同步。

构建验收应同时证明：

- 生产插件不再包含 `onnxruntime_cxx_api.h`，没有 `Ort::InitApi` 或其他隐式 C++ API 表使用。
- 插件只使用 ORT C 头文件及函数表，不再链接会把 ORT 加入导入表的目标。
- Linux 插件的 `DT_NEEDED`、Windows 插件的 PE 导入中没有 ORT；不能只根据 CMake 源文件判断。
- 没有未解析的直接 `OrtGetApiBase` 符号引用；入口只通过加载器解析。
- 常驻插件不再把 `OrtSession`、`OrtEnv`、原生输出对象或绑定它们的 deleter 保存为成员。

不应为这次改写盲目添加 `-fno-gnu-unique`、修改全局可见性或强制重建所有依赖。C API 路线先消除本插件的全局 API 缓存；实际库能否解除映射继续按平台和依赖状态验证。

## 4. C API 加载与原生所有权

### 4.1 入口与版本校验

`OrtApiBase` 通过 `OrtGetApiBase` 获取，`GetApi(ORT_API_VERSION)` 请求编译所用版本；不支持该版本时返回空指针。[R2]

加载顺序应是：持有库、解析入口、按头文件调用约定转换函数指针、获取 `OrtApiBase`、检查所需 API 表，最后才执行创建环境或模型的调用。Windows 尤其不能遗漏 `ORT_API_CALL` 等调用约定；模型路径也继续遵循 C API 的平台字符类型。

错误信息需要区分：候选文件缺失、库或传递依赖加载失败、缺少 `OrtGetApiBase`、版本不支持、模型加载失败。版本字符串和加载器错误应在持有库时复制为插件拥有的文字，不返回 DSO 内部 `const char*`。

`GetApi` 非空只证明版本接口可获取，不证明某个模型、执行提供程序或其依赖一定可用。`isAvailable()` 的成功语义不能扩大成“本次识别保证成功”。

### 4.2 原生句柄清单

每种拥有型句柄都应在创建后立即进入绑定本次 `const OrtApi&` 的 RAII 管理；不能使用指向进程全局表的通用析构器。

| 句柄或缓冲 | 释放规则 |
| --- | --- |
| `OrtEnv*` | `ReleaseEnv`；先销毁关联会话 |
| `OrtSession*` | `ReleaseSession`；先结束推理及输出消费 |
| `OrtSessionOptions*` | `ReleaseSessionOptions` |
| `OrtMemoryInfo*` | `ReleaseMemoryInfo` |
| 输入、输出 `OrtValue*` | `ReleaseValue`；调用方输入像素或浮点缓冲不因此释放 |
| `OrtTensorTypeAndShapeInfo*` | `ReleaseTensorTypeAndShapeInfo` |
| 若使用 `OrtTypeInfo*` 或 `OrtRunOptions*` | 分别调用对应 Release 接口 |
| `OrtStatus*` | 先复制必要错误内容，再 `ReleaseStatus`；每个失败调用都必须处理 |
| 输入、输出名称 `char*` | 使用分配该名称的 allocator 释放 |
| `GetAllocatorWithDefaultOptions` 返回的 allocator | 借用，不得释放 allocator 本体 |

官方明确：默认 allocator 始终返回同一个指针，返回对象不能释放；`SessionGetInputName` 分配的 UTF-8 名称则必须使用传入 allocator 释放。[R3]

同一个错误分支可能同时持有名称、会话、环境和状态。必须依靠局部所有权自动释放，不能只在成功返回路径手动清理。每个输出句柄先初始化为空，后续查询或错误文字转换失败时，已创建资源仍应有释放责任人。

### 4.3 C++ 包装改为 C API 时的易错点

`CreateTensorWithDataAsOrtValue` 的长度参数单位是**字节**。之前 C++ `CreateTensor<float>` 接收元素数，直接复制原参数会造成缓冲长度错误。需要检查乘法和范围后传入 `input.size() * sizeof(float)`。[R3]

该接口借用调用方缓冲，`ReleaseValue` 不释放输入数据。输入 `QVector<float>` 必须存活到同步 `Run()` 返回；不能把当前消除输入复制的行为改成悬空借用。[R3]

`Run()` 不会在模型执行完成前返回，但可以使用内部工作线程。[R3] 输出类型、维数、元素数量及转换为 Qt 长度类型时的边界检查仍须保留。获取输出数据指针前检查浮点类型，后处理结束前不释放输出张量。

本次包装改写不得同时改变通道顺序、归一化、缩放方式、检测阈值、框扩张、argmax 并列处理、CTC blank 处理、字符跨度或阅读顺序。算法不变仍需完整对照，不以 token 数相同代替内容和坐标一致。

## 5. 短线程与异常清理

一次运行中的原生模型对象都在短线程内构造与销毁。线程函数结束后，调用方 join；仅确认回调返回或某个完成标记，不等于线程退出及线程局部清理全部完成。

必须覆盖以下路径：

| 路径 | 所需保证 |
| --- | --- |
| 库或 API 校验失败 | 不启动线程，不调用原生创建接口；本次加载器正常释放 |
| 线程创建失败 | 捕获创建异常；没有任务需要等待，不留下本次库引用或半初始化结果 |
| 环境、会话或张量创建失败 | 状态信息在库仍有效时复制；已经创建的原生对象全部析构 |
| 回调或后处理抛出异常 | 在线程内部捕获；以插件拥有的错误值返回，不让异常逃出线程入口 |
| 正常返回、无文本或部分识别行失败 | 保留已有错误语义，全部原生对象在回调结束前释放 |
| 已创建线程后调用方出现异常 | 仍必须完成 join；不能 detach 后让运行库离开作用域 |
| 重叠的可用性检查或其他持有者仍存在 | 本次只释放自己的加载引用，不把未立即解除映射当作推理失败 |

C++ 工作草案规定，无法启动线程时可抛 `system_error`；线程入口异常未捕获会终止程序，仍为 joinable 的 `std::thread` 析构也会调用 `terminate`。[R4] 当前插件构建默认 C++17，不应未经目标配置确认就依赖 C++20 `std::jthread`。

不应把 `std::exception_ptr`、原生异常对象或原生 `what()` 指针保留到运行库卸载后。C API 错误优先转换成明确错误值；线程内异常也只提取必要内容。错误文字分配本身若失败，兜底路径应保持最小化，不能在清理过程中再次抛出导致线程入口异常逃逸。

join 失败或仍存在活动线程时不能继续卸载；不能用 detach 避开 joinable 检查。RAII 的库析构顺序必须放在线程收尾之后，而非在任意异常展开时先卸载库。

主应用后台任务原本可以等待内部短线程；不要把这次 join 移到 UI 主线程执行，也不要改变现有拖选和后台 OCR 行为。

## 6. QLibrary 引用计数与并发校验

Qt `QLibrary` 是可重入类，同一个实例不应被多个线程无同步地修改。每次运行和每次可用性检查使用各自加载器对象；工作线程只借用有效 API 表，不同时操作持有它的加载器。[R5]

**`QLibrary` 析构不会自动卸载已加载的库。** 官方说明，没有显式 `unload()` 时库可保留到程序终止。因此运行库封装必须对每次成功加载建立配对的显式卸载清理，覆盖入口缺失、API 版本不符、线程创建失败、回调失败和正常结束。`isAvailable()` 同样需要这一清理；尝试下一个候选之前，要释放已加载但校验失败的前一个候选。[R5]

Qt 为同一库管理共享加载状态和卸载引用。只要推理持有自己的加载引用，可用性检查释放其引用不会成为最终卸载；系统加载器也维护库引用计数。[R5][R6][R7]

该保护只覆盖动态库生命周期，不保护应用共享模型状态，也不保证“同名但不同路径”的候选是同一个运行时。候选路径规范化、实际版本记录和每次持有完整作用域仍有必要。

`isAvailable()` 应只验证模型路径、加载入口和 API 表，不调用 `InitApi`、不创建模型、不返回 API 指针。加载动态库仍可能执行其静态初始化，不能宣称这个检查完全没有时间或内存成本；建议加入连续可用性检查与推理并发测试。

默认 `QLibrary` 与默认 `QPluginLoader` 不同：前者不设置防卸载提示，后者默认设置。Qt 对同一文件的加载提示存在共享和合并规则，已有其他调用者设置防卸载时，本次不能强行覆盖。[R5]

Linux 受控测试可以观察实际映射；生产日志区分“释放本次引用”“卸载接口返回值”和“映射仍存在”，不添加必须每次解除映射才能返回成功的跨平台断言。Windows `FreeLibrary` 同样依赖加载引用计数，其他模块持有引用时库会保留。[R6][R7]

## 7. 搜索顺序与版本兼容

### 7.1 正常路径

候选顺序应可解释且稳定：优先插件实际所在目录和应用私有运行时目录，再尝试平台正常库名及系统搜索路径。路径来源不能依赖当前工作目录。

每个候选都独立经历加载和 API 版本验证。某个私有候选加载失败或不支持所需 API 时，正常模式可以继续检查后备候选，并保留各次失败原因。`isAvailable()` 和实际识别必须采用一致的候选策略，不能前者检查一套库、后者又强行加载另一套库。

Linux 正常运行库包未必安装无版本开发链接名，候选不能只验证本机开发环境的 `.so` 名称。应按支持平台的实际 SONAME、安装路径和打包布局测试，不通过任意全盘搜索猜测库位置。

### 7.2 严格诊断路径

诊断环境变量一旦指定，只尝试该单一路径；路径不存在、库缺少入口、版本不符或依赖加载失败时直接报错，不偷偷回退系统库。推荐要求绝对文件路径，错误消息说明这是严格覆盖模式。具体变量名以生产实现为准，本文不声明尚未实现的配置接口。

严格模式只限定**顶层 ORT 候选**。它并不自动隔离 ORT 的传递依赖搜索，也不能承诺进程中已有同名依赖完全不参与解析。

不应为了本次调用临时修改进程全局 `PATH`、`LD_LIBRARY_PATH` 或 DLL 搜索目录；这些修改可能影响其他线程和插件。需要私有依赖搜索策略时，应单独设计和验证。

## 8. Windows 动态依赖与打包

### 8.1 顶层 DLL 绝对路径不保证依赖位于同一目录

Microsoft 文档明确：即使顶层 DLL 使用完整路径加载，其依赖仍按模块名搜索；已加载模块列表和其他搜索规则仍参与解析。[R8]

Qt v6.12.0 `qlibrary_win.cpp` 使用普通 `LoadLibrary`，没有为这条路径设置 `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`。[R9] 因此，把 ORT 放到插件目录或私有 runtime 子目录，并不意味着同目录的 Protobuf、Abseil 等依赖一定可找到。

当前部署脚本把原生依赖放在应用 `bin` 目录，这与 Windows 应用目录搜索方式一致。若改为私有目录存放整套依赖，应评估 `LoadLibraryEx` 的受限搜索标志等平台方式，而不是把临时全局搜索路径修改混入通用加载器。[R8][R10]

### 8.2 显式动态根不可省略

核验的 [Windows 部署脚本](../qt/scripts/windows-deploy-runtime.sh) 从安装树现有 EXE/DLL 开始，调用 `objdump -p` 递归解析 `DLL Name`。取消插件对 ORT 的 PE 导入后，未复制到安装树的 ORT 不会自然进入这个队列。

包含 OCR 插件的安装包应显式处理：

1. 从构建选定的运行时前缀或明确制品中定位 ORT DLL，确认架构和所需 API 匹配；不能无条件接受 System32 中的同名旧库。
2. 将该 DLL 复制到加载策略实际覆盖的位置，并作为递归依赖队列的根。
3. 递归复制其非系统依赖，保留现有缺失依赖即构建失败的行为；不能只复制一份 ORT 就认为闭包完整。
4. 防止同名 DLL 按偶然目录顺序覆盖，记录选定路径、版本和打包结果。
5. 在没有开发工具链 PATH、没有构建目录的干净运行环境中完成实际 OCR，而不只检查压缩包包含文件。

脚本已有“不把构建前缀中存在的同名 DLL 当作系统库跳过”逻辑，应保留。没有安装 OCR 插件的构建则不应被强制要求携带无用 ORT。

测试应包含：动态根缺失、根存在但依赖缺失、旧系统 ORT 与应用私有 ORT 共存、严格覆盖到不兼容库，以及最终安装目录中实际识别成功。

## 9. Linux 包依赖边界

当前 [Debian 控制文件](../debian/control) 在构建依赖中列出 `libonnxruntime-dev`，运行依赖主要依靠 `${shlibs:Depends}`；[Arch SONAME 扫描脚本](../qt/scripts/arch-soname-depends.sh) 从安装树 ELF 的 `DT_NEEDED` 提取库依赖。

取消 ORT 的链接导入后，自动扫描不再必然产生 ORT 运行时包依赖。构建环境能找到头文件和库，不证明用户安装后二者仍存在；应根据发行策略显式声明动态运行时依赖，或将经过验证的私有运行库和依赖完整部署。[R11]

运行时依赖包名及最低版本应对应各发行版实际拆包和本次 `ORT_API_VERSION`，不能一律把开发包当作用户运行依赖。只把无版本 `dlopen` 名字写入代码也不能取代软件包管理器的依赖声明。

Linux 私有库的传递依赖搜索同样需要实际验证；绝对路径加载一个 `.so` 不等于其依赖自动使用同目录，不能假定与 Windows 或开发环境完全一致。[R6]

## 10. 生产验收清单

| 类别 | 必须验证 |
| --- | --- |
| API 边界 | 缺失入口、不支持版本、严格路径失败不回退；所有原生函数只经本次表调用 |
| 句柄清理 | 环境、选项、会话、名称、张量、形状信息和错误状态各阶段失败后均无遗漏 |
| 线程异常 | 创建失败、回调抛异常、错误文字转换失败的兜底，不出现 joinable 析构或 detach 后卸载 |
| 数据寿命 | callback 返回时无原生句柄、数据视图、异常对象或 DSO 字符串逃逸；输出仅为拥有型 Qt 值 |
| 并发引用 | availability 与推理重叠、多个加载器同路径、不同候选路径；释放一个引用不破坏另一个调用 |
| 原模型行为 | 三张原型样本之外继续覆盖空白、密集文本、长行、透明图、错误模型；完整文本、框、跨度和置信度对照 |
| 连续运行 | 同一插件实例反复识别，成功与失败交替，取消和关闭后真实排空；每轮重新取得当前 API |
| Linux 归因 | 受控测试每轮记录映射、堆、线程、RSS 高水位；正常生产不强制解除映射断言 |
| Windows 包 | ORT 显式根及传递依赖齐全，架构一致，在干净 PATH 与旧系统 ORT 共存环境可运行 |
| Linux 包 | 最终包仍声明或携带动态运行时；在没有开发包的环境验证正常名称和私有候选 |
| 性能 | 完整入口高水位、固定空闲 RSS、单次与连续识别耗时；不把原型数字当成改写后的测量结果 |

成功返回应由识别和资源生命周期正确性决定。库因其他有效持有者或平台规则继续映射时，可以记录诊断信息，但不应把正确识别结果改成错误，也不能反复强制关闭其他模块的引用。

## 11. 证据来源

本轮继续使用 `smart-search` 获取官方页面与固定版本源码；C++ 线程语义参考公开工作草案。核心结论已与本机 ORT v1.29.0 头文件及限定范围打包脚本对照，没有执行生产构建或修改脚本。

- [R1] [ORT v1.29.0 C++ API 头](https://github.com/microsoft/onnxruntime/blob/v1.29.0/include/onnxruntime/core/session/onnxruntime_cxx_api.h)：`Global::Api`、`InitApi(nullptr)` 行为和全部编译单元的宏一致性要求。
- [R2] [ORT OrtApiBase](https://onnxruntime.ai/docs/api/c/struct_ort_api_base.html)：入口、API 版本获取与版本字符串。
- [R3] [ORT C API](https://onnxruntime.ai/docs/api/c/struct_ort_api.html)：原生对象释放、默认 allocator、名称分配、张量字节数和同步 Run 语义。
- [R4] [C++ 线程构造](https://eel.is/c++draft/thread.thread.constr)、[线程析构](https://eel.is/c++draft/thread.thread.destr) 与 [工作草案 threads.tex](https://github.com/cplusplus/draft/blob/master/source/threads.tex)：线程启动异常、入口异常、join 和 joinable 析构规则。
- [R5] [Qt QLibrary](https://doc.qt.io/qt-6/qlibrary.html) 与 [Qt v6.12.0 qlibrary.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qlibrary.cpp)：可重入性、加载提示和共享加载引用。
- [R6] [Linux dlopen、dlclose](https://man7.org/linux/man-pages/man3/dlopen.3.html)：加载引用、依赖搜索、NODELETE 与成功关闭不保证移除映射。
- [R7] [Microsoft FreeLibrary](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary)：模块引用计数和卸载条件。
- [R8] [Microsoft DLL 搜索顺序](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order)：完整路径、传递依赖、已加载模块和应用目录。
- [R9] [Qt v6.12.0 qlibrary_win.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qlibrary_win.cpp)：实际使用 LoadLibrary、FreeLibrary 与防卸载提示处理。
- [R10] [Microsoft LoadLibraryEx](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw) 与 [DLL 安全加载](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-security)：依赖目录搜索标志和搜索路径安全边界。
- [R11] [Debian dpkg-shlibdeps](https://manpages.debian.org/testing/dpkg-dev/dpkg-shlibdeps.1.en.html)：从二进制使用的共享库生成依赖；动态运行时仍需由项目打包策略覆盖。

代表性取证命令：

```sh
smart-search fetch 'https://onnxruntime.ai/docs/api/c/struct_ort_api_base.html' --format markdown
smart-search fetch 'https://onnxruntime.ai/docs/api/c/struct_ort_api.html' --format markdown
smart-search fetch \
  'https://raw.githubusercontent.com/qt/qtbase/v6.12.0/src/corelib/plugin/qlibrary_win.cpp' \
  --format markdown
smart-search fetch \
  'https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order' \
  --format markdown
smart-search fetch 'https://raw.githubusercontent.com/cplusplus/draft/master/source/threads.tex' \
  --format markdown
```

## 12. 实现后的只读核验增量

本节针对已经写入的 C API 包装进行静态核验。主测试报告真实模型三项、运行库寿命 fixture 四项通过；本文没有重新执行这些测试，也没有修改生产文件。

### 12.1 已确认的正常路径

| 文件 | 核验结果 |
| --- | --- |
| [rapid_onnx_runtime.cpp](../qt/plugins/ocr-rapid/rapid_onnx_runtime.cpp) | 失败候选由 `qScopeGuard` 显式卸载；成功移交 `unique_ptr` 后原 guard 不会误卸载；本次 API 指针不进入全局变量 |
| [rapid_onnx_handle.h](../qt/plugins/ocr-rapid/rapid_onnx_handle.h) | 原生句柄保存创建运行库提供的 Release 函数，禁止复制；输出参数先清理旧对象并从空指针开始 |
| [rapid_onnx_session.cpp](../qt/plugins/ocr-rapid/rapid_onnx_session.cpp) | 输入容量按元素数乘 `sizeof(float)`，有溢出检查；输出核对类型和容量；正常释放及成员逆序析构均先 Session、后 Env |
| [rapid_onnx_session.cpp](../qt/plugins/ocr-rapid/rapid_onnx_session.cpp) | 默认 allocator 仅借用；原生名称通过该 allocator 的 `Free` 释放；错误状态由局部句柄管理，错误文字先复制 |
| [rapid_ocr_engine.cpp](../qt/plugins/ocr-rapid/rapid_ocr_engine.cpp) | 模型对象在回调内部构造和析构，检测完成后先释放 det；返回 token 不包含原生 ORT 句柄或借用张量 |
| [rapid_ocr_plugin.cpp](../qt/plugins/ocr-rapid/rapid_ocr_plugin.cpp) | 本次 runtime 在同步 `run()` 返回后才析构；插件对象继续常驻；可用性检查使用独立 runtime，不修改正在推理的 API 表 |

普通异常在工作线程内转换成 QString，异常对象在线程退出前销毁，没有通过 `exception_ptr` 带出。正常调用路径没有发现字节单位、默认 allocator 所有权或原生对象晚于库析构的问题。

这不等于已经验证所有资源耗尽分支。当前异常处理内的 `QString::fromUtf8` 仍可能分配内存；若该转换再次抛出分配异常，会逃出线程入口。应将持续分配失败的兜底视为独立覆盖缺口，不把普通异常 fixture 通过描述为完整的内存不足安全性证明。

### 12.2 注册器的元数据扫描可能提前设置防卸载提示

初次静态核验发现一项影响私有运行库回收的跨模块风险，随后已通过独立程序与真实注册器测试复现；以下描述修复前行为：

- [ProviderPluginRegistry::discoverOnce](../qt/src/providers/provider_plugin_registry.cpp) 为目录内每个 `QLibrary::isLibrary` 文件创建默认 `QPluginLoader` 并保存在候选中，即使该文件只是 ORT 原生库而非 Qt 插件。
- 元数据读取不需要映射库，但默认 `PreventUnloadHint` 已进入 Qt 按路径共享的 `QLibraryPrivate`。Qt 源码通过 `findOrCreate` 复用该状态，并以 OR 方式合并提示。[R5][R12]
- [RapidOnnxRuntime::load](../qt/plugins/ocr-rapid/rapid_onnx_runtime.cpp) 之后按同一路径构造 `QLibrary`，再追加 `ResolveAllSymbolsHint`，不会清除已存在的防卸载提示。
- 注册器加载失败时调用 `deleteLater()`，并非立即销毁扫描加载器；同一次能力加载期间调用 OCR 的 `isAvailable()` 时，该共享状态仍可能存在。

因此，“没有在元数据扫描时加载 ORT”不代表扫描不会改变之后的加载行为。若首次真正加载时继承了该提示，Linux 会使用 NODELETE，Windows 则可能固定模块；随后正确释放本次 runtime 引用仍不足以移除映射。[R5][R9]

不能把符号链接当作可靠规避：Qt v6.12.0 `QPluginLoader::locatePlugin` 对存在的绝对文件路径返回 `canonicalFilePath()`；当前 [运行库候选](../qt/plugins/ocr-rapid/rapid_runtime_paths.cpp) 也使用该规范路径，链接入口可能最终对应同一个 Qt 库记录。[R12]

影响条件是 ORT 文件位于注册器扫描目录，并且扫描加载器的共享状态在 runtime 构造或加载时仍存在。单独调用运行库 fixture、或将 ORT 放在扫描范围外，都可能无法覆盖这一情况。

### 12.3 最小复现与实际结果

测试无需模型，也无需执行 ORT 推理。本轮在 `build/memory-regression/qt-metadata-hint-probe/` 创建不带 Qt 插件元数据、只导出一个普通 C 函数的共享库及 QtCore 诊断程序。fixture 动态符号表的 `UNIQUE` 数量为 0，没有 ORT 或 Qt 库依赖，仅有编译器生成的常规 C 运行时依赖。

Qt 版本为 6.12.0。八种模式分别启动三次，共 24 个不同 PID，每次初始状态均没有 fixture 映射。所有模式的加载、函数解析与返回值检查均成功，`unload()` 均返回 true。

| 模式，每种三个新进程 | runtime 初始提示 | 追加 ResolveAllSymbols 后 | 卸载后仍映射 |
| --- | ---: | ---: | --- |
| 无扫描加载器 | 0 | 1 | 三次均否 |
| 默认扫描加载器保持存活 | 8 | 9 | 三次均是 |
| 默认扫描加载器使用符号链接入口 | 8 | 9 | 三次均是 |
| 扫描加载器先同步销毁 | 0 | 1 | 三次均否 |
| 调用 deleteLater，但尚未处理延迟删除 | 8 | 9 | 三次均是 |
| 调用 deleteLater，并在测试中处理延迟删除 | 0 | 1 | 三次均否 |
| 空构造、清提示、关联路径，扫描加载器继续存活 | 0 | 1 | 三次均否 |
| 同上，扫描入口为符号链接 | 0 | 1 | 三次均否 |

提示值 1 为 `ResolveAllSymbolsHint`，8 为 `PreventUnloadHint`，9 是两者组合。全部扫描模式都只读取元数据，记录为元数据为空、扫描器未加载、扫描后仍无 fixture 映射。两种符号链接模式中，扫描器最终路径与 runtime 规范路径相同。

这些结果直接确认：元数据扫描可以在不映射目标库的情况下影响稍后的加载提示；在扫描前清空提示能够消除该污染，而且不需要强制提前处理应用事件队列。

完整记录为 `build/memory-regression/qt-metadata-hint-probe/results.json`，另保留每次运行 JSON、fixture 的动态符号表和动态段记录。核心复现顺序是：

1. 每种模式启动独立进程，固定同一个绝对规范路径。
2. 对照模式只创建 `QLibrary`，记录提示，加载、解析并卸载。
3. 实验模式先创建并保留默认 `QPluginLoader`，仅调用 `metaData()`，确认元数据为空且 `isLoaded()` 为 false；再创建同路径 `QLibrary`，记录其提示是否已包含 `PreventUnloadHint`。
4. 追加 `ResolveAllSymbolsHint` 后再次检查提示；执行加载和卸载，观察映射或用可靠的模块卸载标记核对结果。
5. 增加同步销毁扫描加载器后再创建 `QLibrary` 的对照，以及扫描路径为符号链接的对照。记录 `QPluginLoader::fileName()`，不要仅比较传入的字符串。
6. 再使用真实注册器和私有运行库布局执行集成回归，覆盖能力加载及可用性检查触发的先后顺序。

各模式必须使用新进程，不能在已产生 NODELETE 或模块固定状态后切换参数并期待恢复初始条件。诊断程序执行延迟删除仅用于对照，不是建议在生产代码中强制处理全局事件队列。

复现构建命令：

```sh
cd build/memory-regression/qt-metadata-hint-probe
gcc -O2 -fPIC -shared fixture.c -o libmetadata-hint-fixture.so
g++ -O2 -std=c++17 -fPIC probe.cpp -o probe $(pkg-config --cflags --libs Qt6Core)
./probe scan-default "$PWD/libmetadata-hint-fixture.so" "$PWD/libmetadata-hint-link.so"
./probe scan-unpinned "$PWD/libmetadata-hint-fixture.so" "$PWD/libmetadata-hint-link.so"
```

### 12.4 真实注册器回归与修复复核

主测试把 ORT 实际文件复制到插件同目录，使用严格路径执行自动 OCR。修复前 `pinned-ocr-private-runtime` 在识别完成后仍观察到 ORT 映射，`repeatedOpenCloseReleasesRuntime` 断言失败；原始记录为 `build/memory-regression/results/private-runtime-red.log`。

生产修复由主实现完成，本文仅复核：

- `discoverOnce` 改为空构造 QPluginLoader、清空提示，再设置文件名，不再给全部原生库预先设置防卸载。
- `loadCandidate` 仅在顶层 Qt 元数据非空时，在实际 `instance()` 调用前添加 `PreventUnloadHint`。
- 没有声明业务 capabilities 的旧 Qt 插件仍有 Qt 元数据，因此保留常驻策略；没有 Qt 元数据的 ORT 文件不再受到污染。
- 非 Qt 运行库也不符合现有运行时恢复分支要求的 `mark-shot-ocr-rapid` 插件元数据名称，不会由该分支重新附加一份长期依赖引用。

修复后 `private-runtime-green.log` 记录七项相关测试全部通过，包括 `pinned-ocr-private-runtime`，该项耗时 4.27 秒。本文已经读取红、绿日志，未重复执行主测试。这确认了私有库布局中的实际问题得到修复，而不只是无模型探针发生变化。

本次限定范围内没有发现该修复遗漏原有插件常驻策略。外部代码若仍对相同运行库提前设置防卸载，或平台另有有效引用，运行库仍可继续映射；不得据此强制关闭其他持有者。Windows、macOS 的真实卸载没有在本轮执行，继续遵循不添加跨平台强制解除映射断言的边界。

### 12.5 补充源码

- [R12] [Qt v6.12.0 qpluginloader.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qpluginloader.cpp)：默认加载提示、元数据读取、绝对路径规范化和加载器对象释放；与 [qlibrary.cpp](https://github.com/qt/qtbase/blob/v6.12.0/src/corelib/plugin/qlibrary.cpp) 的共享路径记录及提示合并逻辑一并核验。
