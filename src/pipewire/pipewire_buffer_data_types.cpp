#include "pipewire/pipewire_buffer_data_types.h"

#include <QtGlobal>

#ifdef HAVE_PIPEWIRE
#include "pipewire/pipewire_dmabuf_policy.h"
#include "pipewire/pipewire_drm_fourcc.h"

#include <spa/buffer/buffer.h>

#include <QByteArray>
#endif

namespace markshot::pipewire {

#ifdef HAVE_PIPEWIRE
namespace {

bool dmaBufDisabled()
{
    // 环境变量与已知失效组合都会走共享内存，判定集中在 dmabuf policy 中
    return shouldAvoidDmaBuf();
}

}  // namespace
#endif

std::uint32_t bufferDataTypeMask(bool hasModifier)
{
#ifdef HAVE_PIPEWIRE
    // 强制共享内存：部分 labwc / KWin+NVIDIA 组合上 DMA-BUF 无法导入。
    if (dmaBufDisabled()) {
        return (1u << SPA_DATA_MemPtr) | (1u << SPA_DATA_MemFd);
    }
    // modifier 格式通常只能走 DMA-BUF；线性格式优先 CPU 可映射缓冲。
    return hasModifier
        ? (1u << SPA_DATA_DmaBuf)
        : ((1u << SPA_DATA_MemPtr) | (1u << SPA_DATA_MemFd));
#else
    Q_UNUSED(hasModifier);
    return 0;
#endif
}

std::array<bool, 2> modifierPreference(bool rawStreamMode)
{
#ifdef HAVE_PIPEWIRE
    if (dmaBufDisabled()) {
        // 两个槽都声明无 modifier，避免门户仍协商出 DMA-BUF。
        return std::array<bool, 2>{false, false};
    }
    return rawStreamMode
        ? std::array<bool, 2>{false, true}
        : std::array<bool, 2>{true, false};
#else
    Q_UNUSED(rawStreamMode);
    return {false, false};
#endif
}

}  // namespace markshot::pipewire
