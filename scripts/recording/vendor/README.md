# 固定组件源码

`gpu-screen-recorder-f97d1e3.tar.xz` 是原始 Git 提交的源码归档，不包含本项目补丁。

- 上游：<https://repo.dec05eba.com/gpu-screen-recorder>
- 提交：`f97d1e392816503282d20a0804c3ce3fd2943767`
- SHA256：`4c8b74be6a6938369db5500a7bcbca10f5f0a2c21a228f62fc4ef15da6c44d6b`
- 许可证：GPL-3.0-only，完整原文位于归档内 `gpu-screen-recorder/LICENSE`
- 来源：2026-11-11 调研时下载的固定提交；后续上游拒绝该提交的 fetch 请求，因此保留已核验源码以支持重复构建

生成方式：`git archive --format=tar --prefix=gpu-screen-recorder/ f97d1e392816503282d20a0804c3ce3fd2943767 | xz -c`。

构建先校验归档摘要，再执行 `patch-gsr.py`。发行包中的 `COMPONENT-SOURCES.tar.xz` 包含应用补丁后的完整源码、原始归档和构建脚本。

其余固定 Git 源码采用相同归档方式保存，完整许可证位于各归档中：

| 归档 | 上游与提交 | SHA256 |
| --- | --- | --- |
| `vulkan-headers-88487f6.tar.xz` | <https://github.com/KhronosGroup/Vulkan-Headers>，`88487f687f3100ebe1d3559a8a34d971ee85b977` | `3285a5a1726edc80641af1b1f02c0313b1b6883c46588f170bda9e187c9ec1da` |
| `wf-recorder-c5de474.tar.xz` | <https://github.com/ammen99/wf-recorder>，`c5de47440e8e81c92befb696cb69819cdb3bfe8a` | `a5041b47587dedb6e7c578f303141fc3acd4c8993fcaa9b46b71795247c978b9` |

Vulkan-Headers 使用 Apache-2.0／MIT 等逐文件许可；wf-recorder 使用 MIT。归档前缀分别为 `Vulkan-Headers/` 和 `wf-recorder/`。Vulkan-Headers 在远程构建同样出现固定提交无法获取；wf-recorder 一并固定，避免相同供应来源风险。
