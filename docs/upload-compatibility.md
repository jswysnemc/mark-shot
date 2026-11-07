# 图片上传与原版行为对照

截图工具栏上传按钮已接入 Rust 内置 HTTP 上传与自定义命令。沿用原版直接上传当前
选区、复制 URL 和成功后延迟关闭的流程，不增加上传服务列表或设置窗口。

## 模块职责

```text
src/config/upload.rs       上传分组、原版字段别名与系统环境优先级
src/upload/mod.rs          URL 结果与有限错误
src/upload/request.rs      multipart、鉴权、HTTP 响应及 URL 提取
src/upload/process.rs      私有 PNG、命令参数、有限管道与进程组回收
src/upload/task.rs         按需工作线程、取消与一次完成通知
src/backend/task_wake.rs   覆盖层按需通知管道
src/app/upload.rs          原图导出、上传输入隔离、复制与结果恢复
src/ui/toast/state.rs      提示动画与独立到期期限
```

普通截图启动不读取上传凭据，不创建上传通知管道、HTTP 客户或运行时线程。
HTTP 模式直接使用最终 PNG；自定义命令使用随机名称、权限为 `0600` 的临时 PNG。

## 配置与传输

默认上传命令为空，表示使用内置 HTTP。端点没有默认远程地址，需要通过配置或环境
变量指定。下面示例与原版 helper 的环境字段相同：

```toml
[upload]
command = ""
timeout_ms = 60000

[upload.env]
MARK_SHOT_UPLOAD_URL = "https://image-host.example/upload"
MARK_SHOT_UPLOAD_FIELD = "image"
MARK_SHOT_UPLOAD_API_KEY = "configured-token"
MARK_SHOT_UPLOAD_AUTH_HEADER = "Authorization"
MARK_SHOT_UPLOAD_AUTH_SCHEME = "Bearer"
MARK_SHOT_UPLOAD_URL_PATH = "data.url"
MARK_SHOT_UPLOAD_DELETE_URL_PATH = "data.delete_url"
MARK_SHOT_UPLOAD_FIELD_album = "screenshots"

[keys]
upload = "F8"
```

`MARK_SHOT_UPLOAD_HEADER_` 后缀作为自定义请求头，`MARK_SHOT_UPLOAD_FIELD_` 后缀作为
额外表单字段。HTTP 使用 multipart POST，图像保持 PNG 字节和透明度；普通重定向
最多十次。响应先读取配置的点分隔路径，支持数组下标，然后依次尝试 `url`、
`data.url`、`data.link`、`link` 和 `image.url`。删除链接沿用原版独立字段和回退路径，
不写入剪贴板。非 JSON 响应只接受首个非空行中的 HTTP(S) 前缀；自定义命令按原版
Qt 规则忽略前缀大小写，内置 HTTP 沿用原版 helper 的小写前缀规则。JSON 的非空
`url` 字符串保持原版接受范围，不额外限制返回值协议。

命令配置兼容 `command`、`path` 和 `helper`；原版 JSON 分组兼容 `upload`、
`imageUpload`、`uploader` 和 `imageHost`，环境对象兼容 `env`、`environment`、
`envVars` 和 `variables`。首个实际对象或字符串字段优先，空对象和空字符串不会
继续采用后续别名。原版整数超时范围与整值浮点数处理保持一致。

系统 `MARK_SHOT_UPLOAD_COMMAND` 和 `MARK_SHOT_UPLOAD_TIMEOUT_MS` 优先于文件。
配置环境覆盖子进程与 HTTP 实际环境，命令外壳遵循系统 `SHELL`，空值采用 `/bin/sh`。
命令只替换原版 `{image}`、`{imagePath}`、`{imageUrl}` 三种图片占位符，无占位符时
追加路径。参数经过单参数引用，替换结果不会再次作为占位符解析。
命令结果优先读取顶层 JSON `url`；纯文字仍使用首行 URL。

上传入口按需独立读取 TOML 或命令行指定的原版 JSON 上传分组。
这项兼容不代表整个旧版 JSON 配置迁移已经完成。

## 编辑器与生命周期

- 左键同位置按下、释放才请求上传；右键和中键不启动任务
- 上传前提交正文与属性字段，使用包含当前标注和导出外框的最终图像
- 任务期间阻止画布、滚轮、文字和其他快捷键修改；焦点与修饰键状态继续更新
- Escape 取消本次上传，保留截图；取消后丢弃已经到达的过期成功结果
- 网络完成通过独立通知唤醒阻塞事件循环，不增加等待上传的周期计时器
- 成功复制 URL 后延迟一百五十毫秒关闭，不执行预设保存或复制图片，不追加截图历史
- 失败、超时、无 URL 或复制失败恢复同一冻结图像、标注和选区，使用统一限时提示
- 成功、失败、取消及超时均回收工作线程和命令所属进程组，删除本次私有 PNG

