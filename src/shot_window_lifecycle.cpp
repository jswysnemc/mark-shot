#include "shot_window_module.h"

/// @brief 关闭窗口前提交临时编辑内容并同步写入挂起的工具默认值
///
/// @param event Qt 关闭事件
/// @return 无返回值
///
/// 关闭路径可能发生在拖动粗细滑块后的 250ms 节流窗口内。这里统一刷新,
/// 保证最后一次工具默认值变更不会因为窗口立即关闭而丢失。
void ShotWindow::closeEvent(QCloseEvent *event)
{
    // 1. 提交仍在编辑中的文本标注,让字体等默认值变更先进入内存状态
    commitTextEditor();

    // 2. 提交滚轮改粗细的挂起历史,避免防抖定时器随窗口销毁一起丢失
    commitAnnotationWidthWheelHistory();

    // 3. 同步刷新挂起的状态写盘,避免节流定时器随窗口销毁一起丢失
    flushAnnotationStateNow();

    // 4. 交给 QWidget 继续处理关闭生命周期
    QWidget::closeEvent(event);
}
