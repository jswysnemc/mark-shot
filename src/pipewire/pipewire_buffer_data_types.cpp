#include "pipewire/pipewire_buffer_data_types.h"

#include <spa/buffer/buffer.h>

namespace markshot::pipewire {

std::uint32_t bufferDataTypeMask(bool hasModifier)
{
    return hasModifier
        ? (1u << SPA_DATA_DmaBuf)
        : ((1u << SPA_DATA_MemPtr) | (1u << SPA_DATA_MemFd));
}

std::array<bool, 2> modifierPreference(bool rawStreamMode)
{
    return rawStreamMode
        ? std::array<bool, 2>{false, true}
        : std::array<bool, 2>{true, false};
}

}  // namespace markshot::pipewire
