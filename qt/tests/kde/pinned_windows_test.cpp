#include "kwin_script_probe.h"
#include "pinned_resize_checks.h"
#include "pinned_window_top.h"

#include <QApplication>
#include <QElapsedTimer>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QScreen>
#include <QSocketNotifier>
#include <QTextEdit>
#include <QVBoxLayout>
#include <QtTest>

#include <cstdio>
#include <memory>
#include <stdexcept>

namespace {

int checksPassed = 0;

/// @brief 检查真实窗口行为并记录失败原因
/// @param condition 检查结果
/// @param label 行为名称
/// @return 无返回值，失败时抛出异常
void require(bool condition, const char *label)
{
    if (!condition) {
        throw std::runtime_error(label);
    }
    ++checksPassed;
    std::fprintf(stdout, "PASS %s\n", label);
    std::fflush(stdout);
}

/// @brief 按独立宽度识别测试窗口，容忍合成器分数缩放误差
/// @param windows KWin 窗口快照
/// @param width 测试分配的窗口宽度
/// @return 对应窗口，未找到时返回空对象
QJsonObject windowWithWidth(const QJsonArray &windows, int width)
{
    for (const auto &value : windows) {
        const auto window = value.toObject();
        if (qAbs(window.value(QStringLiteral("width")).toDouble() - width) < 0.75) {
            return window;
        }
    }
    return {};
}

/// @brief 等待 KWin 确认窗口的实际置顶状态
/// @param probe 状态探针
/// @param width 待观察窗口的宽度
/// @param expected 预期置顶状态
/// @param label 检查名称
/// @return 已确认的窗口状态
QJsonObject expectAbove(KWinScriptProbe &probe, int width, bool expected, const char *label)
{
    QElapsedTimer timer;
    timer.start();
    QJsonObject window;
    do {
        QTest::qWait(20);
        window = windowWithWidth(probe.snapshot(), width);
        if (!window.isEmpty() && window.value(QStringLiteral("keepAbove")).toBool() == expected) {
            require(true, label);
            return window;
        }
    } while (timer.elapsed() < 3000);
    std::fprintf(stderr, "Observed: %s\n", QJsonDocument(window).toJson(QJsonDocument::Compact).constData());
    require(false, label);
    return {};
}

/// @brief 等待 KWin 确认窗口的实际高度，XWayland 窗口的尺寸变更需要额外一次往返
/// @param probe 状态探针
/// @param width 待观察窗口的宽度
/// @param height 预期窗口高度
/// @param label 检查名称
/// @return 无返回值，超时后抛出异常
void expectHeight(KWinScriptProbe &probe, int width, int height, const char *label)
{
    QElapsedTimer timer;
    timer.start();
    QJsonObject window;
    do {
        QTest::qWait(20);
        window = windowWithWidth(probe.snapshot(), width);
        if (qAbs(window.value(QStringLiteral("height")).toDouble() - height) < 0.75) {
            require(true, label);
            return;
        }
    } while (timer.elapsed() < 3000);
    std::fprintf(stderr, "Observed: %s\n", QJsonDocument(window).toJson(QJsonDocument::Compact).constData());
    require(false, label);
}

/// @brief 创建使用正式置顶入口的无边框窗口
/// @param title 原始应用标题
/// @param width 独立测试宽度
/// @param above 初始置顶状态
/// @return 已显示的窗口
std::unique_ptr<QWidget> makeWindow(const QString &title, int width, bool above)
{
    auto window = std::make_unique<QWidget>(nullptr, Qt::Window | Qt::FramelessWindowHint);
    window->setWindowTitle(title);
    window->resize(width, 300);
    markshot::shot::applyPinnedWindowTopState(window.get(), above);
    window->show();
    return window;
}

/// @brief 在独立进程创建同名窗口，收到标准输入后正常退出
/// @param app 测试应用
/// @return 应用退出码
int runForeignWindow(QApplication &app)
{
    QWidget window(nullptr, Qt::Window | Qt::FramelessWindowHint);
    window.setWindowTitle(app.arguments().last());
    window.resize(530, 280);
    if (app.arguments().contains(QStringLiteral("--managed-child"))) {
        markshot::shot::applyPinnedWindowTopState(&window, true);
    }
    window.show();
    QSocketNotifier input(0, QSocketNotifier::Read);
    QObject::connect(&input, &QSocketNotifier::activated, &app, &QCoreApplication::quit);
    return app.exec();
}

/// @brief 验证独立状态、堆叠顺序、映射与编辑状态
/// @param probe KWin 状态探针
/// @return 无返回值
void checkWindows(KWinScriptProbe &probe)
{
    const bool nativeWayland = QGuiApplication::platformName().contains(QStringLiteral("wayland"));
    const bool chinese = qEnvironmentVariable("MARK_SHOT_KDE_TEST_LANGUAGE") == QStringLiteral("zh");
    const QString ocrTitle = chinese ? QStringLiteral("OCR 结果") : QStringLiteral("OCR Result");
    const QString pinTitle = chinese ? QStringLiteral("钉住的截图") : QStringLiteral("Pinned Mark Shot");

    // 1. 【KDE测试】【多窗口隔离】同类窗口和钉图使用独立状态
    auto first = makeWindow(ocrTitle, 410, true);
    auto second = makeWindow(ocrTitle, 470, true);
    auto pinned = makeWindow(pinTitle, 350, true);
    auto initial = expectAbove(probe, 410, true, "first OCR stays above");
    expectAbove(probe, 470, true, "second OCR stays above");
    expectAbove(probe, 350, true, "pinned image stays above");
    require(!markshot::shot::pinnedWindowHasLayerShellTop(first.get()), "KDE keeps an ordinary movable window");

    auto *layout = new QVBoxLayout(first.get());
    auto *editor = new QTextEdit(first.get());
    layout->addWidget(editor);
    editor->setPlainText(QStringLiteral("original"));
    editor->moveCursor(QTextCursor::End);
    editor->insertPlainText(QStringLiteral(" edited"));
    first->resize(410, 300);

    // 2. 【KDE测试】【窗口层级】普通窗口取得焦点后，置顶窗口仍排在它上方
    QWidget cover(nullptr, Qt::Window | Qt::FramelessWindowHint);
    cover.setWindowTitle(QStringLiteral("KDE window cover"));
    cover.resize(800, 620);
    cover.show();
    QTest::qWait(100);
    probe.snapshot(QStringLiteral(
        "var list = workspace.windowList(); for (var i = 0; i < list.length; ++i) {"
        "if (list[i].caption === 'KDE window cover') { workspace.activeWindow = list[i]; } }"));
    QTest::qWait(80);
    const QJsonArray stacked = probe.snapshot();
    int coverIndex = -1;
    int topIndex = -1;
    for (int i = 0; i < stacked.size(); ++i) {
        const auto window = stacked[i].toObject();
        if (window.value(QStringLiteral("caption")).toString() == cover.windowTitle()) {
            coverIndex = i;
            require(window.value(QStringLiteral("active")).toBool(), "normal cover receives focus");
        }
        if (window.value(QStringLiteral("caption")).toString() == first->windowTitle()) {
            topIndex = i;
        }
    }
    require(coverIndex >= 0 && topIndex > coverIndex, "pinned OCR remains above the active cover");

    markshot::shot::applyPinnedWindowTopState(first.get(), false);
    const auto unpinned = expectAbove(probe, 410, false, "first OCR can disable top state");
    expectAbove(probe, 470, true, "second OCR keeps independent top state");
    expectAbove(probe, 350, true, "pinned image keeps independent top state");
    if (nativeWayland) {
        require(initial.value(QStringLiteral("id")) == unpinned.value(QStringLiteral("id")),
                "KDE toggle preserves the native surface");
        require(initial.value(QStringLiteral("x")) == unpinned.value(QStringLiteral("x"))
                    && initial.value(QStringLiteral("y")) == unpinned.value(QStringLiteral("y")),
                "KDE toggle preserves window position");
    }
    require(editor->toPlainText() == QStringLiteral("original edited"), "toggle retains edited text");
    editor->undo();
    require(editor->toPlainText() == QStringLiteral("original"), "toggle retains undo history");

    first->hide();
    first->show();
    second->hide();
    second->show();
    pinned->hide();
    pinned->show();
    expectAbove(probe, 410, false, "normal OCR remains normal after remap");
    expectAbove(probe, 470, true, "second OCR restores top state after remap");
    expectAbove(probe, 350, true, "pinned image restores top state after remap");

    // 3. 【KDE测试】【连续切换】最终状态保持一致，不重载或覆盖其他窗口脚本
    for (int i = 0; i < 20; ++i) {
        markshot::shot::applyPinnedWindowTopState(first.get(), i % 2 == 0);
    }
    expectAbove(probe, 410, false, "rapid toggles preserve the final requested state");
    expectAbove(probe, 470, true, "rapid toggles preserve sibling top state");
    first->resize(410, 360);
    expectHeight(probe, 410, 360, "normal KDE window remains resizable");

    // 4. 【KDE测试】【进程隔离】相同标题来自其他进程时不修改其层级
    QProcess foreign;
    foreign.start(QCoreApplication::applicationFilePath(), {QStringLiteral("--foreign-window"), first->windowTitle()});
    require(foreign.waitForStarted(2000), "foreign process starts");
    expectAbove(probe, 530, false, "foreign window starts normal");
    markshot::shot::applyPinnedWindowTopState(first.get(), true);
    expectAbove(probe, 410, true, "target window can enable top state again");
    expectAbove(probe, 530, false, "matching foreign title remains normal");
    foreign.write("quit\n");
    require(foreign.waitForFinished(3000), "foreign process exits normally");

    QTest::qWait(50);
    const int scriptsBeforeChild = probe.scriptCount();
    QProcess managedChild;
    managedChild.start(QCoreApplication::applicationFilePath(), {QStringLiteral("--managed-child"), ocrTitle});
    require(managedChild.waitForStarted(2000), "managed child process starts");
    expectAbove(probe, 530, true, "managed child owns its top state");
    managedChild.write("quit\n");
    require(managedChild.waitForFinished(3000), "managed child exits normally");
    QTest::qWait(100);
    require(probe.scriptCount() == scriptsBeforeChild, "application exit unloads its KWin script");

    // 5. 【KDE测试】【关闭清理】删除窗口后，同名普通窗口不会继承过期状态
    const QString oldTitle = second->windowTitle();
    second.reset();
    QTest::qWait(100);
    QWidget replacement(nullptr, Qt::Window | Qt::FramelessWindowHint);
    replacement.setWindowTitle(oldTitle);
    replacement.resize(580, 240);
    replacement.show();
    expectAbove(probe, 580, false, "destroyed window leaves no stale top request");

    if (nativeWayland && qEnvironmentVariableIsSet("MARK_SHOT_KDE_IDLE_CHECK")) {
        std::fprintf(stdout, "Checking subscription after the D-Bus timeout interval\n");
        std::fflush(stdout);
        QTest::qWait(27000);
        markshot::shot::applyPinnedWindowTopState(first.get(), false);
        expectAbove(probe, 410, false, "idle subscription remains usable after 27 seconds");
        expectAbove(probe, 350, true, "idle subscription preserves other pinned windows");
    }
}

}  // namespace

/// @brief 在专用隔离会话中检查实际 KWin 置顶行为
/// @param argc 参数数量
/// @param argv 参数内容
/// @return 检查成功返回 0，缺少隔离环境返回 77
int main(int argc, char **argv)
{
    if (!qEnvironmentVariableIsSet("MARK_SHOT_KDE_TEST_RUNTIME")) {
        std::fprintf(stderr, "Use scripts/test-kde-pinned-windows.py to create an isolated session\n");
        return 77;
    }
    QGuiApplication::setDesktopFileName(QStringLiteral("mark-shot"));
    QApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("mark-shot"));
    app.setQuitOnLastWindowClosed(false);
    if (app.arguments().contains(QStringLiteral("--foreign-window"))
        || app.arguments().contains(QStringLiteral("--managed-child"))) {
        return runForeignWindow(app);
    }
    try {
        KWinScriptProbe probe;
        checkWindows(probe);
        if (QGuiApplication::platformName().contains(QStringLiteral("wayland"))) {
            checksPassed += checkPinnedNativeResizes(probe);
        }
        std::fprintf(stdout, "ALL %d KDE WINDOW CHECKS PASSED\n", checksPassed);
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "FAIL %s\n", error.what());
        return 1;
    }
}
