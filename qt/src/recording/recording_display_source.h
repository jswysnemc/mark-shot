#pragma once

#include "recording/recording_options.h"

#include <QRect>
#include <QString>
#include <QVector>

namespace markshot::recording {

/**
 * 用于把区域录制绑定到单块屏幕的候选几何。
 */
struct RecordingScreenCandidate {
    QString name;
    QRect geometry;
};

/**
 * 读取当前可用于录制的显示器来源。
 * @return 显示器来源列表，多屏时包含全部显示器来源。
 */
QVector<DisplaySource> availableDisplaySources();

/**
 * 当区域完全落在一块屏幕内时，返回该屏幕名称。
 * @param region 录制区域。
 * @param screens 候选屏幕。
 * @return 完全包含该区域的屏幕名称；跨屏或为空时返回空字符串。
 */
inline QString containedRecordingScreenName(const QRect &region,
                                            const QVector<RecordingScreenCandidate> &screens)
{
    const QRect normalized = region.normalized();
    if (!normalized.isValid() || normalized.isEmpty()) {
        return {};
    }

    for (const RecordingScreenCandidate &screen : screens) {
        if (screen.name.isEmpty() || !screen.geometry.isValid() || screen.geometry.isEmpty()) {
            continue;
        }
        if (screen.geometry.contains(normalized)) {
            return screen.name;
        }
    }
    return {};
}

/**
 * 区域录制改绑到实际包含该区域的那块屏幕。
 * 对话框选中的显示器只是默认值，用户可在另一块屏幕框选区域，输出名必须以选区所在屏幕为准，
 * 否则 wlroots/WGC 会按名称选错输出，门户授权令牌也会按错误输出保存和恢复。
 * 整屏录制和跨屏区域保持原样，避免单输出后端误走交互式门户。
 * @param options 录制配置，命中单屏时会改写 display。
 * @param screens 候选屏幕。
 * @return 无返回值。
 */
inline void bindRegionRecordingDisplay(RecordingOptions *options,
                                       const QVector<RecordingScreenCandidate> &screens)
{
    if (!options || options->scope != RecordingScope::Region) {
        return;
    }

    const QString screenName = containedRecordingScreenName(options->captureGeometry, screens);
    if (screenName.isEmpty()) {
        return;
    }

    for (const RecordingScreenCandidate &screen : screens) {
        if (screen.name != screenName || !screen.geometry.contains(options->captureGeometry.normalized())) {
            continue;
        }
        options->display.allOutputs = false;
        options->display.screenName = screen.name;
        options->display.outputName = screen.name;
        options->display.geometry = screen.geometry;
        if (options->display.title.isEmpty()) {
            options->display.title = screen.name;
        }
        return;
    }
}

/**
 * 按当前 Qt 屏幕列表绑定区域录制的目标显示器。
 * @param options 录制配置。
 * @return 无返回值。
 */
void bindRegionRecordingDisplay(RecordingOptions *options);

}  // namespace markshot::recording
