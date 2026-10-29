#include "app_config_store.h"
#include "shot_window.h"
#include "support/isolated_config.h"
#include "ui/compact_toolbar.h"

#include <QApplication>
#include <QTemporaryDir>
#include <QtTest>

#include <algorithm>
#include <memory>

namespace {

// 停靠时工具栏与屏幕边缘的距离
constexpr int kEdge = 12;
// 选区靠近右下角，跟随选区时主工具栏会翻到选区上方，停靠模式下应保持贴底
const QRectF kCornerSelection(1200, 700, 300, 200);

}  // namespace

class ToolbarDockWindowTest final : public QObject {
    Q_OBJECT
public:
    /**
     * 创建按指定摆放方式进入选区编辑的截图窗口。
     * @param position 工具栏摆放方式。
     * @param selection 图像坐标下的选区。
     * @return 截图窗口；窗口未能显示时返回空。
     */
    static std::unique_ptr<ShotWindow> createWindow(markshot::CaptureToolbarPosition position,
                                                    const QRectF &selection = kCornerSelection)
    {
        // 1. 窗口与冻结帧同为 1600x1000，图像坐标与窗口坐标一致
        QImage image(1600, 1000, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        auto window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 1600, 1000),
                                                   QVector<markshot::WindowInfo>{}, false);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->resize(1600, 1000);
        window->show();
        if (!QTest::qWaitForWindowExposed(window.get())) {
            return nullptr;
        }

        // 2. 写入会话的摆放方式并恢复初始排列，再进入选区编辑
        window->m_controls.m_toolbarPosition = position;
        window->restoreConfiguredToolbarLayout();
        window->m_selection = selection;
        window->enterSelectionEditing(QPointF(10, 10));
        return window;
    }

    /** 读取主工具栏几何，参数为截图窗口，返回窗口坐标矩形。 */
    static QRect toolbarRect(const ShotWindow &window)
    {
        return window.m_controls.m_toolbar->geometry();
    }

    /** 读取副工具栏几何，参数为截图窗口，返回窗口坐标矩形。 */
    static QRect actionRect(const ShotWindow &window)
    {
        return window.m_controls.m_actionToolbar->geometry();
    }

