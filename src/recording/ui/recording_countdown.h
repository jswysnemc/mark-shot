#pragma once

#include <QRect>

#include <functional>

class QScreen;

namespace markshot::recording::ui {

/**
 * 【录制】【倒计时】在正式开始录制前显示倒计时提示。
 *
 * 倒计时面板显示在录制区域附近但不进入录制画面，倒计时结束后回调启动录制。
 * 秒数不大于零时立即回调，不创建任何窗口。
 *
 * @param seconds 倒计时秒数。
 * @param regionRect 录制区域矩形，使用屏幕坐标。
 * @param screen 目标屏幕。
 * @param onFinished 倒计时结束后的回调。
 * @return 无返回值。
 */
void runRecordingCountdown(int seconds,
                           const QRect &regionRect,
                           QScreen *screen,
                           std::function<void()> onFinished);

}  // namespace markshot::recording::ui
