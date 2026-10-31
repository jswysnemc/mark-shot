#include "app_config_store.h"
#include "pinned_window/pinned_image_window.h"
#include "selection_translate/selection_translate_placement.h"
#include "selection_translate/translate_language_popup.h"
#include "selection_translate/translate_result_bar.h"
#include "shot_window.h"
#include "shot_window_internal.h"
#include "shot_window_selection_translate.h"
#include "support/isolated_config.h"
#include "translation_layout/translation_seed.h"
#include "ui/compact_toolbar.h"
#include "ui/icons.h"

#include <QApplication>
#include <QClipboard>
#include <QJsonObject>
#include <QKeyEvent>
#include <QPainter>
#include <QPushButton>
#include <QTemporaryDir>
#include <QtTest>

#include <cstdlib>
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

/// 夹具译文：两个识别框在同一行，合并成一段后整句翻译
const QString kJapaneseText = QStringLiteral("[Japanese] Hello world");

/**
 * 写入测试配置：OCR 与翻译都使用夹具命令，贴图不置顶。
 * @param translateMode 翻译夹具模式，未知模式让翻译进程失败。
 * @param ocrMode OCR 夹具模式，未知模式让识别进程失败。
 * @param autoOcr 贴图窗口是否自动识别。
 * @return 写入成功时返回 true。
 */
