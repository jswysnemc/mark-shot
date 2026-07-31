#include "pipewire/pipewire_buffer_data_types.h"

#include <QtGlobal>

#ifdef HAVE_PIPEWIRE
#include <spa/buffer/buffer.h>
#endif

namespace markshot::pipewire {

std::uint32_t bufferDataTypeMask(bool hasModifier)
{
#ifdef HAVE_PIPEWIRE
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
    return rawStreamMode
        ? std::array<bool, 2>{false, true}
        : std::array<bool, 2>{true, false};
#else
    Q_UNUSED(rawStreamMode);
    return {false, false};
#endif
}

}  // namespace markshot::pipewire
