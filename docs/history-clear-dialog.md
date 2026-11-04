# 历史清空独立对话框

## 原版证据

原版 `qt/src/capture_history/history_window_actions.cpp` 使用 `QDialog::open()` 创建
父窗口模态确认，不使用历史窗口内覆盖层。正文为
`Delete all screenshot history? Saved files will be kept.`，按钮为 Cancel 和 Clear History。
Cancel 初始获得焦点并作为默认按钮，确认后才清理历史目录。

探针 `tests/fixtures/qt-history-dialog-reference/main.cpp` 只读链接原版当前 407 个对象，
过期依赖为零。官方 QtTest 投递真实鼠标事件；配置、记录和剪贴板只位于临时进程环境，
离屏采集主动清除真实显示服务、Wayland、桌面 IPC 和会话总线环境。

冻结样本为 `tests/fixtures/qt-history-dialog.json`，原始证据为
`target/qt-history-dialog-reference-r6/`。Qt 版本为 6.12.0，覆盖三十个英文深色场景。

| 区域 | 实测矩形或属性 |
| --- | --- |
| 默认窗口 | 315×96 |
| 最小窗口 | 223×77 |
| 正文 | 11, 11, 293, 38；支持换行 |
| Cancel | 11, 55, 144, 30 |
| Clear History | 161, 55, 143, 30 |
| 窗口模态 | Qt WindowModal |
| Tab 顺序 | Cancel → Clear History → Cancel |

普通合成鼠标事件不能证明焦点行为。改用 QtTest 后确认右键也会取得按钮焦点，但不执行
点击；右键选中 Clear History 后再按 Enter 会确认。Space 和左键按下不执行，释放才
执行；在按钮外释放取消该次事务。默认 Enter、Escape 和窗口管理器关闭均取消。Enter 与 Space 保留修饰键行为；带修饰键的 Escape 不关闭对话框。

## 原生实现

`src/backend/wayland/window/surface.rs` 管理各自配置、缩放、帧回调和缓冲；父子窗口
使用同一个 Wayland 连接、进程和共享内存分配器。`dialog.rs` 在首次提交前设置原生
父子关系，可用时声明 xdg-dialog 模态提示；无论合成器是否过滤，客户端都隔离父窗口输入。

父窗口的布局、焦点生命周期和后台完成仍正常处理。键盘、指针和输入法按存活表面身份
路由，关闭或过期子窗口不能接收新窗口输入。子窗口关闭只回传一次结果，不退出父窗口；
窗口管理器关闭始终取消，父窗口关闭回收所属子窗口。

`clear_dialog.rs` 只处理确认焦点与释放事务；`clear_dialog_draw.rs` 使用统一控件绘制
原版紧凑正文和按钮，不加入额外内部标题、图标或提示文字。清空结果匹配当前对话框身份，
旧结果不能确认后续窗口；明确确认才提交后台清空。原图 RGBA、IDAT、目录身份和配置
保留规则不变。

## 当前验证

本地 release SHA-256 为 `0414da80d889192e0412aaa5b50e173f0a3aac8b03217e5d54579fef301db5ea`，
大小 43283592 字节。783 项 Rust 测试通过、10 项忽略；148 项 Python 测试通过。
严格 Clippy、格式和导入检查通过，日志位于 `target/history-dialog-work/`。

私有 niri 独立确认回归位于 `target/history-clear-dialog-r2/`，完整历史流程位于
`target/history-clear-main-r2/`。确认回归验证同进程独立子窗口、默认取消、窗口管理器
取消、父窗口复制与删除隔离、右键不执行、外部释放取消、Space 释放确认、关闭父窗口
回收子窗口，以及损坏合法记录清理和无关文件、链接保留。空闲采样为单线程、零 CPU
时钟增量。共享窗口后端另执行 niri、Sway OCR 输入法和文字光标回归，不复用旧二进制证据。

该 niri 会话没有提供 xdg-dialog 管理器，当前证据证明原生父子关系和客户端模态隔离，
不证明模态提示分支。窗口系统装饰、全部主题和语言、全部焦点信号与 Qt 全像素比较仍未
完成。新发布必须验证远程下载产物，不能把这些本地证据视为旧预览版已经更新。
