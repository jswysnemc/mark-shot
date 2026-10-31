#include "app_config_store.h"
#include "selection_translate/selection_translate_placement.h"
#include "selection_translate/translate_language_popup.h"
#include "selection_translate/translate_result_bar.h"
#include "shot_window.h"
#include "shot_window_internal.h"
#include "shot_window_selection_translate.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QClipboard>
#include <QJsonObject>
#include <QKeyEvent>
#include <QLabel>
#include <QPainter>
#include <QPushButton>
#include <QTemporaryDir>
#include <QtTest>

#include <memory>

namespace {

namespace translate = markshot::selection_translate;

/// 测试使用的截图选区，夹具识别框落在选区左上部
const QRect kSelection(100, 100, 400, 300);

/**
 * 返回夹具程序的命令行。
 * @param mode 夹具模式参数。
 * @return 已转义的命令行，夹具要求的文件路径由调用方追加或用占位符替换。
 */
QString fixtureCommand(const QString &mode)
{
    return markshot::shot::shellQuote(QString::fromUtf8(MARK_SHOT_TEST_OCR_FIXTURE_PATH)) + QLatin1Char(' ') + mode;
}

/**
 * 写入测试配置：OCR 与翻译都使用夹具命令。
 * @param translateMode 翻译夹具模式，未知模式让翻译进程失败。
 * @return 写入成功时返回 true。
 */
bool writeFixtureConfig(const QString &translateMode)
{
    return markshot::writeAppConfigRoot({
        {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                            {QStringLiteral("command"), fixtureCommand(QStringLiteral("--ocr-fixture"))}}},
        {QStringLiteral("translation"),
         QJsonObject{{QStringLiteral("command"), fixtureCommand(translateMode) + QStringLiteral(" {input}")},
                     {QStringLiteral("targetLanguage"), QStringLiteral("Simplified Chinese")},
                     {QStringLiteral("autoAfterOcr"), false}}},
    });
}

/**
 * 判断两张图片的像素是否完全一致。
 * @param left 第一张图片。
 * @param right 第二张图片。
 * @return 尺寸与像素都一致时返回 true。
 */
bool samePixels(const QImage &left, const QImage &right)
{
    return left.convertToFormat(QImage::Format_ARGB32) == right.convertToFormat(QImage::Format_ARGB32);
}

}  // namespace

/// @brief 检查框选翻译的目标语言弹层、译文写回、复制、还原与浮条位置
class SelectionTranslateTest final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建已确定选区并进入编辑的截图窗口，夹具识别框所在位置画上深色块，便于比较原图与译文。
     * @return 截图窗口，窗口未能显示时返回空指针。
     */
    static std::unique_ptr<ShotWindow> createWindow()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(255, 255, 255));
        QPainter painter(&image);
        painter.fillRect(QRect(kSelection.topLeft() + QPoint(40, 40), QSize(140, 40)), QColor(30, 30, 30));
        painter.end();
        auto window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 800, 600),
                                                   QVector<markshot::WindowInfo>{}, false);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->resize(800, 600);
        window->show();
        if (!QTest::qWaitForWindowExposed(window.get())) {
            return nullptr;
        }
        window->m_selection = QRectF(kSelection);
        window->enterSelectionEditing(QPointF(10, 10));
        return window;
    }

    /** 查找截图窗口的目标语言弹层，参数为截图窗口，返回弹层，尚未创建时为空。 */
    static translate::TranslateLanguagePopup *languagePopup(ShotWindow &window)
    {
        return window.findChild<translate::TranslateLanguagePopup *>(QStringLiteral("translateLanguagePopup"));
    }

    /** 查找截图窗口的译文浮条，参数为截图窗口，返回浮条，尚未创建时为空。 */
    static translate::TranslateResultBar *resultBar(ShotWindow &window)
    {
        return window.findChild<translate::TranslateResultBar *>(QStringLiteral("translateResultBar"));
    }

    /** 向弹层发送一次按键，参数为弹层和按键，无返回值。 */
    static void pressKey(QWidget *popup, Qt::Key key)
    {
        QKeyEvent press(QEvent::KeyPress, key, Qt::NoModifier);
        QCoreApplication::sendEvent(popup, &press);
    }

    /**
     * 打开语言弹层，从当前语言向下移动若干行后回车确认。
     * @param window 截图窗口。
     * @param steps 向下移动的行数。
     * @return 弹层已打开并完成选择时返回 true。
     */
    static bool chooseLanguage(ShotWindow &window, int steps)
    {
        window.translateSelection();
        translate::TranslateLanguagePopup *popup = languagePopup(window);
        if (!popup || !popup->isVisible()) {
            return false;
        }
        for (int i = 0; i < steps; ++i) {
            pressKey(popup, Qt::Key_Down);
        }
        pressKey(popup, Qt::Key_Return);
        return !popup->isVisible();
    }