失败提示仍保持默认四秒停留及原淡入淡出。此前限时提示把整个停留期视为动画，
每帧检查是否到期。现在完整显示期间只等待最早到期期限，到期后继续原淡出绘制。
最小失败记录为 `target/upload-toast-before.log`，回归同时验证到期、淡出和最终恢复
无期限阻塞等待。第一轮真实采样进入了尚未结束的淡出，保留
`target/wayland-upload-sway-r1/` 的原始失败证据，没有降低 CPU 或画面相等断言。

## 原版证据与验证入口

`scripts/upload-reference.py` 提取未修改的完整 `uploadConfig`、
`ShotWindow::uploadSelection`、参数引用函数和配置对象读取函数。外围提供私有文件、
本地命令接收端与复制接收端，没有链接完整截图窗口。探针包括十二组配置、八组
上传结果、六组 helper 字段提取及三组 multipart 字节；冻结样本为
`tests/fixtures/qt-upload.json`，源码和产物指纹位于 `target/qt-upload-return-values/index.json`。

```sh
uv run --no-project python scripts/upload-reference.py --root target/qt-upload-new
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
uv run --no-project --with pillow --with numpy python scripts/wayland_text_sway_runtime.py \
  --root target/p-upload-new -- \
  uv run --no-project --with pillow --with numpy python scripts/wayland-upload.py \
  --binary target/release/mark-shot --root target/wayland-upload-new
```

Rust 测试覆盖真实 HTTP、三百零三重定向、无长度超限响应、实际命令、私有文件、
两类输出管道、取消、超时和后代终止。Wayland 验证执行实际左右键、带框 PNG、
矩形标注、持久 URL 复制、失败恢复、取消和超时，并在提示消失后逐像素比较恢复
画面与原画面，再核对单线程、零 CPU 计数增长和零表面损坏提交增长。

本轮本地二进制为 46,462,952 字节，SHA-256 为
`bb53a056a8e0d57795eb0e5b82fbb4be441eaf1fb4c7bb2789ba8c24e951d356`。
工作区 1003 项 Rust 测试通过、10 项原有忽略；199 项 Python 测试、格式、全部目标
与全部功能严格 Clippy、新脚本 Ruff F 检查及发布构建通过。动态链接没有 Qt、GTK
或 WebView，721 个 Rust 文件均少于 750 行，最长为 692 行。

最终实际记录为 `target/wayland-upload-sway-r5/result.json` 和
`target/wayland-upload-niri-r5/result.json`，汇总为
`target/verification-upload/result.json`。两种合成器的上传 PNG 与原版带框样本逐像素
相同，失败、取消和超时后保存的标注图与实际上传图逐像素相同，恢复界面与之前画面
逐像素相同。六次一秒静止采样均为单线程、零 CPU 计数增量和零损坏提交增量。

第一轮完整场景在取消后的日志等待处失败：脚本读取标准错误，产品的信息级完成
记录实际位于独立日志文件。原始文件保留在 `target/wayland-upload-*-final/`。
最终脚本按当前进程号读取所属日志，超时必须明确匹配 `Upload timed out`，没有以
其他进程或其他错误替代预期结果。进程组测试也改为在一秒期限内观察实际后代退出，
不把直接子进程已回收当作全部后代在同一调度时刻退出，最终仍拒绝运行中的后代。

最后复核补入原版接受的非 HTTP JSON `url` 和不完整大写纯文本前缀，修正了初版
过严的返回值校验。主题采集也移除固定时刻截帧：一次快捷键后等待保存值与实际
像素同时达到原版精确颜色，记录约 205–300 ms 的本机观察上限，不重复投递按键。
此前提前采样失败保留在 `target/wayland-upload-*-r4/`，这些观察耗时包含输入、截图
等待与采集开销，不等同于文件同步耗时或合成器呈现延迟。

实际商业图床的账户、额度和远程服务差异未在这些本地测试中验证；完整旧配置、
全部合成器、工具提示和全界面逐像素对照仍属于整体重写的未完成范围。
网络与命令输出均有内存限额，空 URL、超限响应和非法请求头返回失败。
