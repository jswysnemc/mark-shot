# Windows 发布说明

## 录制依赖

Windows 录制使用 Windows Graphics Capture 采集画面，使用 FFmpeg/libav 写入 MP4 或 GIF。MSYS2/UCRT64 构建必须安装以下包：

```bash
pacman -S --needed \
  mingw-w64-ucrt-x86_64-ffmpeg \
  mingw-w64-ucrt-x86_64-qt6-base \
  mingw-w64-ucrt-x86_64-qt6-tools
```

发布构建使用 `-DMARK_SHOT_REQUIRE_FFMPEG=ON`。如果缺少 FFmpeg 头文件或导入库，CMake 会在配置阶段失败，避免生成无法录制的 Windows 包。

## 运行时部署

Windows 包通过 `scripts/windows-deploy-runtime.sh` 运行 `windeployqt`，并用 `objdump` 递归复制 `mark-shot.exe` 和 DLL 的依赖项。FFmpeg 的 `avcodec`、`avformat`、`avutil`、`swresample`、`swscale` 相关 DLL 会随依赖闭包进入 `app/bin`。

## 代码签名

发布工作流支持 Authenticode 签名。需要在 GitHub 配置：

- `WINDOWS_CODESIGN_CERTIFICATE_BASE64`：PFX 证书的 base64 内容，放在 Secrets。
- `WINDOWS_CODESIGN_CERTIFICATE_PASSWORD`：PFX 密码，放在 Secrets。
- `WINDOWS_CODESIGN_TIMESTAMP_URL`：时间戳服务器地址，放在 Variables；未配置时使用 `http://timestamp.digicert.com`。

本地生成 base64 内容可使用：

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("codesign.pfx")) | Set-Content codesign.pfx.base64
```

脚本 `scripts/windows-sign-artifact.ps1` 会在压缩发布包前签名 `app` 目录下的 `.exe` 和 `.dll`。未配置证书时脚本会跳过签名，不影响普通 CI。

代码签名可以显著降低 Windows 和杀毒软件误报概率，但误报还与证书信誉、下载量、文件行为和分发域名有关。新证书发布初期仍可能触发 SmartScreen 提示，需要持续使用同一证书发布版本来积累信誉。
