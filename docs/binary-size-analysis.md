# Linux 发布版体积归因

测量日期：2026-11-08。代码版本：`c4608de0961bdb5d8d0c2ab603876632e3273d76`。

当前发布版为 **49,923,000 字节，即 49.92 MB / 47.61 MiB**。最大的单项来源是静态链接的
ONNX Runtime：链接映射可直接归属的代码和数据至少占 **19.70 MiB，约为发布版的 41.4%**。
这解释了当前程序接近 50 MB 的主要原因。识别模型独立下载，不包含在这些字节中。

## 构建与测量口径

- 平台：`x86_64-unknown-linux-gnu`，Rust/Cargo 1.98.1，LLVM 22.1.8。
- 发布命令：`cargo build --locked --release --bin mark-shot`。
- 使用默认功能集合，没有启用 `ocr-gpu`。
- 保持现有 `lto = "thin"`、`codegen-units = 1`、`strip = "symbols"`，优化等级沿用 release 默认值。
- `file target/release/mark-shot` 确认为已剥离符号的 ELF；没有保留完整调试信息。
- 发布文件 SHA-256：`8d26e1afdbe60287c710ef51e77b252e574fed5f8f9a674cc9618e231ce6d845`。

归因使用独立的 `target/size-analysis` 目录，临时保留符号。该构建大小为 63,422,976 字节，
对副本执行 `strip --strip-all` 后为 49,917,744 字节。分析副本与实际发布版相差 5,256 字节，
约 0.0105%，二者不是逐字节相同的文件。下文 ELF 总量取实际发布版，模块归属取分析副本；
模块百分比只用于近似解释实际发布文件。

分析副本 SHA-256：`c5477f79137c285ea26eb5d49aab2684994a2ca31ca9af10d41be0d7e523c34b`。
增加链接映射参数后，剥离符号的分析副本哈希保持一致。

本次仅做归因，没有修改依赖特性、发布参数或 OCR 加载方式。以下 MiB 均为 1,048,576 字节。

## 文件实际组成

| 内容 | 字节数 | MiB | 文件占比 |
| --- | ---: | ---: | ---: |
| 机器代码 `.text` | 37,706,210 | 35.96 | 75.53% |
| 只读数据 `.rodata` | 4,618,408 | 4.40 | 9.25% |
| 异常与栈展开表 | 4,971,504 | 4.74 | 9.96% |
| 动态重定位记录 | 1,466,664 | 1.40 | 2.94% |
| 数据、地址表与初始化数组 | 1,112,072 | 1.06 | 2.23% |
| 其余文件内容与对齐 | 48,142 | 0.05 | 0.10% |
| 合计 | 49,923,000 | 47.61 | 100% |

异常与栈展开表包括 `.eh_frame`、`.eh_frame_hdr`、`.gcc_except_table`，不是可直接删除的
调试符号。重定位包括 `.rela.dyn` 和 `.rela.plt`。数据项包括 `.data.rel.ro`、`.data`、
`.got`、`.got.plt`、`.tdata`、`.init_array`、`.fini_array`。

`.bss`、`.tbss` 等 NOBITS 段不占用对应数量的文件字节，未计入上表。`size` 默认输出中的
`text` 也不等于单独的 `.text` 段，因此这里使用 `size -A` 与 ELF 段信息逐项核对。

## 最大来源：ONNX Runtime

依赖路径为 `mark-shot → oar-ocr 0.9.2 → oar-ocr-core → ort → ort-sys 2.0.0-rc.13`。
`ort-sys` 构建输出明确包含 `cargo:rustc-link-lib=static=onnxruntime`。
最终链接映射中的原生对象位于 `libort_sys-*.rlib`，可按输入对象汇总：

| 原生运行时内容 | 字节数 | MiB |
| --- | ---: | ---: |
| 机器代码 `.text` | 17,823,717 | 17.00 |
| 栈展开 `.eh_frame` | 1,738,132 | 1.66 |
| 异常表 `.gcc_except_table` | 798,548 | 0.76 |
| 只读地址数据 `.data.rel.ro` | 160,864 | 0.15 |
| 可直接归属的 `.rodata` | 134,644 | 0.13 |
| 其余文件数据与初始化代码 | 4,975 | 0.005 |
| 可直接归属合计 | 20,660,880 | 19.70 |

这 17.00 MiB 原生代码约占全部机器代码的 47.3%。合计是可归属下界：链接器会合并字符串、
常量并生成重定位和索引，部分内容不再带有唯一输入对象归属，未强行分摊给 OCR。
Rust 的 `ort` / `oar-ocr-core` 包装代码也未算入这张原生对象表。

不能把缓存中的 `libonnxruntime.a` 文件大小当成最终占用。静态库包含最终没有保留的对象、
段和符号，最终链接结果才是有效依据。也不能把改成动态加载后的主程序缩小量当成完整安装包
缩小量：随包分发的动态库仍然占用磁盘。

`src/ocr/models.rs` 定义的检测器、各语言识别器和字典存放在用户数据目录，按需下载。
当前约 50 MB 包含推理运行时，但不包含这些 `.onnx` 模型。

## Rust 代码与内置资源

以下使用 cargo-bloat 0.12.1 按函数符号估计 `.text` 归属，便于判断其余成本来源。
LTO 内联、泛型实例化、符号合并会影响分组，数字不是“删除功能即可减少”的实测结果，
也不能与上一节的原生对象表相加。

