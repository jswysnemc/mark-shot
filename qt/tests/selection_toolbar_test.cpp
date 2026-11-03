#include "selection_aspect/selection_aspect_ratio.h"
#include "selection_aspect/selection_hud_layout.h"
#include "selection_aspect/selection_ratio_popup.h"
#include "selection_aspect/selection_size_field.h"
#include "selection_aspect/selection_size_panel.h"
#include "shot_window_popup_geometry.h"

#include <QLineEdit>
#include <QSignalSpy>
#include <QWheelEvent>
#include <QtTest/QtTest>

#include <optional>

namespace aspect = markshot::selection_aspect;

namespace markshot::i18n {

/**
 * 测试内使用的翻译桩，直接返回英文原文，避免链接配置存储。
 * @param source 英文原文。
 * @return 原文。
 */
QString translate(const QString &source)
{
    return source;
}

}  // namespace markshot::i18n

namespace {

/**
 * 沿 HUD 中线从左向右扫描，返回第一个点击后发出比例信号的位置，即锁形按钮。
 * @param panel 尺寸 HUD。
 * @param spy 监听 aspectRatioChanged 的信号探针。
 * @return 锁形按钮坐标，找不到时返回空值。
 */
std::optional<QPoint> findLockPoint(aspect::SelectionSizePanel &panel, QSignalSpy &spy)
{
    const int y = panel.height() / 2;
    for (int x = 1; x < panel.width(); ++x) {
        const QPoint point(x, y);
        if (panel.childAt(point)) {
            continue;
        }
        QTest::mouseClick(&panel, Qt::LeftButton, Qt::NoModifier, point);
        if (!spy.isEmpty()) {
            return point;
        }
    }
    return std::nullopt;
}

/**
 * 发送一次纵向滚轮事件。
 * @param widget 接收控件。
 * @param up true 表示向上滚动。
 * @param modifiers 键盘修饰键。
 * @return 无返回值。
 */
void sendWheel(QWidget *widget, bool up, Qt::KeyboardModifiers modifiers)
{
    const QPointF position(widget->width() / 2.0, widget->height() / 2.0);
    QWheelEvent event(position, widget->mapToGlobal(position), QPoint(), QPoint(0, up ? 120 : -120), Qt::NoButton,
                      modifiers, Qt::NoScrollPhase, false);
    QApplication::sendEvent(widget, &event);
}

}  // namespace

class SelectionToolbarTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证比例预设顺序与下标查找。
     * @return 无返回值。
     */
    void presetsAndLookup()
    {
        const QVector<aspect::AspectRatioPreset> presets = aspect::aspectRatioPresets();
        QStringList labels;
        for (const auto &preset : presets) {
            labels << preset.label;
        }
        QCOMPARE(labels, QStringList({"Free", "1:1", "4:3", "16:9", "9:16", "21:9"}));
        QCOMPARE(aspect::presetIndexForRatio(0.0), 0);
        QCOMPARE(aspect::presetIndexForRatio(16.0 / 9.0), 3);
        QCOMPARE(aspect::presetIndexForRatio(9.0 / 16.0), 4);
        QCOMPARE(aspect::presetIndexForRatio(2.0), -1);
    }

    /**
     * 验证尺寸联动：自由比例只做范围限制，锁定比例按修改侧联动并在越界时反推。
     * @return 无返回值。
     */
    void linkedSizeKeepsRatioAndBounds()
    {
        using aspect::SizeField;
        QCOMPARE(aspect::linkedSize(QSize(5000, 0), SizeField::Width, 0.0, QSize(1000, 800)), QSize(1000, 1));
        QCOMPARE(aspect::linkedSize(QSize(1920, 1), SizeField::Width, 16.0 / 9.0, QSize(1920, 1080)), QSize(1920, 1080));
        QCOMPARE(aspect::linkedSize(QSize(1, 100), SizeField::Height, 2.0, QSize(1000, 800)), QSize(200, 100));
        QCOMPARE(aspect::linkedSize(QSize(1000, 1), SizeField::Width, 1.0, QSize(1000, 800)), QSize(800, 800));
    }

    /**
     * 验证 HUD 默认放在选区左上角外侧，并在左右屏幕边缘内收。
     * @return 无返回值。
     */
    void hudFollowsTopLeftAndClampsHorizontally()
    {
        aspect::SelectionHudLayoutInput input;
        input.hudSize = QSize(200, 26);
        input.bounds = QRect(0, 0, 1000, 800);

        input.selection = QRect(100, 100, 300, 200);
        QCOMPARE(aspect::placeSelectionHud(input), std::optional<QRect>(QRect(100, 68, 200, 26)));

        input.selection = QRect(0, 100, 300, 200);
        QCOMPARE(aspect::placeSelectionHud(input)->left(), 6);

        input.selection = QRect(900, 100, 80, 200);
        QCOMPARE(aspect::placeSelectionHud(input)->left(), 794);
    }

    /**
     * 验证顶部空间不足或与文本输入框冲突时退到选区内侧，全部冲突时返回空值。
     * @return 无返回值。
     */
    void hudAvoidsTopEdgeAndObstacles()
    {
        aspect::SelectionHudLayoutInput input;
        input.hudSize = QSize(200, 26);
        input.bounds = QRect(0, 0, 1000, 800);

        input.selection = QRect(100, 10, 300, 200);
        QCOMPARE(aspect::placeSelectionHud(input), std::optional<QRect>(QRect(106, 16, 200, 26)));

        input.selection = QRect(100, 100, 300, 200);
        input.obstacles = {QRect(80, 60, 240, 30)};
        QCOMPARE(aspect::placeSelectionHud(input), std::optional<QRect>(QRect(106, 106, 200, 26)));

        input.selection = QRect(0, 0, 1000, 800);
        input.obstacles = {QRect(0, 0, 400, 100)};
        QVERIFY(!aspect::placeSelectionHud(input).has_value());
    }

    /**
     * 验证字体弹层列表与底部字号、B/I/U 控件行上下分区且互不重叠，矮窗口下仍保留最少行数。
     * @return 无返回值。
     */
    void fontPanelListDoesNotOverlapControls()
    {
        namespace popup = markshot::shot_popup;
        // 1. 字体较多时列表限制为 10 行，控件行贴底
        const popup::FontPanelLayout layout = popup::fontPanelLayout(260, 24, 300, 26, QSize(1920, 1080));
        QCOMPARE(layout.panelSize, QSize(260, 6 * 2 + 6 + 26 + 10 * 24));
        QVERIFY(!layout.listRect.intersects(layout.controlsRect));
        QCOMPARE(layout.controlsRect.top() - layout.listRect.bottom() - 1, 6);
        QCOMPARE(layout.controlsRect.bottom() + 1 + 6, layout.panelSize.height());
        QCOMPARE(layout.listRect.height(), 10 * 24);

        // 搜索框占用列表上方的高度，列表仍然不与底部控件重叠
        const popup::FontPanelLayout filtered = popup::fontPanelLayout(360, 24, 300, 26, QSize(1920, 1080), 6, 6, 28);
        QVERIFY(!filtered.listRect.intersects(filtered.controlsRect));
        QCOMPARE(filtered.listRect.top(), 6 + 28 + 6);
        QCOMPARE(filtered.listRect.height(), 10 * 24);

        // 2. 窗口过矮时压缩列表而不是压到控件行
        const popup::FontPanelLayout compact = popup::fontPanelLayout(260, 24, 300, 26, QSize(400, 150));
        QVERIFY(!compact.listRect.intersects(compact.controlsRect));
        QVERIFY(compact.listRect.height() >= 3 * 24);
        QCOMPARE(compact.controlsRect.height(), 26);
    }

    /**
     * 验证弹层位于锚点面板下方 6-8px，底部空间不足时翻转到上方，并始终留在窗口内。
     * @return 无返回值。
     */
    void popupSitsBelowPropertyBar()
    {
        namespace popup = markshot::shot_popup;
        const QRect bounds(0, 0, 1000, 800);
        const QRect propertyBar(100, 200, 400, 40);

        const QRect below = popup::popupBelowAnchor(propertyBar, 120, QSize(240, 120), bounds);
        const int gap = below.top() - (propertyBar.bottom() + 1);
        QVERIFY(gap >= 6 && gap <= 8);
        QCOMPARE(below.left(), 120);

        const QRect nearBottom(100, 700, 400, 40);
        const QRect above = popup::popupBelowAnchor(nearBottom, 120, QSize(240, 120), bounds);
        QCOMPARE(nearBottom.top() - (above.bottom() + 1), popup::kPopupGap);

        const QRect clamped = popup::popupBelowAnchor(propertyBar, 900, QSize(240, 120), bounds);
        QCOMPARE(clamped.right() + 1, 1000 - popup::kPopupEdgeMargin);
    }

    /**
     * 验证形状弹层作为避让项时 HUD 退让，完全冲突时隐藏。
     * @return 无返回值。
     */
    void hudHidesBehindShapePopup()
    {
        aspect::SelectionHudLayoutInput input;
        input.hudSize = QSize(200, 26);
        input.bounds = QRect(0, 0, 1000, 800);
        input.selection = QRect(100, 100, 300, 200);
        // 弹层覆盖选区左上角内外两侧以及左下角外侧
        input.obstacles = {QRect(90, 40, 260, 300)};
        QVERIFY(!aspect::placeSelectionHud(input).has_value());
    }

    /**
     * 验证选区控制点为以端点为中心的 6px 方块。
     * @return 无返回值。
     */
    void selectionHandleIsSixPixels()
    {
        const QRectF handle = markshot::shot_popup::selectionHandleRect(QPointF(50.0, 80.0));
        QCOMPARE(handle.size(), QSizeF(6.0, 6.0));
        QCOMPARE(handle.center(), QPointF(50.0, 80.0));
    }

    /**
     * 验证锁定比例时 Enter 提交宽度或高度会联动另一侧。
     * @return 无返回值。
     */
    void fieldCommitAppliesLinkedSize()
    {
        QWidget host;
        aspect::SelectionSizePanel panel(&host);
        panel.setValues(QSize(160, 90), QSize(1920, 1080), 16.0 / 9.0);
        QSignalSpy sizeSpy(&panel, &aspect::SelectionSizePanel::sizeApplied);
        QSignalSpy releaseSpy(&panel, &aspect::SelectionSizePanel::focusReleaseRequested);

        const auto fields = panel.findChildren<aspect::SelectionSizeField *>();
        QCOMPARE(fields.size(), 2);
        auto *widthField = fields.at(0);
        auto *heightField = fields.at(1);

        widthField->beginEditing();
        widthField->findChild<QLineEdit *>()->setText(QStringLiteral("320"));
        QTest::keyClick(widthField->findChild<QLineEdit *>(), Qt::Key_Return);
        QCOMPARE(sizeSpy.size(), 1);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(320, 180));
        QCOMPARE(releaseSpy.size(), 1);
        QVERIFY(!widthField->isEditing());

        heightField->beginEditing();
        heightField->findChild<QLineEdit *>()->setText(QStringLiteral("360"));
        QTest::keyClick(heightField->findChild<QLineEdit *>(), Qt::Key_Return);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(640, 360));
    }

    /**
     * 验证 Esc 放弃输入、实时同步不覆盖正在编辑的文本、Tab 切换到另一字段。
     * @return 无返回值。
     */
    void editingCancelPreserveAndTab()
    {
        QWidget host;
        aspect::SelectionSizePanel panel(&host);
        panel.setValues(QSize(200, 100), QSize(1000, 800), 0.0);
        QSignalSpy sizeSpy(&panel, &aspect::SelectionSizePanel::sizeApplied);
        const auto fields = panel.findChildren<aspect::SelectionSizeField *>();
        auto *widthField = fields.at(0);
        auto *heightField = fields.at(1);
        QLineEdit *widthEditor = widthField->findChild<QLineEdit *>();

        // 1. 实时同步时保留用户输入
        widthField->beginEditing();
        widthEditor->setText(QStringLiteral("77"));
        panel.setValues(QSize(210, 100), QSize(1000, 800), 0.0);
        QCOMPARE(widthEditor->text(), QStringLiteral("77"));
        QVERIFY(panel.isInteracting());

        // 2. Esc 放弃输入
        QTest::keyClick(widthEditor, Qt::Key_Escape);
        QVERIFY(sizeSpy.isEmpty());
        QVERIFY(!panel.isInteracting());

        // 3. Tab 提交后切到高度字段
        widthField->beginEditing();
        widthEditor->setText(QStringLiteral("300"));
        QTest::keyClick(widthEditor, Qt::Key_Tab);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(300, 100));
        QVERIFY(heightField->isEditing());
    }

    /**
     * 验证滚轮按步长调整，Shift 为 10 倍步长，越界时不发信号。
     * @return 无返回值。
     */
    void wheelStepsSize()
    {
        QWidget host;
        aspect::SelectionSizePanel panel(&host);
        panel.setValues(QSize(200, 100), QSize(1000, 800), 0.0);
        QSignalSpy sizeSpy(&panel, &aspect::SelectionSizePanel::sizeApplied);
        const auto fields = panel.findChildren<aspect::SelectionSizeField *>();

        sendWheel(fields.at(0), true, Qt::NoModifier);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(201, 100));
        sendWheel(fields.at(1), false, Qt::ShiftModifier);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(200, 90));

        panel.setValues(QSize(1000, 100), QSize(1000, 800), 0.0);
        sendWheel(fields.at(0), true, Qt::NoModifier);
        QVERIFY(sizeSpy.isEmpty());
    }

    /**
     * 验证编辑中滚轮实时应用尺寸，输入框与联动后的实际值保持同步。
     * @return 无返回值。
     */
    void wheelWhileEditingSyncsLive()
    {
        QWidget host;
        aspect::SelectionSizePanel panel(&host);
        const QSize maximum(1000, 800);
        panel.setValues(QSize(199, 100), maximum, 2.0);
        QSignalSpy sizeSpy(&panel, &aspect::SelectionSizePanel::sizeApplied);
        // 1. 模拟截图窗口：应用尺寸后把实际选区回写给 HUD
        connect(&panel, &aspect::SelectionSizePanel::sizeApplied, &panel,
                [&panel, maximum](QSize size) { panel.setValues(size, maximum, 2.0); });
        const auto fields = panel.findChildren<aspect::SelectionSizeField *>();
        QCOMPARE(fields.size(), 2);
        auto *widthField = fields.at(0);
        auto *heightField = fields.at(1);
        QLineEdit *widthEditor = widthField->findChild<QLineEdit *>();
        QVERIFY(widthEditor);

        // 2. 编辑中滚轮立即应用，输入框保持编辑并显示新值，高度按比例联动
        widthField->beginEditing();
        sendWheel(widthEditor, true, Qt::NoModifier);
        QCOMPARE(sizeSpy.size(), 1);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(200, 100));
        QVERIFY(widthField->isEditing());
        QCOMPARE(widthEditor->text(), QStringLiteral("200"));
        QCOMPARE(heightField->value(), 100);

        sendWheel(widthEditor, true, Qt::ShiftModifier);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(210, 105));
        QCOMPARE(widthEditor->text(), QStringLiteral("210"));
        QCOMPARE(heightField->value(), 105);

        // 3. 手动输入后再滚轮，以输入值为基准继续调整
        widthEditor->setText(QStringLiteral("300"));
        sendWheel(widthEditor, false, Qt::NoModifier);
        QCOMPARE(sizeSpy.takeFirst().at(0).toSize(), QSize(299, 150));
        QCOMPARE(widthEditor->text(), QStringLiteral("299"));

        // 4. 越界被限制时输入框回写实际值，Enter 结束编辑不重复应用
        panel.setValues(QSize(1000, 500), maximum, 2.0);
        sendWheel(widthEditor, true, Qt::NoModifier);
        QVERIFY(sizeSpy.isEmpty());
        QCOMPARE(widthEditor->text(), QStringLiteral("1000"));
        QTest::keyClick(widthEditor, Qt::Key_Return);
        QVERIFY(sizeSpy.isEmpty());
        QVERIFY(!widthField->isEditing());
        QCOMPARE(widthField->value(), 1000);
    }

    /**
     * 验证锁形按钮：未锁定时锁定当前宽高比，已锁定时解锁。
     * @return 无返回值。
     */
    void lockToggleUsesCurrentRatio()
    {
        QWidget host;
        host.resize(600, 200);
        aspect::SelectionSizePanel panel(&host);
        panel.setValues(QSize(200, 100), QSize(1000, 800), 0.0);
        host.show();
        QVERIFY(QTest::qWaitForWindowExposed(&host));
        QSignalSpy ratioSpy(&panel, &aspect::SelectionSizePanel::aspectRatioChanged);

        const std::optional<QPoint> lockPoint = findLockPoint(panel, ratioSpy);
        QVERIFY(lockPoint.has_value());
        QCOMPARE(ratioSpy.takeFirst().at(0).toReal(), 2.0);

        panel.setValues(QSize(200, 100), QSize(1000, 800), 2.0);
        QTest::mouseClick(&panel, Qt::LeftButton, Qt::NoModifier, *lockPoint);
        QCOMPARE(ratioSpy.takeFirst().at(0).toReal(), 0.0);
    }

    /**
     * 验证比例下拉：点击触发区打开弹层，键盘选择后发出比例并关闭，再次点击触发区关闭弹层。
     * @return 无返回值。
     */
    void ratioPopupSelectsPreset()
    {
        QWidget host;
        host.resize(800, 600);
        auto *panel = new aspect::SelectionSizePanel(&host);
        panel->setValues(QSize(200, 100), QSize(1000, 800), 0.0);
        panel->move(20, 20);
        host.show();
        QVERIFY(QTest::qWaitForWindowExposed(&host));
        QSignalSpy ratioSpy(panel, &aspect::SelectionSizePanel::aspectRatioChanged);

        // 1. 锁形按钮右侧即比例触发区
        const std::optional<QPoint> lockPoint = findLockPoint(*panel, ratioSpy);
        QVERIFY(lockPoint.has_value());
        ratioSpy.clear();
        const QPoint ratioPoint(panel->width() - 20, panel->height() / 2);
        QTest::mouseClick(panel, Qt::LeftButton, Qt::NoModifier, ratioPoint);
        auto *popup = host.findChild<aspect::SelectionRatioPopup *>();
        QVERIFY(popup);
        QVERIFY(popup->isVisible());
        QVERIFY(panel->isInteracting());
        QVERIFY(popup->geometry().top() >= panel->geometry().bottom());

        // 2. 向下移动一行选择 1:1
        QTest::keyClick(popup, Qt::Key_Down);
        QTest::keyClick(popup, Qt::Key_Return);
        QCOMPARE(ratioSpy.size(), 1);
        QCOMPARE(ratioSpy.takeFirst().at(0).toReal(), 1.0);
        QVERIFY(!popup->isVisible());

        // 3. 打开后再次点击触发区只关闭弹层
        QTest::mouseClick(panel, Qt::LeftButton, Qt::NoModifier, ratioPoint);
        QVERIFY(popup->isVisible());
        QTest::mouseClick(panel, Qt::LeftButton, Qt::NoModifier, ratioPoint);
        QVERIFY(!popup->isVisible());
        QVERIFY(ratioSpy.isEmpty());
    }

    /**
     * 验证尺寸 HUD 跟随工具栏亮色配色：底板为浅色、文字为深色，暗色仍保持原有深色底板。
     * @return 无返回值。
     */
    void hudFollowsLightToolbarPalette()
    {
        const auto backgroundLightness = [](const markshot::ui::ToolbarPalette *palette) {
            QWidget host;
            host.resize(400, 120);
            aspect::SelectionSizePanel panel(&host);
            if (palette) {
                panel.setToolbarPalette(*palette);
            }
            panel.setValues(QSize(200, 100), QSize(1000, 800), 0.0);
            panel.resize(panel.sizeHint());
            const QImage image = panel.grab().toImage();
            // 1. 取 HUD 中部偏左的空白底板像素，避开文字与图标
            return image.pixelColor(3, image.height() / 2).lightness();
        };
        const markshot::ui::ToolbarPalette light = markshot::ui::toolbarPalette(markshot::ui::UiThemeMode::Light);
        QVERIFY(backgroundLightness(nullptr) < 80);
        QVERIFY(backgroundLightness(&light) > 200);

        const aspect::hud_style::HudColors lightColors = aspect::hud_style::hudColors(light);
        QVERIFY(lightColors.text.lightness() < 128);
        QVERIFY(lightColors.panelFill.lightness() > 200);
        const aspect::hud_style::HudColors darkColors = aspect::hud_style::hudColors();
        QCOMPARE(darkColors.panelFill, QColor(8, 13, 19, 228));
        QCOMPARE(darkColors.text, QColor(229, 231, 235));
    }
};

QTEST_MAIN(SelectionToolbarTest)

#include "selection_toolbar_test.moc"
