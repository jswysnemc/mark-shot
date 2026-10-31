#include "shot_window_module.h"

#include "shot_window_selection_translate.h"
#include "toolbar_dock_layout.h"
#include "ui/compact_toolbar.h"

#include <QScopeGuard>

using namespace markshot::shot;

namespace {

// 停靠时工具栏与屏幕边缘的距离，与全屏纵向贴边保持一致
constexpr int kDockedToolbarEdgeMargin = 12;

/// @brief 【截图工具栏】【停靠布局】计算两条工具栏停靠到指定边缘后的几何
/// @param controls 截图窗口控件组合
/// @param edge 停靠的屏幕边缘
/// @param bounds 截图窗口矩形
/// @return 停靠几何；副工具栏被隐藏时只摆放主工具栏
markshot::DockedToolbarLayout currentDockedToolbarLayout(const ShotWindowControls &controls,
                                                         markshot::ToolbarDockEdge edge,
                                                         const QRect &bounds)
{
    const QSize toolbarSize = controls.m_toolbar ? controls.m_toolbar->sizeHint() : QSize();
    // 1. 副工具栏只在选区编辑时显示，全屏标注时其动作并入主工具栏
    const QSize actionSize = controls.m_actionToolbar && !controls.m_actionToolbar->isHidden()
        ? controls.m_actionToolbar->sizeHint()
        : QSize();
    return markshot::dockedToolbarLayout(edge, bounds, toolbarSize, actionSize, kDockedToolbarEdgeMargin,
                                         qRound(kToolbarMargin));
}

}  // namespace

/// @brief 【截图工具栏】【停靠布局】返回当前窗口工具栏停靠的屏幕边缘
///
/// 纵向排列停靠右侧，横向排列停靠配置的顶部或底部；配置为右侧时横向排列停靠底部，
/// 因此全屏切换排列方向即在右侧与该横向边缘之间切换。
/// @return 截图会话且配置为固定位置时返回停靠边；跟随选区或打开图片的独立窗口返回空
std::optional<markshot::ToolbarDockEdge> ShotWindow::toolbarDockEdge() const
{
    if (m_controls.m_toolbarPosition == markshot::CaptureToolbarPosition::FollowSelection
        || m_imageNavigationEnabled) {
        return std::nullopt;
    }
    if (m_controls.m_toolbarVerticalLayout) {
        return markshot::ToolbarDockEdge::Right;
    }
    return m_controls.m_toolbarPosition == markshot::CaptureToolbarPosition::ScreenTop
        ? markshot::ToolbarDockEdge::Top
        : markshot::ToolbarDockEdge::Bottom;
}

/// @brief 【截图工具栏】【排列方向】切换主工具栏横纵方向；停靠模式下同时切换停靠边
/// @return 无返回值
void ShotWindow::toggleToolbarLayout()
{
    m_controls.m_toolbarVerticalLayout = !m_controls.m_toolbarVerticalLayout;
    m_toolbarUserPlaced = false;
    m_actionToolbarUserPlaced = false;
    applyToolbarLayout();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
}

/// @brief 【截图工具栏】【排列方向】按当前横纵状态切换主、副工具栏组件方向
/// @return 无返回值
void ShotWindow::applyToolbarLayout()
{
    if (!m_controls.m_toolbarPanel) {
        return;
    }

    m_controls.m_toolbarPanel->setOrientation(m_controls.m_toolbarVerticalLayout ? Qt::Vertical : Qt::Horizontal);
    m_controls.m_toolbarPanel->adjustSize();
    if (m_controls.m_actionToolbarPanel) {
        // 1. 副工具栏默认纵向；停靠顶部或底部时改为横向，与主工具栏排在同一行
        const std::optional<markshot::ToolbarDockEdge> edge = toolbarDockEdge();
        const bool dockedInRow = edge && *edge != markshot::ToolbarDockEdge::Right;
        m_controls.m_actionToolbarPanel->setOrientation(dockedInRow ? Qt::Horizontal : Qt::Vertical);
    }
}

/// @brief 【截图工具栏】【停靠布局】按配置的摆放方式恢复初始排列方向
///
/// 停靠右侧时主工具栏纵向排列，停靠顶部、底部和跟随选区时主工具栏横向排列。
/// 窗口创建、切换图片导航和退出全屏标注时调用，覆盖会话内的临时横纵切换。
/// @return 无返回值
void ShotWindow::restoreConfiguredToolbarLayout()
{
    m_controls.m_toolbarVerticalLayout = !m_imageNavigationEnabled
        && m_controls.m_toolbarPosition == markshot::CaptureToolbarPosition::ScreenRight;
    applyToolbarLayout();
}

