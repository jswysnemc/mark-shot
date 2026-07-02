#include "shot_window_module.h"

#include "display_capture/display_capture_picker.h"
#include "display_capture/display_capture_save.h"
#include "notifications/app_notifications.h"

using namespace markshot::shot;

namespace {

/**
 * 判断显示器目标索引是否可用。
 * @param targets 显示器目标列表。
 * @param index 目标索引。
 * @return 索引可用时返回 true，否则返回 false。
 */
bool validTargetIndex(const QVector<markshot::display_capture::Target> &targets, int index)
{
    return index >= 0 && index < targets.size();
}

}  // namespace

/**
 * 显示按下 D 键瞬间生成的显示器快照目标。
 * @param targets 显示器快照目标列表。
 * @return 无返回值。
 */
void ShotWindow::showDisplayCaptureTargets(QVector<markshot::display_capture::Target> targets)
{
    if (m_mode != Mode::Selecting) {
        return;
    }

    m_displayCaptureTargets = std::move(targets);
    ensureDisplayCapturePicker();
    m_displayCapturePicker->setTargets(m_displayCaptureTargets);

    if (!isVisible()) {
        show();
    }
    raise();
    activateWindow();

    updateDisplayCapturePickerGeometry();
    m_displayCapturePicker->show();
    m_displayCapturePicker->raise();
    m_displayCapturePicker->setFocus(Qt::ShortcutFocusReason);
    update();
}

/**
 * 显示或隐藏显示器快速截取选择器。
 * @return 无返回值。
 */
void ShotWindow::toggleDisplayCapturePicker()
{
    if (m_mode != Mode::Selecting) {
        return;
    }
    if (m_displayCapturePicker && m_displayCapturePicker->isVisible()) {
        m_displayCapturePicker->hide();
        setFocus(Qt::OtherFocusReason);
        update();
        return;
    }

    if (m_startupTool != StartupTool::None) {
        leaveStartupTool();
    }
    emit displayCaptureSnapshotRequested(this);
}

/**
 * 确保显示器快速截取选择器已经创建并连接信号。
 * @return 无返回值。
 */
void ShotWindow::ensureDisplayCapturePicker()
{
    if (m_displayCapturePicker) {
        return;
    }

    m_displayCapturePicker = new markshot::display_capture::DisplayCapturePicker(this);
    connect(m_displayCapturePicker,
            &markshot::display_capture::DisplayCapturePicker::copyRequested,
            this,
            &ShotWindow::copyDisplayCaptureTarget);
    connect(m_displayCapturePicker,
            &markshot::display_capture::DisplayCapturePicker::editRequested,
            this,
            &ShotWindow::editDisplayCaptureTarget);
    connect(m_displayCapturePicker,
            &markshot::display_capture::DisplayCapturePicker::saveRequested,
            this,
            &ShotWindow::saveDisplayCaptureTarget);
    connect(m_displayCapturePicker,
            &markshot::display_capture::DisplayCapturePicker::dismissed,
            this,
            [this] {
                m_displayCapturePicker->hide();
                setFocus(Qt::OtherFocusReason);
                update();
            });
}

/**
 * 复制指定显示器目标图像。
 * @param index 目标索引。
 * @return 无返回值。
 */
void ShotWindow::copyDisplayCaptureTarget(int index)
{
    if (!validTargetIndex(m_displayCaptureTargets, index)) {
        return;
    }

    const bool copied = markshot::copyImageToClipboard(m_displayCaptureTargets.at(index).image);
    showToast(copied ? MS_TR("Display image copied") : MS_TR("Copy failed"), 1800);
}

/**
 * 编辑指定显示器目标图像。
 * @param index 目标索引。
 * @return 无返回值。
 */
void ShotWindow::editDisplayCaptureTarget(int index)
{
    if (!validTargetIndex(m_displayCaptureTargets, index)) {
        return;
    }

    if (m_displayCapturePicker) {
        m_displayCapturePicker->hide();
    }
    emit displayCaptureEditRequested(this, m_displayCaptureTargets.at(index));
}

/**
 * 保存指定显示器目标图像。
 * @param index 目标索引。
 * @return 无返回值。
 */
void ShotWindow::saveDisplayCaptureTarget(int index)
{
    if (!validTargetIndex(m_displayCaptureTargets, index)) {
        return;
    }

    QString path;
    if (markshot::display_capture::saveDisplayCaptureTarget(m_displayCaptureTargets.at(index), &path)) {
        const QString message = MS_TR("Saved to %1").arg(path);
        if (!markshot::notifications::notifyScreenshotSaved(path)) {
            showToast(message, 2500);
        }
        return;
    }

    showToast(MS_TR("Save failed"), 2500);
}

/**
 * 隐藏显示器快速截取选择器。
 * @return 无返回值。
 */
void ShotWindow::hideDisplayCapturePicker()
{
    if (!m_displayCapturePicker) {
        return;
    }
    m_displayCapturePicker->hide();
}

/**
 * 判断显示器选择器是否可见。
 * @return 可见时返回 true，否则返回 false。
 */
bool ShotWindow::displayCapturePickerVisible() const
{
    return m_displayCapturePicker && m_displayCapturePicker->isVisible();
}

/**
 * 判断坐标是否位于显示器选择器内。
 * @param point 窗口坐标。
 * @return 位于选择器内时返回 true，否则返回 false。
 */
bool ShotWindow::displayCapturePickerContains(QPoint point) const
{
    return m_displayCapturePicker && m_displayCapturePicker->isVisible()
        && m_displayCapturePicker->geometry().contains(point);
}

/**
 * 更新显示器选择器的位置。
 * @return 无返回值。
 */
void ShotWindow::updateDisplayCapturePickerGeometry()
{
    if (!m_displayCapturePicker) {
        return;
    }

    const int panelWidth = std::min(width() - 32, 440);
    m_displayCapturePicker->setFixedWidth(std::max(320, panelWidth));
    m_displayCapturePicker->adjustSize();
    const QSize panelSize = m_displayCapturePicker->sizeHint();
    const QPoint topLeft((width() - panelSize.width()) / 2,
                         (height() - panelSize.height()) / 2);
    m_displayCapturePicker->setGeometry(QRect(topLeft, panelSize));
}
