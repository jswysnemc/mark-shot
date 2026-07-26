#include "recording/ui/recording_overlay_layout.h"

#include <algorithm>

namespace markshot::recording::ui {

QRect recordingControlBarRect(const QRect &regionRect,
                              const QRect &screenRect,
                              const QSize &barSize,
                              int spacing)
{
    if (barSize.isEmpty() || screenRect.isEmpty()) {
        return {};
    }

    // 1. 水平方向跟随录制区域中心，超出屏幕时向内收
    const int centerX = regionRect.isEmpty() ? screenRect.center().x() : regionRect.center().x();
    int x = centerX - barSize.width() / 2;
    x = std::clamp(x, screenRect.left() + spacing, screenRect.right() - barSize.width() - spacing + 1);

    // 2. 没有有效录制区域时贴屏幕底部
    if (regionRect.isEmpty()) {
        const int y = screenRect.bottom() - barSize.height() - spacing + 1;
        return {x, std::max(screenRect.top() + spacing, y), barSize.width(), barSize.height()};
    }

    // 3. 优先放在录制区域下方
    const int below = regionRect.bottom() + spacing + 1;
    if (below + barSize.height() <= screenRect.bottom() + 1) {
        return {x, below, barSize.width(), barSize.height()};
    }

    // 4. 其次放在录制区域上方
    const int above = regionRect.top() - spacing - barSize.height();
    if (above >= screenRect.top()) {
        return {x, above, barSize.width(), barSize.height()};
    }

    // 5. 区域几乎占满屏幕时贴在区域内部底边
    const int inside = std::min(regionRect.bottom() - barSize.height() - spacing + 1,
                                screenRect.bottom() - barSize.height() - spacing + 1);
    return {x, std::max(screenRect.top() + spacing, inside), barSize.width(), barSize.height()};
}

RecordingOverlayPlacement recordingOverlayPlacement(const QRect &regionRect,
                                                    const QRect &screenRect,
                                                    bool recordsWholeScreen,
                                                    const QSize &barSize,
                                                    int spacing)
{
    RecordingOverlayPlacement placement;
    if (screenRect.isEmpty() || barSize.isEmpty()) {
        return placement;
    }

    // 1. 整屏录制没有需要标示的区域边界
    placement.showRegionFrame = !recordsWholeScreen && !regionRect.isEmpty();

    // 2. 控制条一旦落进录制画面就会被录下来，此时改由托盘与快捷键控制
    const QRect barRect = recordingControlBarRect(regionRect, screenRect, barSize, spacing);
    const bool intersectsRecording = !regionRect.isEmpty() && barRect.intersects(regionRect);
    placement.showControlBar = !recordsWholeScreen && !intersectsRecording && !barRect.isEmpty();
    if (placement.showControlBar) {
        placement.controlBarRect = barRect;
    }
    return placement;
}

}  // namespace markshot::recording::ui
