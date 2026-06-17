#include "shot_window_module.h"

#include "annotation_state_store.h"
#include "debug_log.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

/// @brief 在构造阶段从磁盘载入工具默认值,覆盖 m_* 内置默认值
///
/// 该函数仅修改"工具默认值"成员,不会触碰 UI 面板;UI 在 setup 末尾按当前
/// 默认值刷新一次,因此调用时机必须早于 initializeToolbar 之后的属性面板刷新,
/// 又必须晚于 initializeShortcuts(避免被快捷键配置回填)。约定在 setup 流程
/// 紧随 m_toolbarAppearance 读取之后调用。
void ShotWindow::loadAnnotationStateFromDisk()
{
    // 1. 从磁盘读取(文件不存在则得到结构默认值)
    const markshot::AnnotationState state = markshot::loadAnnotationState();

    // 2. 颜色与不透明度
    if (state.currentColor.isValid()) {
        m_currentColor = state.currentColor;
    }
    m_textBackgroundColor = state.textBackgroundColor;

    // 3. 各类工具笔宽
    m_strokeWidth = state.strokeWidth;
    m_highlighterWidth = state.highlighterWidth;
    m_numberWidth = state.numberWidth;
    m_textSize = state.textSize;
    m_mosaicBlockSize = state.mosaicBlockSize;

    // 4. 矩形相关
    m_shapeFilled = state.shapeFilled;
    m_rectangleCornerRadius = state.rectangleCornerRadius;
    m_rectangleStyle = state.rectangleStyle;

    // 5. 放大镜相关
    m_magnifierScale = state.magnifierScale;
    m_magnifierShape = state.magnifierShape;

    // 6. 各风格枚举
    m_arrowStyle = state.arrowStyle;
    m_highlighterStyle = state.highlighterStyle;
    m_numberStyle = state.numberStyle;

    // 7. 文本字体(空字符串保留主题默认值)
    if (!state.textFontFamily.isEmpty()) {
        m_textFontFamily = state.textFontFamily;
    }
}

/// @brief 调度一次工具默认值的持久化写盘
///
/// 写盘走 QSaveFile 的同步 IO,在拖动滑块这类高频改动场景中直接写会让
/// 主线程绘制掉帧、滑块响应"看起来没生效"。这里只把 dirty 标志立起来,
/// 真正的磁盘写入交给 m_annotationStateTimer 在用户停止连续调整后执行,
/// 多次连续调用会通过重启 single-shot 定时器合并成一次写入。
///
/// 任意改变 m_currentColor / m_strokeWidth / m_highlighterWidth / m_numberWidth /
/// m_textSize / m_mosaicBlockSize / m_shapeFilled / m_rectangleCornerRadius /
/// m_rectangleStyle / m_magnifierScale / m_magnifierShape / m_arrowStyle /
/// m_highlighterStyle / m_numberStyle / m_textFontFamily / m_textBackgroundColor
/// 的入口写入完成后都应调用本函数。窗口关闭等退出路径需要
/// 调用 flushAnnotationStateNow 立即落盘,以避免崩溃丢失最后一次改动。
///
/// @return 无返回值
void ShotWindow::persistAnnotationState()
{
    m_annotationStateDirty = true;
    if (m_annotationStateTimer) {
        m_annotationStateTimer->start();
    }
}

/// @brief 立即把内存中的工具默认值写回磁盘
///
/// 用于关键退出路径或需要确认落盘的场景。无挂起改动时直接返回,有则取消
/// 节流定时器并同步执行 QSaveFile 的写入。
///
/// @return 无返回值
void ShotWindow::flushAnnotationStateNow()
{
    if (!m_annotationStateDirty) {
        return;
    }
    if (m_annotationStateTimer) {
        m_annotationStateTimer->stop();
    }
    m_annotationStateDirty = false;

    markshot::AnnotationState state;
    state.currentColor = m_currentColor;
    state.strokeWidth = m_strokeWidth;
    state.highlighterWidth = m_highlighterWidth;
    state.numberWidth = m_numberWidth;
    state.textSize = m_textSize;
    state.mosaicBlockSize = m_mosaicBlockSize;
    state.shapeFilled = m_shapeFilled;
    state.rectangleCornerRadius = m_rectangleCornerRadius;
    state.rectangleStyle = m_rectangleStyle;
    state.magnifierScale = m_magnifierScale;
    state.magnifierShape = m_magnifierShape;
    state.arrowStyle = m_arrowStyle;
    state.highlighterStyle = m_highlighterStyle;
    state.numberStyle = m_numberStyle;
    state.textFontFamily = m_textFontFamily;
    state.textBackgroundColor = m_textBackgroundColor;

    // 写入失败仅记录日志,不影响交互;dirty 已清,等待下次任意修改再次尝试
    if (!markshot::saveAnnotationState(state)) {
        markshot::debugLog("annotation_state",
                           "failed to persist annotation state to %s",
                           markshot::annotationStateFilePath().toUtf8().constData());
    }
}
