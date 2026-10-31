#include "app_config_store.h"
#include "support/isolated_config.h"
#include "pinned_window/pinned_image_window.h"
#include "providers/provider_task.h"
#include "shell_command.h"
#include "shot_window.h"
#include "window_detection.h"

#include <QApplication>
#include <QClipboard>
#include <QDir>
#include <QFile>
#include <QJsonObject>
#include <QMouseEvent>
#include <QPointer>
#include <QTemporaryDir>
#include <QtTest>

#include <memory>

namespace {

/**
 * 【贴图测试】【鼠标输入】向真实窗口发送包含完整按键状态的事件
 * @param window 目标窗口
 * @param type 事件类型
 * @param point 窗口内坐标
 * @param button 本次改变的按钮
 * @param buttons 当前按住的按钮
 * @param modifiers 当前键盘修饰键
 * @return 无返回值
 */
void pointerEvent(QWidget &window, QEvent::Type type, QPoint point,
                  Qt::MouseButton button = Qt::NoButton, Qt::MouseButtons buttons = Qt::NoButton,
                  Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QMouseEvent event(type, QPointF(point), QPointF(window.mapToGlobal(point)),
                      button, buttons, modifiers);
    QCoreApplication::sendEvent(&window, &event);
}

}

class PinnedTextSelectionTest final : public QObject {
    Q_OBJECT
private slots:
    /// @brief 提供普通屏幕和高分屏的首次拖选场景，无参数和返回值
    void selectsTextAfterBackgroundOcr_data()
    {
        QTest::addColumn<qreal>("dpr");
        QTest::newRow("normal") << 1.0;
        QTest::newRow("hidpi") << 2.0;
    }

