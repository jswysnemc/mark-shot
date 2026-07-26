#pragma once

#include <QRect>
#include <QSize>

namespace markshot::recording::ui {

struct RecordingOverlayPlacement {
    bool showRegionFrame = false;
    bool showControlBar = false;
    QRect controlBarRect;
};

/**
 * 【录制】【覆盖层】计算控制条在屏幕上的位置。
 *
 * 控制条优先放在录制区域正下方，空间不足时依次尝试区域上方与区域内底部，
 * 始终保持在屏幕可见范围内，尽量不遮挡被录制内容。
 *
 * @param regionRect 录制区域矩形，使用屏幕坐标。
 * @param screenRect 目标屏幕矩形。
 * @param barSize 控制条尺寸。
 * @param spacing 控制条与录制区域之间的间距。
 * @return 控制条矩形。
 */
QRect recordingControlBarRect(const QRect &regionRect,
                              const QRect &screenRect,
                              const QSize &barSize,
                              int spacing = 12);

/**
 * 【录制】【覆盖层】计算覆盖层各元素是否显示及其位置。
 *
 * 控制条与边框都不能进入录制画面，因此与录制区域相交时不显示控制条，
 * 整屏录制也不绘制区域边框。
 *
 * @param regionRect 录制区域矩形，使用屏幕坐标。
 * @param screenRect 目标屏幕矩形。
 * @param recordsWholeScreen 是否为整屏录制。
 * @param barSize 控制条尺寸。
 * @param spacing 控制条与录制区域之间的间距。
 * @return 覆盖层布局结果。
 */
RecordingOverlayPlacement recordingOverlayPlacement(const QRect &regionRect,
                                                    const QRect &screenRect,
                                                    bool recordsWholeScreen,
                                                    const QSize &barSize,
                                                    int spacing = 12);

}  // namespace markshot::recording::ui
