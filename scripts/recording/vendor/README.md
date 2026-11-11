# 固定组件源码

`gpu-screen-recorder-f97d1e3.tar.xz` 是原始 Git 提交的源码归档，不包含本项目补丁。

- 上游：<https://repo.dec05eba.com/gpu-screen-recorder>
- 提交：`f97d1e392816503282d20a0804c3ce3fd2943767`
- SHA256：`4c8b74be6a6938369db5500a7bcbca10f5f0a2c21a228f62fc4ef15da6c44d6b`
- 许可证：GPL-3.0-only，完整原文位于归档内 `gpu-screen-recorder/LICENSE`
- 来源：2026-11-11 调研时下载的固定提交；后续上游拒绝该提交的 fetch 请求，因此保留已核验源码以支持重复构建

生成方式：`git archive --format=tar --prefix=gpu-screen-recorder/ f97d1e392816503282d20a0804c3ce3fd2943767 | xz -c`。

构建先校验归档摘要，再执行 `patch-gsr.py`。发行包中的 `COMPONENT-SOURCES.tar.xz` 包含应用补丁后的完整源码、原始归档和构建脚本。