| 符号归属 | 代码 MiB | 主要依赖 |
| --- | ---: | --- |
| 主程序命名空间 | 4.40 | `mark_shot` |
| Rust 标准库符号 | 2.44 | cargo-bloat 合并后的 `std` |
| 字体与文本排版 | 2.11 | `harfrust`、`skrifa`、`read_fonts`、`cosmic_text`、`swash` 等 |
| Wayland 与 D-Bus | 2.07 | `zbus`、`zvariant`、Wayland 客户端、portal 等 |
| 图像处理与编解码 | 1.79 | `image`、WebP、JPEG、EXR、TIFF、PNG 等 |
| HTTP 与 DNS | 1.05 | `reqwest`、`ureq`、`hyper`、`hickory` 等 |
| SVG 与路径栅格化 | 0.89 | `usvg`、`resvg`、`tiny-skia` 等 |

cargo-bloat 把 `ort_sys` 识别为 13.97 MiB，另有 2.81 MiB 无法按包识别的代码，以及部分
C++ 模板符号归入其他命名空间。因此原生推理引擎采用链接映射的 17.00 MiB 代码统计，
不直接使用 cargo-bloat 的包名结果。上述表只列主要组，不构成完整文件分摊。

五份内置字体共 **1,679,368 字节 / 1.60 MiB**，约占整个发布文件的 3.36%。已在分析副本中
按完整字体字节查找，四份 Inter 与一份序号字体各出现一次。这些字节已经计入 `.rodata`，
不是额外增加的安装大小。来源见 `src/fonts.rs` 与 `assets/fonts/`。

## 依赖特性发现

根项目虽然对 `image` 配置了 `default-features = false`，`oar-ocr`、`oar-ocr-core` 和
`imageproc` 仍然启用了它的默认特性。Cargo 合并特性后，除根项目显式选择的格式，还启用了
AVIF、EXR、DDS、HDR、QOI、TGA 等格式。

“启用并编译”不代表相关实现全部进入最终二进制。链接器会移除不可达代码，例如不能仅凭
构建日志出现 `ravif` 就将其整个库大小计入发布文件。当前符号分析能确认 EXR 等代码保留；
缩减格式需要同时调整上游依赖特性，并以最终发布文件和格式回归验证收益。

系统动态依赖包括 fontconfig、libstdc++、OpenSSL、libxkbcommon、PipeWire、libgcc、libm
与 libc。它们没有全部复制进这个 ELF，完整安装占用需另加系统库和下载模型。文件体积与
运行时 RSS/PSS 是不同指标。

## 优化判断

| 方向 | 当前证据与边界 |
| --- | --- |
| 精简 ONNX Runtime 算子与执行提供方 | 最大单项来源；需要按实际模型构建并验证，当前没有裁剪收益实测 |
| 将 OCR 运行时拆成动态库或可选组件 | 能改变主程序体积，完整分发体积未必下降，需要处理安装与加载行为 |
| 控制图像格式特性 | 上游重新启用了默认格式；仅修改根项目的 `image` 配置不足以收敛 |
| 评估 `dist` 的完整 LTO 或体积优化等级 | 项目已有 `dist`，本次未构建对照，不能承诺收益或性能不变 |
| 调整内置字体 | 可归属总量仅 1.60 MiB，涉及字形、粗体与斜体覆盖，不是最大来源 |

发布版已执行常规符号剥离，重复 strip 不会解决主要体积来源。异常表包含预编译 C++ 库
数据，单独修改 Rust 的 panic 策略不能假定可以消除全部 4.74 MiB，也需要先检查恢复路径。

## 复现命令与验证

```sh
cargo build --locked --release --bin mark-shot
stat -c '%s' target/release/mark-shot
sha256sum target/release/mark-shot
file target/release/mark-shot
size -A target/release/mark-shot
readelf -SW target/release/mark-shot
readelf -d target/release/mark-shot

cargo bloat --release --locked --bin mark-shot \
  --target-dir target/size-analysis --crates -n 0 --message-format json \
  > target/binary-size-crates.json

CARGO_PROFILE_RELEASE_STRIP=false cargo rustc --release --locked --bin mark-shot \
  --target-dir target/size-analysis -- \
  -C link-arg=-Wl,-Map=target/binary-size-link.map
strip --strip-all -o target/size-analysis/release/mark-shot.stripped \
  target/size-analysis/release/mark-shot

cargo tree --locked -e features -i ort-sys
cargo tree --locked -e features -i image
```

原生对象归因按 lld 映射的输入段行求和：只统计输入路径含 `libort_sys-` 的行，按所属输出段
分组；排除符号说明行、`.bss`、`.tbss` 等 NOBITS 数据。合并后无法识别来源的数据保留为未分摊。
原始结果保存在忽略版本控制的 `target/binary-size-crates.json`、`target/binary-size-link.map`
和 `target/binary-size-link-origins.json`，本报告保留关键数字与测量口径。

提交前验证：工作区 Rust 测试 1425 项通过、20 项忽略；Python 日志解析测试 5 项通过，
14 个改动脚本通过语法检查。严格 Clippy、Rust 格式检查、差异空白检查和 release 构建通过。
首次默认并发测试中 OCR 进程恢复用例等待就绪超时；单独复测通过，随后四线程全量复测通过。
本次没有修改该用例，也不将一次复测通过解释为已经修复其偶发超时。
