# OCR 翻译窗口输入法兼容性

本文件记录本地 Rust 重构的输入法差分验证。原版 Qt 源码只读，测试使用私有
Sway 或 niri 会话、固定 OCR 文档和本地翻译命令，不读取用户配置，不调用模型
或外部翻译服务。不提交、不推送、不发版，不修改远程工作流。

本轮范围是原文、译文和目标语言的编辑输入，以及多行候选在切页和菜单期间的
显示与输入资格。它不代表整个重写项目完成，也不代表完整窗口像素一致。

## 模块边界

| 文件 | 职责 |
| --- | --- |
| `src/ui/text_editor/input.rs` | 共享编辑器键盘输入；空按键事件不打断预编辑 |
| `src/backend/wayland/window.rs` | 内容接口区分临时停用与真正取消输入 |
| `src/backend/wayland/window/text_input.rs` | 原生目标切换、周边正文同步和过期批次拒绝 |
| `src/features/ocr_result/input_method.rs` | OCR 字段输入资格、批次分派及两类候选清理 |
| `src/features/ocr_result/source_menu.rs` | 原生菜单生命周期，菜单期间不编辑正文 |
| `src/features/ocr_result/translation/input.rs` | 各字段键盘分派及 Qt 多行重做快捷键 |
| `src/features/ocr_result/translation/input_method_tests.rs` | 原版序列、真实后端停用调用、过期目标与修饰键回归 |
| `scripts/wayland-ocr-translation-input.py` | 同一真实输入客户端驱动 Qt 和 Rust |
| `scripts/wayland_ocr_translation_input_checks.py` | 两端证据读取、协议独立序号和候选像素断言 |
| `scripts/test_wayland_ocr_translation_input_checks.py` | 证据解析、编码边界和禁止伪造上下文的回归 |

## 已核对行为

- 原文和译文使用 `Ctrl+Shift+Z` 重做，`Ctrl+Y` 不恢复多行编辑历史。
- 目标语言初始光标位于末尾，程序设置目标语言后也回到末尾。
- 原文与译文的输入法确认分别进入自己的撤销历史；预编辑不进入全文复制。
- 辅助平面字符通过实际 UTF-8 字节删除替换，Qt 对照记录使用 UTF-16 光标。
- 忙碌期间原文仍可编辑，译文只读，目标语言停用；请求只使用确认正文快照。
- 文本页切到原图页再返回，原文和译文保留候选显示；隐藏期间没有输入资格。
- 原文和译文右键菜单保留候选显示；菜单期间输入法停用，关闭后使用新目标。
- 旧目标和停用期间的确认不能修改恢复后的编辑器。
- 临时停用不是窗口失焦。真正失焦、普通字段切换仍清理候选；语言和表格候选
  不使用多行候选保留规则。
- 单独修饰键按下没有编辑按键或文字，不应清理预编辑。实际 niri 键盘事件验证
  这一行为，不只依赖虚拟键盘修饰状态。
- 语言右键菜单关闭后，光标和锚点均位于文本末尾。记录来自实际控件的选区
  和光标信号；不能用尚未提交的旧协议选区推断控件恢复状态。

冻结控件序列位于 `tests/fixtures/qt-ocr-translation-input.json`，覆盖 380 和
760 两种宽度，各 26 个状态。原版探针直接连接实际 `OcrResultWindow`，不使用
仿制编辑器代替原版业务控件。

## 参考实现缺陷与安全差异

原版观察和原生安全断言分别保存。原版自身的错误不能成为原生实现的兼容目标。

1. 原版替换等宽辅助平面字符后，控件正文正确，但部分路径只发送新的周边正文，
   不调用 text-input-v3 `commit()`。输入法客户端仍持有旧正文。测试记录控件
   查询与实际协议状态，两者不能合并或相互替代。
2. 原版只读转可编辑时，部分路径重新启用输入但不重新发送缓存的周边正文和
   内容类型。原生实现必须提供完整的新上下文，不允许使用原版查询豁免。
3. 原版原文、译文和语言菜单期间，输入仍活跃，延迟的 `stale-menu` 确认会进入
   正文。原生菜单保留多行候选外观，但停用输入并拒绝这些确认。结果明确记录
   `stale_commit_ignored` 和 `input_active_while_menu`，不要求复现原版的错误修改。
4. input-method-v2 的 `done` 计数和 text-input-v3 的提交计数是两个独立序号。
   Sway 中的偶然相等不能当作协议保证。测试从实际 text-input-v3 创建与提交
   请求计算该实例的序号，并对当前批次核对精确交付。
5. 原版 niri 观察在语言右键菜单关闭后的首次指针焦点恢复步骤失败，证据保留于
   `target/ocr-translation-input-qt-niri-r2/`。该轮没有完整 `result.json`，不能列为
   通过。原生 niri 场景使用成功的 Sway 原版观察核对已确认行为；它不表示原版
   niri 平台测试完整通过。

## 验证入口

构建原版只读探针：

```sh
uv run --no-project python tests/fixtures/qt-property-reference/build_reference.py \
  --source tests/fixtures/qt-ocr-translation-input-reference/main.cpp \
  --output target/qt-ocr-translation-input-r4/reference --qt-test
```

原版和原生场景分别使用新目录：

```sh
uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/private-input-reference -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-ocr-translation-input.py \
  --binary target/qt-ocr-translation-input-r4/reference \
  --root target/input-reference --reference

uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/private-input-native -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-ocr-translation-input.py \
  --binary target/release/mark-shot --root target/input-native \
  --expected-reference target/input-reference/result.json
```

每轮保存实际协议日志、输入法客户端状态、控件查询、候选截图、复制正文、请求
快照与二进制 SHA-256。最终构建、全量测试及既有场景结果集中记录于
`target/ocr-translation-input-local-r1/validation.json`。旧二进制的通过结果不能认证
本轮源码；失败目录也不能作为成功结果引用。

本轮优化二进制为 `target/release/mark-shot`，SHA-256：
`7d7945ba2372f2cb93cc1dfd0d1b23de652408c2cdd35cab036d2b7c31043d99`。

| 验证 | 结果 |
| --- | --- |
| Rust 工作区 | 898 项通过，10 项忽略；既有标尺失败单独复现并明确排除 |
| Python 工具 | 192 项通过 |
| 格式、严格 Clippy、优化构建 | 通过 |
| 原版固定序列重复核对 | 52 个冻结状态一致，另确认 6 个切页候选状态 |
| 优化二进制真实输入法 | Sway 1 倍、1.2 倍及 niri 1.2 倍均通过 |
| 既有交互回归 | 翻译窗口与退出清理、反馈、更多菜单、按钮、原文／表格输入法及历史通过 |

上述原生输入窗口空闲时均为单线程，连续观察期间 CPU 计数不增长，RSS 约
36 至 40 MiB。这些记录不替代完整启动性能或多输出测试。

## 尚未验证

- 语言候选存在时的首次右键行为：原版首次操作可能不创建菜单。本轮只核对
  无候选的语言菜单；结果使用 `candidate_tested: false`，不以零像素冒充通过。
- 实际 Fcitx/IBus、多输出、大文档候选资源、完整键盘焦点遍历及全部修饰快捷键。
- 全窗口逐像素一致、中文界面、独立贴图、占位文字、滚动条、光标闪烁及其他
  已列入重写状态的功能。
- 已知标尺旋转测试失败仍单独报告，不能称全量 Rust 测试无例外通过；未修改
  HEAD 的复现证据是 `target/ocr-titlebar-ruler-head-baseline.log`。