/// @brief 【截图工具栏】【边界限制】把工具栏几何限制在窗口内并保留 8px 边距
/// @param toolbarGeometry 待限制的工具栏几何
/// @return 限制后的几何，尺寸不变
QRect ShotWindow::clampedToolbarGeometry(QRect toolbarGeometry) const
{
    toolbarGeometry.moveLeft(std::clamp(toolbarGeometry.left(), 8, std::max(8, width() - toolbarGeometry.width() - 8)));
    toolbarGeometry.moveTop(std::clamp(toolbarGeometry.top(), 8, std::max(8, height() - toolbarGeometry.height() - 8)));
    return toolbarGeometry;
}

/// @brief 【截图工具栏】【主面板定位】按拖拽、停靠、全屏或跟随选区的优先级摆放主工具栏
/// @return 无返回值
void ShotWindow::updateToolbarGeometry()
{
    // 【截图】【选区尺寸】进入编辑、全屏标注切换时同步 HUD 显隐
    updateSelectionSizePanelGeometry();
    if (!m_controls.m_toolbar || !hasUsableSelection()) {
        return;
    }

    m_controls.m_toolbar->adjustSize();
    if (m_toolbarUserPlaced) {
        const QSize toolbarSize = m_controls.m_toolbar->sizeHint();
        QRect toolbarGeometry = m_controls.m_toolbar->geometry();
        toolbarGeometry.setSize(toolbarSize);
        m_controls.m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (const std::optional<markshot::ToolbarDockEdge> edge = toolbarDockEdge()) {
        // 1. 【截图工具栏】【停靠布局】按配置停靠屏幕顶部、底部或右侧，位置不随选区变化
        m_controls.m_toolbar->setGeometry(currentDockedToolbarLayout(m_controls, *edge, rect()).toolbar);
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (m_fullscreenAnnotation && m_controls.m_toolbarVerticalLayout) {
        // 2. 【截图工具栏】【纵向排列】全屏纵向时贴靠左侧并垂直居中，避免竖条压在画面中央
        const QSize toolbarSize = m_controls.m_toolbar->sizeHint();
        const QRect toolbarGeometry(QPoint(12, qRound((height() - toolbarSize.height()) / 2.0)), toolbarSize);
        m_controls.m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (m_imageNavigationEnabled && m_fullscreenAnnotation) {
        const QSize toolbarSize = m_controls.m_toolbar->sizeHint();
        const QRect toolbarGeometry(QPoint(qRound((width() - toolbarSize.width()) / 2.0), 12), toolbarSize);
        m_controls.m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_controls.m_toolbar->sizeHint();
    int x = qRound(selection.center().x() - toolbarSize.width() / 2.0);
    int y = qRound(selection.bottom() + kToolbarMargin);

    x = std::clamp(x, 8, std::max(8, width() - toolbarSize.width() - 8));
    if (y + toolbarSize.height() > height() - 8) {
        y = qRound(selection.top() - toolbarSize.height() - kToolbarMargin);
    }
    y = std::clamp(y, 8, std::max(8, height() - toolbarSize.height() - 8));
    m_controls.m_toolbar->setGeometry(x, y, toolbarSize.width(), toolbarSize.height());
    updateAnnotationPropertyPanelGeometry();
}

/// @brief 【截图工具栏】【副面板定位】按拖拽、停靠或避让选区与主工具栏的顺序摆放副工具栏
/// @return 无返回值
void ShotWindow::updateActionToolbarGeometry()
{
    // 【框选翻译】【译文浮条】浮条避让两条工具栏，副工具栏摆好后再刷新
    const auto updateTranslateResultBar = qScopeGuard([this] { ShotWindowSelectionTranslate(*this).updateResultBar(); });
    if (!m_controls.m_actionToolbar || !hasUsableSelection() || m_fullscreenAnnotation) {
        return;
    }

    m_controls.m_actionToolbar->adjustSize();
    if (m_actionToolbarUserPlaced) {
        const QSize toolbarSize = m_controls.m_actionToolbar->sizeHint();
        QRect toolbarGeometry = m_controls.m_actionToolbar->geometry();
        toolbarGeometry.setSize(toolbarSize);
        m_controls.m_actionToolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        return;
    }
    if (const std::optional<markshot::ToolbarDockEdge> edge = toolbarDockEdge()) {
        // 1. 【截图工具栏】【停靠布局】与主工具栏一起停靠屏幕边缘，不再避让选区
        m_controls.m_actionToolbar->setGeometry(currentDockedToolbarLayout(m_controls, *edge, rect()).actionToolbar);
        return;
    }

    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_controls.m_actionToolbar->sizeHint();
    const QRect selectionRect = selection.toAlignedRect();
    const QRect toolbarRect = m_controls.m_toolbar && m_controls.m_toolbar->isVisible() ? m_controls.m_toolbar->geometry() : QRect();
    const QRect propertyRect = m_controls.m_annotationPropertyPanel && m_controls.m_annotationPropertyPanel->isVisible()
        ? m_controls.m_annotationPropertyPanel->geometry()
        : QRect();

    auto clamped = [this, toolbarSize](QPoint topLeft) {
        const int x = std::clamp(topLeft.x(), 8, std::max(8, width() - toolbarSize.width() - 8));
        const int y = std::clamp(topLeft.y(), 8, std::max(8, height() - toolbarSize.height() - 8));
        return QRect(QPoint(x, y), toolbarSize);
    };
    auto clearOfPanels = [toolbarRect, propertyRect, selectionRect](const QRect &candidate) {
        const QRect padded = candidate.adjusted(-4, -4, 4, 4);
        return (toolbarRect.isNull() || !padded.intersects(toolbarRect))
            && (propertyRect.isNull() || !padded.intersects(propertyRect))
            && !padded.intersects(selectionRect);
    };

    const int selectionCenterY = qRound(selection.center().y() - toolbarSize.height() / 2.0);
    QVector<QRect> candidates = {
        clamped(QPoint(qRound(selection.right() + kToolbarMargin), selectionCenterY)),
        clamped(QPoint(qRound(selection.left() - toolbarSize.width() - kToolbarMargin), selectionCenterY)),
    };
    if (!toolbarRect.isNull()) {
        candidates.append(clamped(QPoint(toolbarRect.right() + kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.left() - toolbarSize.width() - kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.bottom() + kToolbarMargin)));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.top() - toolbarSize.height() - kToolbarMargin)));
    }

    for (const QRect &candidate : candidates) {
        if (clearOfPanels(candidate)) {
            m_controls.m_actionToolbar->setGeometry(candidate);
            return;
        }
    }
    m_controls.m_actionToolbar->setGeometry(candidates.first());
}

/// @brief 【标注属性】【面板定位】按工具栏位置摆放属性面板，避免压住工具栏
/// @return 无返回值
void ShotWindow::updateAnnotationPropertyPanelGeometry()
{
    if (!m_controls.m_annotationPropertyPanel) {
        return;
    }

    m_controls.m_annotationPropertyPanel->adjustSize();
    const QSize panelSize = m_controls.m_annotationPropertyPanel->sizeHint();
    const bool toolbarVisible = m_controls.m_toolbar && m_controls.m_toolbar->isVisible();
    QRect toolbarRect = toolbarVisible ? m_controls.m_toolbar->geometry() : QRect(8, 8, 0, 0);
    const std::optional<markshot::ToolbarDockEdge> edge = toolbarDockEdge();
    if (toolbarVisible && !m_toolbarUserPlaced && !m_actionToolbarUserPlaced && edge) {
        // 1. 【截图工具栏】【停靠布局】两条工具栏并排停靠时按合并区域避让，取计算值以免依赖副工具栏的刷新顺序
        toolbarRect |= currentDockedToolbarLayout(m_controls, *edge, rect()).actionToolbar;
    }
    int x = toolbarRect.left();
    int y = toolbarRect.bottom() + kToolbarMargin;
    if (m_controls.m_toolbarVerticalLayout && toolbarVisible) {
        // 2. 【截图工具栏】【纵向排列】属性面板放在竖条右侧顶端，右侧放不下时移到竖条左侧
        x = toolbarRect.right() + kToolbarMargin;
        y = toolbarRect.top();
        if (x + panelSize.width() > width() - 8) {
            x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
        }
    } else if (y + panelSize.height() > height() - 8) {
        y = toolbarRect.top() - panelSize.height() - kToolbarMargin;
    }
    if (x + panelSize.width() > width() - 8) {
        x = toolbarRect.right() - panelSize.width();
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_controls.m_annotationPropertyPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
    updatePropertyColorDialogGeometry();
    updatePropertyFontPanelGeometry();
    // 形状网格弹层锚定属性栏，属性栏移动后同步跟随
    updateShapeMarkerPopupGeometry();
}
