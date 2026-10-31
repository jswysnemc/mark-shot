#include "shot_window.h"
#include "support/isolated_config.h"
#include "ui/toolbar_panel_style.h"

#include <QApplication>
#include <QMouseEvent>
#include <QPaintEvent>
#include <QTemporaryDir>
#include <QtTest>

namespace {

/** 记录窗口实际绘制区域，不消费绘制事件。 */
class PaintRegions final : public QObject {
public:
    QRegion dirty;

    /**
     * 记录真实画布绘制区域。
     * @param watched 绘制目标。
     * @param event 窗口事件。
     * @return false，保持正常绘制。
     */
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        Q_UNUSED(watched)
        if (event->type() == QEvent::Paint) {
            dirty |= static_cast<QPaintEvent *>(event)->region();
        }
        return false;
    }
};

/**
 * 向画布发送鼠标移动并完成增量绘制。
 * @param window 截图窗口。
 * @param point 窗口内坐标。
 * @return 无返回值。
 */
void movePointer(ShotWindow &window, QPoint point)
{
    QMouseEvent event(QEvent::MouseMove, QPointF(point),
                      QPointF(window.mapToGlobal(point)), Qt::NoButton, Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &event);
    QTest::qWait(30);
}

}  // namespace

class SelectionHoverRepaintTest final : public QObject {
    Q_OBJECT
private slots:
    /** 验证悬停候选切换后旧边框外侧像素纳入重绘，无参数和返回值。 */
    void clearsPreviousHoverStroke()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        const QRect first(80, 80, 200, 180);
        const QRect second(450, 320, 200, 180);
        ShotWindow window(image, {}, QRect(0, 0, 800, 600),
                          {{first, 1}, {second, 1}}, true);
        window.resize(800, 600);
        PaintRegions paints;
        window.installEventFilter(&paints);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTest::qWait(400);

        // 1. 【选区测试】【边框清除】固定候选，直接验证实际损伤计算，避免放大镜或整窗刷新掩盖遗漏
        window.applyHoveredRect(first);
        const QRegion dirty = window.applyHoveredRect(second);
        const QRectF previous = window.imageRectToWidget(QRectF(first));
        const QPoint outsideLeft(qFloor(previous.left()) - 1, qRound(previous.center().y()));
        const QPoint outsideRight(qCeil(previous.right()), qRound(previous.center().y()));
        QVERIFY2(dirty.contains(outsideLeft), "旧悬停框左侧描边未纳入重绘");
        QVERIFY2(dirty.contains(outsideRight), "旧悬停框右侧描边未纳入重绘");
    }

    /** 验证点击尚未形成拖动时保留识别选区，无参数和返回值。 */
    void keepsRecognizedSelectionForClick()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        const QRect recognized(80, 80, 200, 180);
        ShotWindow window(image, {}, QRect(0, 0, 800, 600), {{recognized, 1}}, true);
        window.resize(800, 600);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        window.applyHoveredRect(recognized);
        window.m_dragging = true;
        window.m_selectionClickStart = QPointF(120, 120);
        window.m_startupHoverImagePoint = QPointF(122, 121);
        window.m_startupHoverValid = true;

        window.dismissRecognizedSelectionOnManualDragIfEnabled();
        QVERIFY(window.m_hoveredWindowRect.has_value());
    }

    /** 验证手动拖动超过点击阈值后识别选区立即消失，无参数和返回值。 */
    void dropsRecognizedSelectionWhenDragStarts()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        const QRect recognized(80, 80, 200, 180);
        ShotWindow window(image, {}, QRect(0, 0, 800, 600), {{recognized, 1}}, true);
        window.resize(800, 600);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        window.applyHoveredRect(recognized);
        window.m_dragging = true;
        window.m_selectionClickStart = window.imageToWidget(QPointF(120, 120));
        window.m_startupHoverImagePoint = QPointF(160, 170);
        window.m_startupHoverValid = true;

        const QRegion dirty = window.dismissRecognizedSelectionOnManualDragIfEnabled();
        QVERIFY(!window.m_hoveredWindowRect.has_value());
        QVERIFY(!dirty.isEmpty());
        QVERIFY(window.applyHoveredRect(recognized).isEmpty() == false);
    }

    /** 验证激活按钮使用较浅主题色，悬停使用更纯的主题色和深色前景，无参数和返回值。 */
    void toolbarHoverIsPurerThemeThanActive()
    {
        const QString style = markshot::theme::panelStyleSheet();
        QVERIFY(style.contains(QStringLiteral(
            "QPushButton[active=\"true\"] { color: #E5E7EB; background: rgba(153, 246, 228, 78);")));
        QVERIFY(style.contains(QStringLiteral(
            "QPushButton:hover { color: #042F2E; background: #2DD4BF;")));
        QVERIFY(style.contains(QStringLiteral(
            "QPushButton[active=\"true\"]:hover { color: #042F2E; background: #2DD4BF;")));
    }
};

/**
 * 使用隔离配置运行真实窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    SelectionHoverRepaintTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "selection_hover_repaint_test.moc"