private slots:
    /** 验证配置文件中的 capture.toolbarPosition 在窗口创建时生效，无参数和返回值。 */
    void configFileSelectsDockPosition()
    {
        const QStringList key{QStringLiteral("capture"), QStringLiteral("toolbarPosition")};
        QVERIFY(markshot::writeAppConfigValue(key, QStringLiteral("right")));
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::white);
        ShotWindow window(image, QString(), QRect(0, 0, 800, 600), QVector<markshot::WindowInfo>{}, false);
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        QCOMPARE(window.m_controls.m_toolbarPosition, markshot::CaptureToolbarPosition::ScreenRight);
        QVERIFY(window.m_controls.m_toolbarVerticalLayout);
        QCOMPARE(window.m_controls.m_toolbarPanel->orientation(), Qt::Vertical);
        QVERIFY(markshot::writeAppConfigValue(key, QStringLiteral("selection")));
    }

    /** 验证默认跟随选区时主工具栏仍贴在选区下方、副工具栏纵向，无参数和返回值。 */
    void followSelectionKeepsLegacyPlacement()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::FollowSelection, QRectF(200, 150, 600, 300));
        QVERIFY(window);
        QVERIFY(!window->m_controls.m_toolbarVerticalLayout);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Vertical);
        QCOMPARE(toolbarRect(*window).top(), 460);
    }

    /** 验证底部停靠时两条工具栏横向贴底居中排成一行，且不随选区移动，无参数和返回值。 */
    void bottomDockPinsBothToolbars()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenBottom);
        QVERIFY(window);
        QCOMPARE(window->m_controls.m_toolbarPanel->orientation(), Qt::Horizontal);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Horizontal);
        const QRect toolbar = toolbarRect(*window);
        const QRect action = actionRect(*window);
        QCOMPARE(window->height() - 1 - std::max(toolbar.bottom(), action.bottom()), kEdge);
        QCOMPARE(action.left(), toolbar.right() + 11);
        QVERIFY(qAbs(toolbar.left() - (window->width() - 1 - action.right())) <= 1);
        // 1. 选区底边离屏幕底部不足一条工具栏时，跟随模式会翻到选区上方，停靠模式保持贴底
        QVERIFY(toolbar.top() > qRound(kCornerSelection.bottom()));

        window->m_selection = QRectF(40, 40, 300, 200);
        window->refreshViewGeometry();
        QCOMPARE(toolbarRect(*window), toolbar);
        QCOMPARE(actionRect(*window), action);
    }

    /** 验证顶部停靠时两条工具栏横向贴顶居中排成一行，切换排列方向在右侧与顶部之间往返，无参数和返回值。 */
    void topDockPinsBothToolbars()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenTop, QRectF(600, 20, 400, 300));
        QVERIFY(window);
        QCOMPARE(window->m_controls.m_toolbarPanel->orientation(), Qt::Horizontal);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Horizontal);
        const QRect toolbar = toolbarRect(*window);
        const QRect action = actionRect(*window);
        QCOMPARE(std::min(toolbar.top(), action.top()), kEdge);
        QCOMPARE(action.left(), toolbar.right() + 11);
        QVERIFY(qAbs(toolbar.left() - (window->width() - 1 - action.right())) <= 1);

        // 1. 换到右下角的选区后，两条工具栏仍停在顶部原位
        window->m_selection = kCornerSelection;
        window->refreshViewGeometry();
        QCOMPARE(toolbarRect(*window), toolbar);
        QCOMPARE(actionRect(*window), action);

        // 2. 切换排列方向先停靠右侧，再切回时回到顶部而不是底部
        window->toggleToolbarLayout();
        QCOMPARE(window->width() - 1 - std::max(toolbarRect(*window).right(), actionRect(*window).right()), kEdge);
        window->toggleToolbarLayout();
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Horizontal);
        QCOMPARE(toolbarRect(*window), toolbar);
        QCOMPARE(actionRect(*window), action);
    }

    /** 验证顶部停靠进入全屏标注后主工具栏仍贴顶居中，无参数和返回值。 */
    void fullscreenKeepsTopEdge()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenTop);
        QVERIFY(window);
        window->enterFullscreenAnnotation(false);
        const QRect toolbar = toolbarRect(*window);
        QCOMPARE(toolbar.top(), kEdge);
        QVERIFY(qAbs(toolbar.left() - (window->width() - 1 - toolbar.right())) <= 1);
    }

    /** 验证右侧停靠时两条工具栏纵向贴右排成一列，无参数和返回值。 */
    void rightDockPinsBothToolbars()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenRight, QRectF(40, 40, 300, 200));
        QVERIFY(window);
        QCOMPARE(window->m_controls.m_toolbarPanel->orientation(), Qt::Vertical);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Vertical);
        const QRect toolbar = toolbarRect(*window);
        const QRect action = actionRect(*window);
        QCOMPARE(window->width() - 1 - toolbar.right(), kEdge);
        QVERIFY(!toolbar.intersects(action));
        if (toolbar.height() + 10 + action.height() <= window->height() - 2 * kEdge) {
            // 1. 一列放得下时副工具栏接在主工具栏下方
            QCOMPARE(action.top(), toolbar.bottom() + 11);
        } else {
            // 2. 高度不足时副工具栏并排在主工具栏左侧并底对齐，1080p 屏幕属于这一情况
            QCOMPARE(action.right(), toolbar.left() - 11);
            QCOMPARE(action.bottom(), toolbar.bottom());
        }
    }

    /** 验证停靠模式下切换排列方向会把两条工具栏移到另一条边，无参数和返回值。 */
    void toggleLayoutSwitchesDockEdge()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenBottom);
        QVERIFY(window);
        window->toggleToolbarLayout();
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Vertical);
        QCOMPARE(window->width() - 1 - std::max(toolbarRect(*window).right(), actionRect(*window).right()), kEdge);
        QVERIFY(!toolbarRect(*window).intersects(actionRect(*window)));
    }

    /** 验证全屏标注沿用停靠边，退出后恢复配置的排列方向，无参数和返回值。 */
    void fullscreenKeepsDockEdge()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenBottom);
        QVERIFY(window);
        window->enterFullscreenAnnotation(false);
        QVERIFY(window->m_controls.m_actionToolbar->isHidden());
        QRect toolbar = toolbarRect(*window);
        QCOMPARE(window->height() - 1 - toolbar.bottom(), kEdge);
        QVERIFY(qAbs(toolbar.left() - (window->width() - 1 - toolbar.right())) <= 1);

        window->toggleToolbarLayout();
        toolbar = toolbarRect(*window);
        QCOMPARE(window->width() - 1 - toolbar.right(), kEdge);

        window->leaveFullscreenAnnotation();
        QVERIFY(!window->m_controls.m_toolbarVerticalLayout);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Horizontal);
        QCOMPARE(window->height() - 1 - std::max(toolbarRect(*window).bottom(), actionRect(*window).bottom()), kEdge);
    }

    /** 验证打开图片的独立窗口不停靠工具栏，保留顶部居中布局，无参数和返回值。 */
    void imageWindowIgnoresDock()
    {
        auto window = createWindow(markshot::CaptureToolbarPosition::ScreenRight);
        QVERIFY(window);
        window->setImageNavigationEnabled(true);
        QVERIFY(!window->toolbarDockEdge().has_value());
        QVERIFY(!window->m_controls.m_toolbarVerticalLayout);
        QCOMPARE(window->m_controls.m_actionToolbarPanel->orientation(), Qt::Vertical);
        window->enterFullscreenAnnotation(true);
        QCOMPARE(toolbarRect(*window).top(), kEdge);
    }

    /** 提供停靠位置与贴着工具栏的选区，无参数和返回值。 */
    void sizeHudStaysUnderDockedToolbars_data()
    {
        QTest::addColumn<int>("position");
        QTest::addColumn<QRectF>("selection");
        QTest::newRow("right") << static_cast<int>(markshot::CaptureToolbarPosition::ScreenRight)
                               << QRectF(1300, 500, 280, 200);
        QTest::newRow("top") << static_cast<int>(markshot::CaptureToolbarPosition::ScreenTop)
                             << QRectF(500, 10, 600, 985);
    }

    /** 验证选区贴着停靠的工具栏、状态条只能与工具栏重叠时，重叠处仍由工具栏响应点击，无参数和返回值。 */
    void sizeHudStaysUnderDockedToolbars()
    {
        QFETCH(int, position);
        QFETCH(QRectF, selection);
        auto window = createWindow(static_cast<markshot::CaptureToolbarPosition>(position), selection);
        QVERIFY(window);
        // 1. 状态条在工具栏移动后的下一轮事件循环重新避让
        QTest::qWait(20);
        QWidget *sizePanel = window->m_selectionSizePanel;
        QVERIFY(sizePanel && sizePanel->isVisible());
        bool overlapped = false;
        for (const QRect &toolbar : {toolbarRect(*window), actionRect(*window)}) {
            const QRect overlap = sizePanel->geometry() & toolbar;
            if (overlap.isEmpty()) {
                continue;
            }
            overlapped = true;
            QWidget *hit = window->childAt(overlap.center());
            QVERIFY(hit);
            QVERIFY(hit != sizePanel && !sizePanel->isAncestorOf(hit));
        }
        // 2. 选区上方、内侧与下方都没有空位，状态条必然与工具栏重叠，确保覆盖到层级处理
        QVERIFY(overlapped);
    }

    /** 提供停靠位置参数，无参数和返回值。 */
    void panelsAvoidDockedToolbars_data()
    {
        QTest::addColumn<int>("position");
        QTest::newRow("bottom") << static_cast<int>(markshot::CaptureToolbarPosition::ScreenBottom);
        QTest::newRow("right") << static_cast<int>(markshot::CaptureToolbarPosition::ScreenRight);
        QTest::newRow("top") << static_cast<int>(markshot::CaptureToolbarPosition::ScreenTop);
    }

    /** 验证停靠时属性面板与侧边面板都不压住任一工具栏，无参数和返回值。 */
    void panelsAvoidDockedToolbars()
    {
        QFETCH(int, position);
        auto window = createWindow(static_cast<markshot::CaptureToolbarPosition>(position));
        QVERIFY(window);
        window->setTool(ShotWindow::Tool::Pen);
        window->updateAnnotationPropertyPanel();
        QVERIFY(window->m_controls.m_annotationPropertyPanel->isVisible());
        window->updateOpenWithPanelGeometry();
        window->updateExtensionPanelGeometry();

        const QRect toolbar = toolbarRect(*window);
        const QRect action = actionRect(*window);
        for (QWidget *panel : {window->m_controls.m_annotationPropertyPanel,
                               window->m_controls.m_openWithPanel,
                               window->m_controls.m_extensionPanel}) {
            const QRect geometry = panel->geometry();
            QVERIFY2(!geometry.intersects(toolbar), qPrintable(panel->objectName()));
            QVERIFY2(!geometry.intersects(action), qPrintable(panel->objectName()));
            QVERIFY(window->rect().contains(geometry));
        }
    }
};

/**
 * 使用隔离配置运行工具栏停靠窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    ToolbarDockWindowTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "toolbar_dock_window_test.moc"