bool writeFixtureConfig(const QString &translateMode, const QString &ocrMode = QStringLiteral("--ocr-fixture"),
                        bool autoOcr = false)
{
    return markshot::writeAppConfigRoot({
        {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                            {QStringLiteral("command"), fixtureCommand(ocrMode)}}},
        {QStringLiteral("translation"),
         QJsonObject{{QStringLiteral("command"), fixtureCommand(translateMode) + QStringLiteral(" {input}")},
                     {QStringLiteral("targetLanguage"), QStringLiteral("Simplified Chinese")},
                     {QStringLiteral("autoAfterOcr"), false}}},
        {QStringLiteral("pinnedWindow"),
         QJsonObject{{QStringLiteral("autoOcr"), autoOcr}, {QStringLiteral("alwaysOnTop"), false}}},
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

/// @brief 检查框选翻译模式：目标语言弹层、段落译文写回与缓存、原文译文切换、退出、转贴图交接与浮条位置
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
     * 在已打开的语言弹层中从当前语言移动若干行后回车确认。
     * @param window 截图窗口。
     * @param steps 移动的行数，正数向下，负数向上。
     * @return 弹层已打开并完成选择时返回 true。
     */
    static bool pickInOpenPopup(ShotWindow &window, int steps)
    {
        translate::TranslateLanguagePopup *popup = languagePopup(window);
        if (!popup || !popup->isVisible()) {
            return false;
        }
        for (int i = 0; i < std::abs(steps); ++i) {
            pressKey(popup, steps > 0 ? Qt::Key_Down : Qt::Key_Up);
        }
        pressKey(popup, Qt::Key_Return);
        return !popup->isVisible();
    }

    /**
     * 点工具栏翻译按钮打开语言弹层，从当前语言向下移动若干行后回车确认。
     * @param window 截图窗口。
     * @param steps 向下移动的行数。
     * @return 弹层已打开并完成选择时返回 true。
     */
    static bool chooseLanguage(ShotWindow &window, int steps)
    {
        window.translateSelection();
        return pickInOpenPopup(window, steps);
    }

    /**
     * 从译文浮条的语言按钮打开语言弹层，再移动若干行后回车确认。
     * @param window 截图窗口。
     * @param steps 移动的行数，正数向下，负数向上。
     * @return 弹层已打开并完成选择时返回 true。
     */
    static bool chooseLanguageFromBar(ShotWindow &window, int steps)
    {
        translate::TranslateResultBar *bar = resultBar(window);
        auto *button = bar ? bar->findChild<QPushButton *>(QStringLiteral("translateLanguageButton")) : nullptr;
        if (!button) {
            return false;
        }
        button->click();
        return pickInOpenPopup(window, steps);
    }

    /** 查找浮条上的按钮，参数为截图窗口和按钮对象名，返回按钮，找不到时为空。 */
    static QPushButton *barButton(ShotWindow &window, const QString &name)
    {
        translate::TranslateResultBar *bar = resultBar(window);
        return bar ? bar->findChild<QPushButton *>(name) : nullptr;
    }

    /** 返回工具栏翻译按钮是否处于选中态，参数为截图窗口，找不到按钮时返回 false。 */
    static bool translateButtonActive(ShotWindow &window)
    {
        const QString key = markshot::ui::actionName(ShotWindow::Action::Translate);
        for (markshot::ui::CompactToolbar *toolbar :
             {window.m_controls.m_actionToolbarPanel, window.m_controls.m_toolbarPanel}) {
            if (QPushButton *button = toolbar ? toolbar->button(key) : nullptr) {
                return button->property("active").toBool();
            }
        }
        return false;
    }

    /** 返回当前语言的译文，参数为截图窗口，不在翻译模式或尚未翻译时为空指针。 */
    static const translate::SelectionTranslation *currentTranslation(ShotWindow &window)
    {
        const auto &session = window.m_controls.m_selectionTranslateSession;
        return session ? session->current() : nullptr;
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

    /** 验证选中语言后写回整段译文、记住语言、工具栏翻译按钮选中，浮条不压住工具栏，无参数和返回值。 */
    void choosingLanguageWritesTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(!translateButtonActive(*window));
        // 内置列表依次是简体中文、繁体中文、英语、日语，从当前的简体中文下移三行选日语
        QVERIFY(chooseLanguage(*window, 3));

        QVERIFY(!samePixels(window->m_frozenFrame, before));
        const translate::SelectionTranslation *translation = currentTranslation(*window);
        QVERIFY(translation);
        // 同一行的两个识别框合并成一段整句翻译
        QCOMPARE(translation->text, kJapaneseText);
        QCOMPARE(translation->blocks.size(), 1);
        QVERIFY(translateButtonActive(*window));
        const QJsonObject config = markshot::readAppConfigRoot().value(QStringLiteral("translation")).toObject();
        QCOMPARE(config.value(QStringLiteral("targetLanguage")).toString(), QStringLiteral("Japanese"));

        translate::TranslateResultBar *bar = resultBar(*window);
        QVERIFY(bar && bar->isVisible());
        for (const QWidget *panel :
             {window->m_controls.m_toolbar, window->m_controls.m_actionToolbar, window->m_selectionSizePanel}) {
            if (panel && panel->isVisible()) {
                QVERIFY2(!bar->geometry().intersects(panel->geometry()), qPrintable(panel->objectName()));
            }
        }
        // 浮条文字跟随截图窗口的工具栏配色，构造后再换样式表时文字会留在旧配色
        QCOMPARE(barButton(*window, QStringLiteral("translateCopyButton"))->palette().color(QPalette::ButtonText),
                 window->m_toolbarAppearance.palette.text);
        QVERIFY(barButton(*window, QStringLiteral("translateTranslatedButton"))->isChecked());
    }

    /** 验证复制按钮把整段译文放进剪贴板，无参数和返回值。 */
    void copyButtonCopiesTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        QVERIFY(chooseLanguage(*window, 3));
        QPushButton *copy = barButton(*window, QStringLiteral("translateCopyButton"));
        QVERIFY(copy);
        copy->click();
        QCOMPARE(QGuiApplication::clipboard()->text(), kJapaneseText);
    }

    /** 验证原文与译文按钮只交换像素、不再调用翻译器，且始终停留在翻译模式，无参数和返回值。 */
    void viewSwitchKeepsTranslationMode()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        const QImage japanese = window->m_frozenFrame;
        // 翻译器改为失败，切换仍能显示译文说明用的是缓存
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fail")));

        QPushButton *original = barButton(*window, QStringLiteral("translateOriginalButton"));
        QPushButton *translated = barButton(*window, QStringLiteral("translateTranslatedButton"));
        QVERIFY(original && translated);
        original->click();
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(window->m_controls.m_selectionTranslateSession.has_value());
        QVERIFY(!window->m_controls.m_selectionTranslateSession->showingTranslation);
        QVERIFY(original->isChecked());
        QVERIFY(resultBar(*window)->isVisible());
        QVERIFY(translateButtonActive(*window));

        translated->click();
        QVERIFY(samePixels(window->m_frozenFrame, japanese));
        QVERIFY(translated->isChecked());
        QVERIFY(window->m_controls.m_selectionTranslateSession->showingTranslation);
    }

    /** 验证浮条退出按钮写回原图、丢弃缓存、隐藏浮条并取消工具栏选中态，无参数和返回值。 */
    void exitButtonLeavesTranslationMode()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        // 显示原文时退出同样回到原图
        barButton(*window, QStringLiteral("translateOriginalButton"))->click();
        QPushButton *exit = barButton(*window, QStringLiteral("translateExitButton"));
        QVERIFY(exit);
        exit->click();
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(!window->m_controls.m_selectionTranslateSession.has_value());
        QVERIFY(!resultBar(*window)->isVisible());
        QVERIFY(!translateButtonActive(*window));
    }

    /** 验证翻译模式内再点工具栏翻译按钮直接退出，退出后再点重新弹出语言列表，无参数和返回值。 */
    void translateButtonTogglesMode()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        window->translateSelection();
        QVERIFY(!window->m_controls.m_selectionTranslateSession.has_value());
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(!languagePopup(*window)->isVisible());
        QVERIFY(!translateButtonActive(*window));

        window->translateSelection();
        QVERIFY(languagePopup(*window)->isVisible());
        // 弹层开着时再点只关闭弹层
        window->translateSelection();
        QVERIFY(!languagePopup(*window)->isVisible());
        QVERIFY(!window->m_controls.m_selectionTranslateSession.has_value());
    }

    /** 验证翻译模式内换语言复用识别结果，切回翻译过的语言直接取缓存，无参数和返回值。 */
    void switchingLanguageReusesRecognitionAndCache()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        const QImage japanese = window->m_frozenFrame;
        // 识别改为失败，换语言仍能翻译说明复用了已有识别结果
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fixture"), QStringLiteral("--ocr-fail")));
        // 当前语言已是日语，上移一行选英语
        QVERIFY(chooseLanguageFromBar(*window, -1));
        const auto &session = window->m_controls.m_selectionTranslateSession;
        QVERIFY(session.has_value());
        QCOMPARE(session->language, QStringLiteral("English"));
        QCOMPARE(currentTranslation(*window)->text, QStringLiteral("[English] Hello world"));
        // 没有字体时两种译文都画成同样的方框，改为比较保存的原图，确认译文排在原图上
        QVERIFY(samePixels(session->originalPixels, before.copy(session->imageRect)));

        // 翻译器也改为失败，切回日语只能来自缓存
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fail"), QStringLiteral("--ocr-fail")));
        QVERIFY(chooseLanguageFromBar(*window, 1));
        QCOMPARE(session->language, QStringLiteral("Japanese"));
        QCOMPARE(currentTranslation(*window)->text, kJapaneseText);
        QVERIFY(samePixels(window->m_frozenFrame, japanese));
        window->translateSelection();
        QVERIFY(samePixels(window->m_frozenFrame, before));
    }

    /** 验证显示原文时换语言，翻译完成后切到新译文，无参数和返回值。 */
    void switchingLanguageWhileShowingOriginalShowsTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        QVERIFY(chooseLanguage(*window, 3));
        barButton(*window, QStringLiteral("translateOriginalButton"))->click();
        QVERIFY(chooseLanguageFromBar(*window, -1));
        const auto &session = window->m_controls.m_selectionTranslateSession;
        QVERIFY(session->showingTranslation);
        QVERIFY(samePixels(window->m_frozenFrame.copy(session->imageRect), currentTranslation(*window)->pixels));
        QVERIFY(barButton(*window, QStringLiteral("translateTranslatedButton"))->isChecked());
    }

    /** 验证重新翻译失败时保留上一次译文、语言和浮条，无参数和返回值。 */
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
        const auto &session = window->m_controls.m_selectionTranslateSession;
        QVERIFY(session.has_value());
        QCOMPARE(session->language, QStringLiteral("Japanese"));
        QVERIFY(!session->translations.contains(QStringLiteral("English")));
        QVERIFY(resultBar(*window)->isVisible());
        ShotWindowSelectionTranslate(*window).exitTranslation();
        QVERIFY(samePixels(window->m_frozenFrame, before));
    }

    /** 验证首次翻译失败时不进入翻译模式，截图保持原样，无参数和返回值。 */
    void failedFirstTranslateStaysOutOfMode()
    {
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fail")));
        auto window = createWindow();
        QVERIFY(window);
        const QImage before = window->m_frozenFrame;
        QVERIFY(chooseLanguage(*window, 3));
        QVERIFY(!window->m_controls.m_selectionTranslateSession.has_value());
        QVERIFY(samePixels(window->m_frozenFrame, before));
        QVERIFY(!resultBar(*window) || !resultBar(*window)->isVisible());
        QVERIFY(!translateButtonActive(*window));
    }

    /** 验证翻译模式下转贴图交出原图、识别结果和译文，截图窗口仍显示译文，无参数和返回值。 */
    void pinHandsOverOriginalAndTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        const QImage plain = window->renderedSelection();
        QVERIFY(chooseLanguage(*window, 3));
        const QImage japanese = window->m_frozenFrame;

        markshot::translation_layout::TranslationSeed seed;
        const QImage output = ShotWindowSelectionTranslate(*window).renderSelectionForPin(&seed);
        QVERIFY(samePixels(output, plain));
        QVERIFY(samePixels(window->m_frozenFrame, japanese));
        QCOMPARE(seed.sourceTokens.size(), 2);
        // 选区与翻译区域重合，识别框坐标不变
        QCOMPARE(seed.sourceTokens.first().imageRect, QRectF(40, 40, 60, 40));
        QCOMPARE(seed.blocks.size(), 1);
        QCOMPARE(seed.blocks.first().text, kJapaneseText);
        QVERIFY(seed.showTranslation);

        // 显示原文时转贴图，译文仍交出，贴图先显示原文
        barButton(*window, QStringLiteral("translateOriginalButton"))->click();
        markshot::translation_layout::TranslationSeed originalSeed;
        QVERIFY(samePixels(ShotWindowSelectionTranslate(*window).renderSelectionForPin(&originalSeed), plain));
        QCOMPARE(originalSeed.blocks.size(), 1);
        QVERIFY(!originalSeed.showTranslation);
    }

    /** 验证贴图窗口收下交接的译文后直接显示，不再自动识别，右键切换沿用已有译文，无参数和返回值。 */
    void pinnedWindowUsesHandedOverTranslation()
    {
        auto window = createWindow();
        QVERIFY(window);
        QVERIFY(chooseLanguage(*window, 3));
        markshot::translation_layout::TranslationSeed seed;
        const QImage output = ShotWindowSelectionTranslate(*window).renderSelectionForPin(&seed);
        // 贴图打开自动识别，但识别与翻译都改为失败，再识别或再翻译都会丢掉交接的内容
        QVERIFY(writeFixtureConfig(QStringLiteral("--translate-fail"), QStringLiteral("--ocr-fail"), true));

        std::unique_ptr<QWidget> widget(markshot::shot::createPinnedImageWindow(output, QPoint(80, 80), &seed));
        auto *pinned = dynamic_cast<markshot::shot::PinnedImageWindow *>(widget.get());
        QVERIFY(pinned);
        pinned->setAttribute(Qt::WA_DeleteOnClose, false);
        pinned->show();
        QVERIFY(QTest::qWaitForWindowExposed(pinned));
        QCoreApplication::processEvents();
        QVERIFY(!pinned->m_ocrTask);
        QCOMPARE(pinned->m_ocrTokens.size(), 2);
        QVERIFY(pinned->m_translationActive);
        QCOMPARE(pinned->m_translationBlocks.size(), 1);
        QCOMPARE(pinned->m_translationBlocks.first().text, kJapaneseText);

        pinned->setTranslationActive(false);
        pinned->requestTranslation();
        QVERIFY(pinned->m_translationActive);
        QVERIFY(!pinned->m_translationTask);
    }

    /** 验证贴图窗口自己翻译时同样按段落排版，无参数和返回值。 */
    void pinnedWindowTranslatesParagraphs()
    {
        QImage image(400, 200, QImage::Format_RGB32);
        image.fill(Qt::white);
        markshot::shot::PinnedImageWindow pinned(image, QPoint(80, 80));
        pinned.setAttribute(Qt::WA_DeleteOnClose, false);
        pinned.show();
        QVERIFY(QTest::qWaitForWindowExposed(&pinned));
        pinned.requestTranslation();
        QTRY_VERIFY_WITH_TIMEOUT(pinned.m_translationActive, 5000);
        QCOMPARE(pinned.m_translationBlocks.size(), 1);
        QCOMPARE(pinned.m_translationBlocks.first().text, QStringLiteral("[Simplified Chinese] Hello world"));
        QVERIFY(!pinned.m_translatedTokens.isEmpty());
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