private slots:
    /** 每个用例前写入夹具配置，无参数和返回值。 */
    void init()
    {
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fixture")));
    }

    /** 验证点击翻译先弹出语言列表，Esc 关闭后不改动截图，无参数和返回值。 */
    void translateOpensLanguagePopup()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        window->translateSelection();
        translate::TranslateLanguagePopup *popup = languagePopup(*window);
        QVERIFY(popup && popup->isVisible());
        QVERIFY(QRect(QPoint(0, 0), window->size()).contains(popup->geometry()));

        pressKey(popup, Qt::Key_Escape);
        QVERIFY(!popup->isVisible());
        QVERIFY(window->isVisible());
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(!resultBar(*window) || !resultBar(*window)->isVisible());
    }

    /** 验证选中语言后写回译文、记住语言，并在译文旁显示不压住工具栏的浮条，无参数和返回值。 */
    void choosingLanguageWritesTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        // 内置列表依次是简体中文、繁体中文、英语、日语，从当前的简体中文下移三行选日语
        QVERIFY(chooseLanguage(*window, 3));

        QVERIFY(!samePixels(window->m_frozenFrame, before));
        QVERIFY(window->m_controls.m_selectionTranslation.has_value());
        QVERIFY(window->m_controls.m_selectionTranslation->translatedText.contains(QStringLiteral("[Japanese] Hello")));
        const QJsonObject translation = markshot::readAppConfigRoot().value(QStringLiteral("translation")).toObject();
        QCOMPARE(translation.value(QStringLiteral("targetLanguage")).toString(), QStringLiteral("Japanese"));

        translate::TranslateResultBar *bar = resultBar(*window);
        QVERIFY(bar && bar->isVisible());
        for (const QWidget *panel :
             {window->m_controls.m_toolbar, window->m_controls.m_actionToolbar, window->m_selectionSizePanel}) {
            if (panel && panel->isVisible()) {
                QVERIFY2(!bar->geometry().intersects(panel->geometry()), qPrintable(panel->objectName()));
            }
        }
        // 浮条文字跟随截图窗口的工具栏配色，构造后再换样式表时文字会留在旧配色
        const QColor text = window->m_toolbarAppearance.palette.text;
        QCOMPARE(bar->findChild<QPushButton *>(QStringLiteral("translateCopyButton"))->palette().color(QPalette::ButtonText),
                 text);
        QCOMPARE(bar->findChild<QLabel *>(QStringLiteral("translateLanguageLabel"))->palette().color(QPalette::WindowText),
                 window->m_toolbarAppearance.palette.mutedText);
    }

    /** 验证复制按钮把译文放进剪贴板，无参数和返回值。 */
    void copyButtonCopiesTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        QVERIFY(chooseLanguage(*window, 3));
        auto *copy = resultBar(*window)->findChild<QPushButton *>(QStringLiteral("translateCopyButton"));
        QVERIFY(copy);
        copy->click();
        const QString clipboard = QGuiApplication::clipboard()->text();
        QVERIFY2(clipboard.contains(QStringLiteral("[Japanese] Hello"))
                     && clipboard.contains(QStringLiteral("[Japanese] world")),
                 qPrintable(clipboard));
    }

    /** 验证还原按钮写回原图像素并隐藏浮条，无参数和返回值。 */
    void restoreButtonRestoresOriginal()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        auto *restore = resultBar(*window)->findChild<QPushButton *>(QStringLiteral("translateRestoreButton"));
        QVERIFY(restore);
        restore->click();
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(!window->m_controls.m_selectionTranslation.has_value());
        QVERIFY(!resultBar(*window)->isVisible());
    }

    /** 验证换一种语言重新翻译时识别的是原文，还原后仍回到最初的原图，无参数和返回值。 */
    void retranslateStartsFromOriginal()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        // 当前语言已是日语，上移一行选英语
        window->translateSelection();
        pressKey(languagePopup(*window), Qt::Key_Up);
        pressKey(languagePopup(*window), Qt::Key_Return);

        const translate::SelectionTranslateResult &result = *window->m_controls.m_selectionTranslation;
        QVERIFY2(result.translatedText.contains(QStringLiteral("[English] Hello"))
                     && !result.translatedText.contains(QStringLiteral("Japanese")),
                 qPrintable(result.translatedText));
        // 没有字体时两种译文都画成同样的方框，改为比较新结果保存的原图，确认识别前已先写回原文
        QVERIFY(samePixels(result.originalPixels, before.copy(result.imageRect)));
        ShotWindowSelectionTranslate(*window).restoreOriginal();
        QVERIFY(samePixels(window->m_frozenFrame, before));
    }

    /** 验证重新翻译失败时保留上一次译文和浮条，无参数和返回值。 */
    void failedRetranslateKeepsPreviousTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        const QImage japanese = window->m_frozenFrame;

        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fail")));
        ShotWindowSelectionTranslate(*window).translateTo(QStringLiteral("English"));
        QVERIFY(samePixels(window->m_frozenFrame, japanese));
        QVERIFY(window->m_controls.m_selectionTranslation.has_value());
        QVERIFY(resultBar(*window)->isVisible());
        ShotWindowSelectionTranslate(*window).restoreOriginal();
        QVERIFY(samePixels(window->m_frozenFrame, before));
    }

    /** 验证选区移开译文后浮条隐藏，移回后再次显示，无参数和返回值。 */
    void resultBarFollowsSelection()
    {
        auto window = createWindow();
        QVERIFY(window);
        QVERIFY(chooseLanguage(*window, 3));
        window->m_selection = QRectF(560, 420, 120, 80);
        window->updateActionToolbarGeometry();
        QVERIFY(!resultBar(*window)->isVisible());
        window->m_selection = QRectF(kSelection);
        window->updateActionToolbarGeometry();
        QVERIFY(resultBar(*window)->isVisible());
    }

    /** 验证语言弹层朝窗口内侧弹出，竖向工具栏在左右两侧，横向工具栏在上下两侧，无参数和返回值。 */
    void languagePopupOpensInward()
    {
        const QRect bounds(0, 0, 1000, 800);
        const QSize size(150, 200);
        QCOMPARE(translate::languagePopupGeometry(QRect(900, 300, 30, 30), true, size, bounds),
                 QRect(QPoint(744, 300), size));
        QCOMPARE(translate::languagePopupGeometry(QRect(40, 300, 30, 30), true, size, bounds),
                 QRect(QPoint(76, 300), size));
        QCOMPARE(translate::languagePopupGeometry(QRect(500, 740, 30, 30), false, size, bounds),
                 QRect(QPoint(380, 534), size));
        QCOMPARE(translate::languagePopupGeometry(QRect(500, 20, 30, 30), false, size, bounds),
                 QRect(QPoint(380, 56), size));
        // 放不下时平移进窗口，四周留出 8px
        QCOMPARE(translate::languagePopupGeometry(QRect(900, 700, 30, 30), true, size, bounds),
                 QRect(QPoint(744, 592), size));
    }

    /** 验证浮条依次尝试区域上方、下方，都被挡住时放进区域内侧，无参数和返回值。 */
    void resultBarAvoidsObstacles()
    {
        const QRect bounds(0, 0, 1000, 800);
        const QSize size(200, 30);
        const QRect region(300, 300, 400, 200);
        QCOMPARE(translate::resultBarGeometry(region, size, bounds, {}), QRect(QPoint(500, 264), size));
        const QRect aboveToolbar(450, 240, 300, 40);
        QCOMPARE(translate::resultBarGeometry(region, size, bounds, {aboveToolbar}), QRect(QPoint(500, 506), size));
        const QRect belowToolbar(450, 500, 300, 60);
        // 上下两行右对齐都被挡住时，在上方一行平移到障碍物左侧
        QCOMPARE(translate::resultBarGeometry(region, size, bounds, {aboveToolbar, belowToolbar}),
                 QRect(QPoint(246, 264), size));
        // 平移后离开区域横向范围的位置不用，上下都放不下时放进区域右上角内侧
        const QRect wideAbove(100, 240, 800, 40);
        const QRect wideBelow(100, 500, 800, 60);
        QCOMPARE(translate::resultBarGeometry(region, size, bounds, {wideAbove, wideBelow}),
                 QRect(QPoint(494, 306), size));
        // 选区尺寸提示占住上方左侧、工具栏占住下方时，上方一行平移到尺寸提示右侧
        const QRect selection(100, 100, 400, 300);
        const QSize wideBar(395, 30);
        const QRect sizePanel(100, 68, 272, 25);
        const QRect mainToolbar(8, 410, 645, 43);
        QCOMPARE(translate::resultBarGeometry(selection, wideBar, QRect(0, 0, 800, 600), {sizePanel, mainToolbar}),
                 QRect(QPoint(376, 64), wideBar));
        // 选区铺满屏幕时外侧没有位置，内侧顶部一行平移到右侧竖向工具栏左边
        const QRect fullScreen(0, 0, 1920, 1080);
        const QRect actionToolbar(1870, 20, 40, 400);
        QCOMPARE(translate::resultBarGeometry(fullScreen, size, fullScreen, {actionToolbar}),
                 QRect(QPoint(1666, 8), size));
        // 区域贴着窗口顶边时上方放不下，改放下方
        QCOMPARE(translate::resultBarGeometry(QRect(300, 4, 400, 200), size, bounds, {}),
                 QRect(QPoint(500, 210), size));
    }
};

/**
 * 使用隔离配置运行框选翻译回归；清掉会话变量，剪贴板只写进程内的 Qt 剪贴板。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    for (const char *name : {"XDG_SESSION_TYPE", "DISPLAY", "WAYLAND_DISPLAY"}) {
        qunsetenv(name);
    }
    QApplication app(argc, argv);
    SelectionTranslateTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "selection_translate_test.moc"
