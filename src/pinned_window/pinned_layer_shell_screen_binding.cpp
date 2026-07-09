#include "pinned_window/pinned_layer_shell_screen_binding.h"

#include "pinned_window/pinned_layer_shell_geometry.h"

namespace markshot::shot {

PinnedLayerShellScreenBinding resolvePinnedLayerShellScreenBinding(
    QRect geometry,
    const QVector<QRect> &screenGeometries,
    int boundScreenIndex)
{
    PinnedLayerShellScreenBinding binding;
    binding.targetScreenIndex = bestPinnedLayerShellScreenIndex(geometry, screenGeometries);
    binding.screenChanged = boundScreenIndex >= 0
        && binding.targetScreenIndex >= 0
        && binding.targetScreenIndex != boundScreenIndex;
    return binding;
}

}  // namespace markshot::shot