    /**
     * 【贴图测试】【后台识别】预识别完成后无需修饰键即可拖选文字，且不会移动图片
     * @return 无返回值
     */
    void selectsTextAfterBackgroundOcr()
    {
        QFETCH(qreal, dpr);
        const QString command = markshot::shot::shellQuote(QString::fromUtf8(MARK_SHOT_TEST_OCR_FIXTURE_PATH))
            + QStringLiteral(" --ocr-fixture");
        QVERIFY(markshot::writeAppConfigRoot({
            {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                                {QStringLiteral("command"), command}}},
            {QStringLiteral("pinnedWindow"), QJsonObject{{QStringLiteral("autoOcr"), false},
                {QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("textSelectionCopyEnabled"), true}}},
            {QStringLiteral("translation"), QJsonObject{{QStringLiteral("autoAfterOcr"), false}}}
        }));
        QImage image(400, 200, QImage::Format_RGB32);
        image.setDevicePixelRatio(dpr);
        image.fill(Qt::white);
        markshot::shot::PinnedImageWindow window(image, QPoint(80, 80));
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        const QPoint position = window.pos();
        // 1. 【贴图测试】【后台识别】不发送鼠标事件也应自动准备文字结果
        QTRY_VERIFY_WITH_TIMEOUT(!window.m_ocrTokens.isEmpty(), 3000);
        QVERIFY(window.cursor().shape() != Qt::BusyCursor);
        pointerEvent(window, QEvent::MouseMove, QPoint(42 / dpr, 60 / dpr));
        QCOMPARE(window.cursor().shape(), Qt::IBeamCursor);
        pointerEvent(window, QEvent::MouseButtonPress, QPoint(42 / dpr, 60 / dpr),
                     Qt::LeftButton, Qt::LeftButton);
        pointerEvent(window, QEvent::MouseMove, QPoint(175 / dpr, 60 / dpr), Qt::NoButton, Qt::LeftButton);
        pointerEvent(window, QEvent::MouseButtonRelease, QPoint(175 / dpr, 60 / dpr), Qt::LeftButton);
        // 2. 【贴图测试】【手势边界】释放后的指针移动不能改变选区
        pointerEvent(window, QEvent::MouseMove, QPoint(42 / dpr, 60 / dpr));
        QCOMPARE(window.pos(), position);
        // 3. 【贴图测试】【复制】通过实际快捷键检查选区文字
        QApplication::clipboard()->clear();
        QTRY_VERIFY_WITH_TIMEOUT(([&] {
            QTest::keyClick(&window, Qt::Key_C, Qt::ControlModifier);
            return QApplication::clipboard()->text() == QStringLiteral("Hello world");
        })(), 3000);
    }

    /// @brief 检查首次识别时不同拖动起点和开关组合，无参数和返回值
    void preservesImageMovement_data()
    {
        QTest::addColumn<bool>("selectionEnabled");
        QTest::addColumn<bool>("ocrEnabled");
        QTest::addColumn<bool>("emptyResult");
        QTest::addColumn<bool>("cachedText");
        QTest::newRow("blank-area") << true << true << false << false;
        QTest::newRow("no-text") << true << true << true << false;
        QTest::newRow("selection-disabled") << false << true << false << false;
        QTest::newRow("ocr-disabled") << true << false << false << false;
        QTest::newRow("cached-text") << true << true << false << true;
    }

    /// @brief 没有可选文字时保留图片移动，避免按需识别阻断原有交互，无参数和返回值
    void preservesImageMovement()
    {
        QFETCH(bool, selectionEnabled);
        QFETCH(bool, ocrEnabled);
        QFETCH(bool, emptyResult);
        QFETCH(bool, cachedText);
        const QString command = markshot::shot::shellQuote(QString::fromUtf8(MARK_SHOT_TEST_OCR_FIXTURE_PATH))
            + (emptyResult ? QStringLiteral(" --ocr-empty") : QStringLiteral(" --ocr-fixture"));
        QVERIFY(markshot::writeAppConfigRoot({
            {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), ocrEnabled},
                                                {QStringLiteral("command"), command}}},
            {QStringLiteral("pinnedWindow"), QJsonObject{{QStringLiteral("autoOcr"), false},
                {QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("textSelectionCopyEnabled"), selectionEnabled}}},
            {QStringLiteral("translation"), QJsonObject{{QStringLiteral("autoAfterOcr"), false}}}
        }));
        QImage image(400, 200, QImage::Format_RGB32);
        image.fill(Qt::white);
        markshot::shot::PinnedImageWindow window(image, QPoint(80, 80));
        if (cachedText) {
            window.m_ocrTokens = {{QStringLiteral("cached"), QRectF(40, 40, 70, 40), 0, 0, 1.0}};
        }
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        const bool attemptedBeforeDrag = window.m_textSelectionOcrAttempted;
        auto *taskBeforeDrag = window.m_ocrTask;
        if (cachedText) {
            QVERIFY(!attemptedBeforeDrag);
            QVERIFY(!taskBeforeDrag);
        }
        const QPoint position = window.pos();
        pointerEvent(window, QEvent::MouseButtonPress, QPoint(250, 150), Qt::LeftButton, Qt::LeftButton);
        pointerEvent(window, QEvent::MouseMove, QPoint(290, 150), Qt::NoButton, Qt::LeftButton);
        pointerEvent(window, QEvent::MouseButtonRelease, QPoint(290, 150), Qt::LeftButton);
        QCOMPARE(window.pos(), position + QPoint(40, 0));
        QCOMPARE(window.m_ocrTask, taskBeforeDrag);
        QCOMPARE(window.m_textSelectionOcrAttempted, attemptedBeforeDrag);
        if (!selectionEnabled || !ocrEnabled) {
            QVERIFY(!window.m_ocrTask);
            QVERIFY(!window.m_textSelectionOcrAttempted);
        }
    }

    /// @brief OCR 在拖动中完成时保持移动手势，不中途改成选字，无参数和返回值
    void backgroundCompletionDoesNotStealDrag()
    {
        const QString command = markshot::shot::shellQuote(QString::fromUtf8(MARK_SHOT_TEST_OCR_FIXTURE_PATH))
            + QStringLiteral(" --ocr-gated-fixture");
        QVERIFY(markshot::writeAppConfigRoot({
            {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                                {QStringLiteral("command"), command}}},
            {QStringLiteral("pinnedWindow"), QJsonObject{{QStringLiteral("autoOcr"), false},
                {QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("textSelectionCopyEnabled"), true}}},
            {QStringLiteral("translation"), QJsonObject{{QStringLiteral("autoAfterOcr"), true},
                {QStringLiteral("command"), command}}}}));
        QImage image(400, 200, QImage::Format_RGB32);
        image.fill(Qt::white);
        markshot::shot::PinnedImageWindow window(image, QPoint(80, 80));
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTRY_VERIFY_WITH_TIMEOUT(window.m_ocrTask, 3000);
        QVERIFY(window.m_ocrTokens.isEmpty());
        const QPoint position = window.pos();
        auto *task = window.m_ocrTask;
        // 1. 【贴图测试】【无等待拖动】起点之后会识别为文字，但当前应立即移动图片
        pointerEvent(window, QEvent::MouseButtonPress, QPoint(42, 60), Qt::LeftButton, Qt::LeftButton);
        pointerEvent(window, QEvent::MouseMove, QPoint(62, 60), Qt::NoButton, Qt::LeftButton);
        QCOMPARE(window.pos(), position + QPoint(20, 0));
        QCOMPARE(window.m_ocrTask, task);
        QCOMPARE(window.cursor().shape(), Qt::ClosedHandCursor);
        // 2. 【贴图测试】【结果交付】放行真实异步任务，移动模式和当前光标必须保持不变
        QFile ready(window.m_ocrTempPath + QStringLiteral(".ready"));
        QVERIFY(ready.open(QIODevice::WriteOnly));
        ready.close();
        QTRY_VERIFY_WITH_TIMEOUT(!window.m_ocrTokens.isEmpty(), 3000);
        QVERIFY(window.m_moving);
        QVERIFY(!window.m_selectingText);
        QVERIFY(!window.hasTextSelection());
        QVERIFY(!window.m_translationTask);
        QVERIFY(!window.m_translationActive);
        QCOMPARE(window.cursor().shape(), Qt::ClosedHandCursor);
        pointerEvent(window, QEvent::MouseButtonRelease, QPoint(62, 60), Qt::LeftButton);
    }

    /// @brief 关闭拖选或销毁窗口时取消准备任务并清理临时图片，无参数和返回值
    void cancelsBackgroundPreparation()
    {
        const QString command = markshot::shot::shellQuote(QString::fromUtf8(MARK_SHOT_TEST_OCR_FIXTURE_PATH))
            + QStringLiteral(" --ocr-gated-fixture");
        QVERIFY(markshot::writeAppConfigRoot({
            {QStringLiteral("ocr"), QJsonObject{{QStringLiteral("enabled"), true},
                                                {QStringLiteral("command"), command}}},
            {QStringLiteral("pinnedWindow"), QJsonObject{{QStringLiteral("autoOcr"), false},
                {QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("textSelectionCopyEnabled"), false}}}}));
        QImage image(400, 200, QImage::Format_RGB32);
        image.fill(Qt::white);
        auto window = std::make_unique<markshot::shot::PinnedImageWindow>(image);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->show();
        QVERIFY(QTest::qWaitForWindowExposed(window.get()));
        QVERIFY(!window->m_ocrTask);
        // 1. 【贴图测试】【开关调度】打开拖选才开始准备，关闭后立即释放任务资源
        window->setTextSelectionCopyEnabled(true);
        QTRY_VERIFY_WITH_TIMEOUT(window->m_ocrTask, 3000);
        const QString firstImage = window->m_ocrTempPath;
        QVERIFY(QFile::exists(firstImage));
        window->setTextSelectionCopyEnabled(false);
        QVERIFY(!window->m_ocrTask);
        QVERIFY(!QFile::exists(firstImage));
        // 2. 【贴图测试】【窗口销毁】重新启用后仍可识别，关闭窗口不会留下子任务或临时图片
        window->setTextSelectionCopyEnabled(true);
        QTRY_VERIFY_WITH_TIMEOUT(window->m_ocrTask, 3000);
        const QString secondImage = window->m_ocrTempPath;
        const QPointer<markshot::providers::ProviderTask> task = window->m_ocrTask;
        window.reset();
        QVERIFY(!QFile::exists(secondImage));
        QVERIFY(task.isNull());
    }

    /// @brief 识别结果不改变原图像素，只给实际选中文字绘制高亮，无参数和返回值
    void leavesUnselectedTextPixelsUnchanged()
    {
        QVERIFY(markshot::writeAppConfigRoot({{QStringLiteral("pinnedWindow"),
            QJsonObject{{QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("autoOcr"), false}}}}));
        QImage image(400, 200, QImage::Format_RGB32);
        image.fill(QColor(30, 40, 50));
        markshot::shot::PinnedImageWindow window(image, QPoint(80, 80));
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.m_ocrTokens = {{QStringLiteral("Hello"), QRectF(40, 40, 60, 40), 0, 0, 1.0}};
        const QImage beforeSelection = window.grab().toImage();
        QCOMPARE(beforeSelection.pixelColor(60, 60), image.pixelColor(60, 60));
        window.m_selectionAnchor = 0;
        window.m_selectionFocus = 0;
        const QImage afterSelection = window.grab().toImage();
        QVERIFY(afterSelection.pixelColor(60, 60) != image.pixelColor(60, 60));
        QCOMPARE(afterSelection.pixelColor(150, 60), image.pixelColor(150, 60));
    }

    /// @brief 编辑入口按当前显示方向打开图片编辑视图，贴图窗口保持可见，无参数和返回值
    void opensImageEditorWithCurrentImage()
    {
        QImage image(300, 120, QImage::Format_RGB32);
        image.fill(Qt::white);
        image.setPixelColor(0, 0, Qt::red);
        markshot::shot::PinnedImageWindow window(image, QPoint(80, 80));
        window.setAttribute(Qt::WA_DeleteOnClose, false);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));

        std::unique_ptr<ShotWindow> editor(window.openImageEditor());
        QVERIFY(editor);
        editor->setAttribute(Qt::WA_DeleteOnClose, false);
        QVERIFY(editor->isVisible());
        QVERIFY(window.isVisible());
        const QImage edited = editor->grab().toImage();
        QVERIFY(!edited.isNull());
        editor->close();
    }
};

/**
 * 【贴图测试】【隔离环境】运行窗口测试
 * @param argc 参数数量
 * @param argv 参数列表
 * @return 测试退出码
 */
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    if (!markshot::test::isolateConfig(isolated)) {
        return 1;
    }
    qputenv("XDG_SESSION_TYPE", "offscreen");
    qunsetenv("DISPLAY");
    qunsetenv("MARK_SHOT_OCR_COMMAND");
    qunsetenv("MARK_SHOT_PINNED_AUTO_OCR");
    qunsetenv("MARK_SHOT_OCR_DISABLED");
    qunsetenv("MARK_SHOT_TRANSLATION_AUTO_AFTER_OCR");
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("mark-shot-selection-test"));
    if (!markshot::appConfigPath().startsWith(isolated.path() + QLatin1Char('/'))) {
        return 1;
    }
    PinnedTextSelectionTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "pinned_text_selection_test.moc"
