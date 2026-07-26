#pragma once

namespace markshot::pipewire {

struct DmaBufEnvironment {
    // 会话运行在 KDE Plasma 上
    bool kdeSession = false;
    // 存在 NVIDIA 专有驱动节点
    bool nvidiaProprietaryDriver = false;
    // 系统中 DRM 渲染节点数量
    int renderNodeCount = 0;
    // 环境变量要求强制使用 DMA-BUF
    bool forcedByEnvironment = false;
    // 环境变量要求禁用 DMA-BUF
    bool disabledByEnvironment = false;
};

/**
 * 【录制】【PipeWire协商】按环境判断是否应当避开 DMA-BUF 缓冲。
 *
 * KWin 在 NVIDIA 专有驱动上导出 DMA-BUF 会失败，表现为 compositor 侧报
 * GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT，PipeWire 收到无有效数据的缓冲，
 * 录制随即失败。这类组合直接改用共享内存，牺牲零拷贝换取可用。
 *
 * @param environment 环境探测结果。
 * @return 应当避开 DMA-BUF 时返回 true。
 */
bool shouldAvoidDmaBuf(const DmaBufEnvironment &environment);

/**
 * 【录制】【PipeWire协商】探测当前会话环境。
 * @return 环境探测结果。
 */
DmaBufEnvironment currentDmaBufEnvironment();

/**
 * 【录制】【PipeWire协商】按当前会话环境判断是否应当避开 DMA-BUF 缓冲。
 * @return 应当避开 DMA-BUF 时返回 true。
 */
bool shouldAvoidDmaBuf();

}  // namespace markshot::pipewire
