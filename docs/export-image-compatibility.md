# Mac 风格外框与导出一致性

原版外框属于 `export.imageFrame` 配置，没有独立工具栏按钮。当前原生实现直接移植
原版 `crates/shot-image/src/export/` 算法，并接入编辑器保存、另存为与复制。
选区预览和初始范围边框不使用此效果。设置窗口仍按本次要求暂不完成。

## 模块职责

- `crates/shot-image/src/export/`：物理尺寸、圆角遮罩、预乘通道、阴影和单次内容合成
- `src/config/export.rs`：原生与原版键名、布尔及对象形式、兼容别名和范围校验
- `src/features/export_image/`：按需 PNG 转换、外框编码及用户导出策略
- `src/app/finish.rs`：共享图片用于保存和复制，原图用于贴图和标准输出
- `src/app/save_as.rs`：打开对话框时冻结外框快照，取消后恢复编辑器
- `src/features/pinned_image/model.rs`：原图编码与截图一致，显示缩放不改变导出

所有新增实现模块均小于 350 行。图像核心不引入图形框架或外部服务。

## 原版规则

默认参数为关闭、边距 112、圆角半径 18、阴影半径 72、纵向偏移 28、透明度 0.32。
边距范围为 0–256，两个半径及阴影偏移范围为 0–128，透明度范围为 0–1。
上述数值使用物理像素，不再乘以设备比例。

圆角模式沿用原版两像素内缩，剔除捕获窗口边缘。阴影以源图遮罩透明度生成，
采用 16 位中间通道和三次滑动盒式模糊。透明边界先参与模糊，再按偏移裁切到画布，
半透明正文只合成一次。零圆角不执行内缩；透明输入不会生成阴影。

原生配置示例：

```toml
[export.image_frame]
enabled = true
padding = 112
corner_radius = 18.0
shadow_radius = 72
shadow_offset_y = 28
shadow_opacity = 0.32
```

原版 `imageFrame`、`cornerRadius`、`shadowRadius`、`shadowOffsetY`、`shadowOpacity`
也可用于 TOML。布尔 `image_frame = true` 启用默认参数，原版对象与旧别名按原规则解析。
整数与实数字符串分别使用原版转换规则，错误类型及非有限实数不进入图像合成器。
原版非整数数字转换为零；字段存在但值非法时，不以旧别名覆盖。

导出前重新读取当前配置，编辑期间启用外框不改变预览。关闭时直接借用原始 PNG，
不分配解码缓冲。图像转换完成后及时释放旧缓冲，不保留缓存图像或等待线程。

## 输出边界

| 操作 | 外框行为 |
| --- | --- |
| 编辑器保存与复制 | 同一次外框合成，保存和剪贴板使用同一结果 |
| 编辑器另存为 | 打开时冻结结果；取消不记录历史，确认只保存一次 |
| 截图历史记录 | 记录实际用户导出的图片 |
| 历史复制、另存为与贴图 | 使用历史已有原图，不能再次套用外框 |
| 编辑器贴图 | 使用未添加外框的标注原图 |
| 贴图复制、另存为及编辑交接 | 保持贴图原图像素与尺寸 |
| 标准输出 | 保留原图，支持与带框保存、复制同时交付 |
| 无界面与滚动捕获 | 原图流程，外框配置不进入该交付入口 |
| OCR 输入及原图页面操作 | 使用已有原图入口，不经过用户图片导出策略 |

打开方式和扩展命令已经接入同一用户导出入口，向外部程序交付单次合成的带框 PNG；
实际参数、进程交接及失败恢复见 `docs/external-actions-compatibility.md`。
上传尚未实现，不能据外框算法认定该集成动作完成。

## 像素验证

`scripts/export-image-reference.py` 在私有目标目录重新编译原版 Rust 静态核心，
只读编译 Qt `export_image_effect.cpp`，由实际 Qt 函数生成期望 PNG。
采集使用 Qt 6.12.0，记录 40 个源码文件、采集程序和所有输出文件的 SHA-256。
原版代码在采集前后保持相同指纹，不复用过期构建对象。

冻结样本位于 `tests/fixtures/qt-export-image/`，共 36 组输出与独立通道舍入矩阵。
对照覆盖开关、原版别名、错误类型、范围、透明度、极小图像及 1.25、2 倍 DPR。
生产 PNG 转换与期望结果比较完整 RGBA，尺寸和全部通道均相等。

首次对照发现两个颜色舍入差异。原版 Qt 的本机 SIMD 转换使用近似倒数及一次牛顿
迭代，在部分半整数值处与既有整数编码规则不同。外框使用独立编码器处理该差异，
全部 32,896 种合法预乘通道与透明度组合通过本机 Qt 输出对照。
其他架构保留 Qt 标量规则，尚未完成这些架构的实际转换矩阵。

实际贴图交接还发现 17 个半透明像素在第二次编码后改变一个颜色单位。
最小回归在修改前失败，修复后通过；贴图现在采用截图相同的原图编码方式。
失败样本保留于 `target/wayland-export-image-sway-r2/raw-pin-copy-failed.png`，
最小失败日志位于 `target/export-image-pin-before.log`。

## 验证入口

```sh
uv run --no-project python scripts/export-image-reference.py \
  --root target/qt-export-image-reference-new

cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/private-export-image-new -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-export-image.py \
  --binary target/release/mark-shot --root target/wayland-export-image-new
```

私有运行器校验合成器进程、套接字与输出，不连接用户桌面。完整回归执行真实保存、
复制、贴图原图复制、另存为取消与确认、历史复制及两种无界面捕获。
另存为打开后再次改动配置，确认结果仍使用已经冻结的图片。
空闲阶段核对线程数、CPU 时钟及表面损坏提交；普通导出不初始化 OCR 模型。

整体重写仍未完成。颜色空间及完整 PNG 元数据、其他架构、全部合成器的运行矩阵，
以及最终完整版本的冷启动和资源释放，继续属于待覆盖范围。
