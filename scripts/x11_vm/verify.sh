#!/usr/bin/env bash
# 【X11虚拟机】【完整回归】按可选证据目录参数依次运行真实虚拟机流程，返回首个失败状态
set -euo pipefail
root=${1:-target/x11-vm-audit/full-$(date +%Y%m%d-%H%M%S)}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
mkdir -p "$root"

# 1. 【X11虚拟机】【核心交互】同一桌面按顺序执行，避免不同测试抢占输入或剪贴板
for case in capture windows wheel settings ocr desktop scrolling; do
    python "$script_dir/workflow.py" --case "$case" --root "$root/$case"
done

# 2. 【X11虚拟机】【媒体交付】分别核对 MP4 与 GIF 的真实成品
for format in mp4 gif; do
    python "$script_dir/workflow.py" --case recording --root "$root/recording-$format" --override "MARK_SHOT_TEST_FORMAT=$format"
done

# 3. 【X11虚拟机】【候选独立性】分别关闭或启用图像和窗口识别，并核对遮挡及最小化
for detection in window region disabled overlap minimized; do
    python "$script_dir/workflow.py" --case detection --root "$root/detection-$detection" --override "MARK_SHOT_TEST_DETECTION=$detection"
done

# 4. 【X11虚拟机】【选择区完整性】普通载荷和真正超过服务器上限的 PNG、中文文本均重复读取
python "$script_dir/workflow.py" --case clipboard --root "$root/clipboard-normal"
python "$script_dir/workflow.py" --case clipboard --root "$root/clipboard-incr" --override MARK_SHOT_TEST_CLIPBOARD_SIZE=large
python "$script_dir/workflow.py" --case clipboard --root "$root/clipboard-text" --override MARK_SHOT_TEST_CLIPBOARD_KIND=text
