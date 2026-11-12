# 二维码与条形码

截图选区后点击副工具栏的扫码图标，或在贴图右键菜单选择“识别二维码与条形码”。
识别结果在独立原生窗口中显示，支持文字选择、复制所选、复制全部和滚动。关闭结果窗口
即可结束该进程。识别出的链接和命令仅作为文本显示。
从置顶贴图打开的结果保持在原图上方；Wayland 使用高于贴图 Top 层的 Overlay 层，
关闭结果不改变原贴图的置顶设置。普通文件和截图入口仍使用普通窗口。

```sh
mark-shot --scan
mark-shot --scan-code
mark-shot --scan image.png
mark-shot --scan-json image.png
```

无图片参数时进入屏幕区域选择；提供图片时直接识别；`--scan-json` 输出 JSON 数组且不开窗口。
每条结果包含 `format` 和 `text`，未发现条码时输出 `[]`；损坏或超限图片返回错误。
快捷键可在配置中设置 `keys.scan_code`，默认不占用按键。自定义副工具栏使用 `scan_code` 项。

## 实现范围

复用项目图片解码库，使用纯 Rust `rxing 0.9.4`。关闭默认功能，只启用条码解码、
多码读取及文本编码支持；条码生成器仅用于测试。没有新增模型、网络请求、浏览器或外部运行库。

真实图像测试覆盖 QR Code、Code128、EAN13、Data Matrix、Aztec、PDF417，以及多二维码、
中文、旋转、透明背景与反色。其他码制沿用库的支持范围，不保证模糊、遮挡或畸变条码可识别。
结果按码制与文本去重。输入文件最多 64 MiB，最多 33,554,432 像素；图片解码器的内部
分配预算为 128 MiB，这不是整个进程的内存上限。解码完成后释放图片，仅保留结果文本。
正常极性和反色极性分别扫描并合并结果，灰度缓存原地复用；不会因为正常条码已识别成功
而跳过同一图片中的反色码。

## 体积与验证

同一台 Linux 主机、相同工具链与 `cargo build --release --locked --bin mark-shot`，
比较基线 `43534bd` 和包含扫码及本次历史问题修复的版本。gzip 使用 Python `gzip.compress(data, mtime=0)`：

| 项目 | 原版本 bytes | 扫码版本 bytes | 增量 |
| --- | ---: | ---: | ---: |
| 主程序 | 30,405,320 | 31,688,992 | 1,283,672，约 1.22 MiB / 4.22% |
| 主程序 gzip | 12,857,180 | 13,384,394 | 527,214，约 515 KiB / 4.10% |

这是主程序对比，不是重新构建的双平台完整压缩包。扫码没有新增独立运行时资源。
`cargo tree -e normal,build,features -i rxing --depth 2` 确认发行依赖不启用 rxing 的
`encoders`、`image`、`imageproc`、`image_formats`、`client_support` 或 `rayon`；
项目原有 OCR 依赖中的同名组件不属于本次新增。

- 工作区回归：1544 项通过、28 项忽略；扫码专项共 6 项通过
- Linux 全目标 Clippy、Windows GNU 全测试目标交叉 Clippy、全仓 Rust 格式检查通过
- Linux 隔离 Niri 实测：文件窗口、屏幕选区、截图工具栏和贴图菜单均识别出两个二维码
- 实测中文复制、全选复制、删除键只读保护、长文本滚动和窗口宽度调整；JSON 输出可直接解析
- Windows 扫码代码通过交叉编译检查，未进行 Windows 原生窗口实测

审查修复的失败样本位于 `tests/fixtures/code-scan/mixed-polarity.png`。修复前整图只识别
Code128，修复后四种旋转和整图反转均识别出 Code128 与 Data Matrix。置顶问题在隔离 Niri
中通过真实贴图菜单复测，确认结果位于 Overlay 层、原图仍在 Top 层，复制、拖动、关闭及
原贴图菜单恢复均正常；截图为 `target/code-scan-evidence/pinned-result-fixed.png`。

本地证据保存在 `target/code-scan-evidence/`；录制资源补测见[录制基准](recording/benchmark.md)。
