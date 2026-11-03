#include "shot_window.h"
#include "annotation_launch.h"
#include "pinned_window/pinned_image_window.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QColorSpace>
#include <QScopedPointer>
#include <QTemporaryDir>
#include <QtTest/QtTest>

class CanvasImageMemoryTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【画布测试】【缓存释放】失效视口必须先释放，再开始分配下一幅视口，无参数和返回值。 */
    void releasesStaleViewportBeforeRendering()
    {
        struct ReleaseProbe {
            QImage *cache = nullptr;
            QByteArray pixels = QByteArray(64 * 48 * 4, char(0));
            bool released = false;
            bool cacheWasEmpty = false;
        } probe;
        QImage input(64, 48, QImage::Format_ARGB32_Premultiplied);
        input.fill(Qt::white);
        ShotWindow window(std::move(input), {}, {}, {}, false);
        window.setImageNavigationEnabled(true);
        window.startFullscreenAnnotation();
        window.resize(400, 300);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        window.m_frozenImageRect = QRectF(-100, -75, 800, 600);
        probe.cache = &window.m_sharpViewportCache;
        window.m_sharpViewportCache = QImage(
            reinterpret_cast<uchar *>(probe.pixels.data()), 64, 48, QImage::Format_ARGB32_Premultiplied,
            [](void *data) {
                auto *state = static_cast<ReleaseProbe *>(data);
                state->released = true;
                state->cacheWasEmpty = state->cache->isNull();
            }, &probe);
        window.m_sharpViewportCacheSourceRect = {};
        window.repaint();
        QVERIFY(probe.released);
        QVERIFY(probe.cacheWasEmpty);
        QVERIFY(!window.m_sharpViewportCache.isNull());
        const qint64 key = window.m_sharpViewportCache.cacheKey();
        window.repaint();
        QCOMPARE(window.m_sharpViewportCache.cacheKey(), key);
    }

    /** 【画布测试】【按需识别】编辑不启动选区识别，返回选区后正常生成候选且不重复启动，无参数和返回值。 */
    void defersDetectionUntilSelectionIsNeeded()
    {
        QImage input(128, 96, QImage::Format_ARGB32_Premultiplied);
        input.fill(Qt::white);
        ShotWindow window(std::move(input), {}, {}, {}, false);
        QVERIFY(!window.m_regionDetectorWatcher);
        window.setImageNavigationEnabled(true);
        window.startFullscreenAnnotation();
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QTest::qWait(30);
        QVERIFY(!window.m_regionDetectorWatcher);
        QVERIFY(!window.m_regionDetector);
        window.toggleCaptureScope();
        QTRY_VERIFY(window.m_regionDetector);
        auto *watcher = window.m_regionDetectorWatcher;
        window.initializeRegionDetection();
        QCOMPARE(window.m_regionDetectorWatcher, watcher);
    }

    /** 【画布测试】【截图识别】首次显示截图仍自动生成候选，重复显示不重复提交，无参数和返回值。 */
    void detectsVisibleCaptureOnce()
    {
        QImage input(128, 96, QImage::Format_ARGB32_Premultiplied);
        input.fill(Qt::white);
        ShotWindow window(std::move(input), {}, {}, {}, false);
        window.show();
        QTRY_VERIFY(window.m_regionDetector);
        auto *watcher = window.m_regionDetectorWatcher;
        window.hide();
        window.show();
        QCOMPARE(window.m_regionDetectorWatcher, watcher);
    }

    /** 【画布测试】【文件入口】真实文件入口延迟切换标注模式期间也不能启动选区工作，无参数和返回值。 */
    void fileEntryDoesNotStartRegionWorker()
    {
        QImage input(128, 96, QImage::Format_ARGB32_Premultiplied);
        input.fill(Qt::white);
        QScopedPointer<ShotWindow> window(markshot::openImageForAnnotation(input, QStringLiteral("memory-test")));
        QVERIFY(window);
        QTRY_VERIFY(window->m_fullscreenAnnotation);
        QVERIFY(!window->m_regionDetectorWatcher);
        QVERIFY(!window->m_regionDetector);
    }

    /** 独占 RGB32 输入应复用原像素分配，避免构造画布时额外复制整帧，无参数和返回值。 */
    void reusesUniqueOpaqueImage()
    {
        QImage input(64, 48, QImage::Format_RGB32);
        input.fill(QColor(65, 100, 145));
        const quintptr address = reinterpret_cast<quintptr>(input.constBits());
        const QByteArray expected(reinterpret_cast<const char *>(input.constBits()), input.sizeInBytes());
        ShotWindow window(std::move(input), {}, {}, {}, false);
        QCOMPARE(window.m_frozenFrame.format(), QImage::Format_RGB32);
        QCOMPARE(reinterpret_cast<quintptr>(window.m_frozenFrame.constBits()), address);
        QCOMPARE(QByteArray(reinterpret_cast<const char *>(window.m_frozenFrame.constBits()),
                             window.m_frozenFrame.sizeInBytes()), expected);
    }

    /** 【画布测试】【共享原图】不透明原图可同时用于贴图和编辑，不能为无效的预乘操作复制整帧，无参数和返回值。 */
    void reusesSharedOpaqueImage()
    {
        QImage input(64, 48, QImage::Format_RGB32);
        input.fill(QColor(65, 100, 145));
        const quintptr address = reinterpret_cast<quintptr>(input.constBits());
        ShotWindow window(input, {}, {}, {}, false);
        QCOMPARE(reinterpret_cast<quintptr>(window.m_frozenFrame.constBits()), address);
        QCOMPARE(window.m_frozenFrame.pixelColor(0, 0), input.pixelColor(0, 0));
        QCOMPARE(input.format(), QImage::Format_RGB32);
    }

    /** 共享输入必须保持原格式与像素，透明颜色仍正确预乘，无参数和返回值。 */
    void protectsSharedTransparentInput()
    {
        QImage input(64, 48, QImage::Format_ARGB32);
        const QRgb pixel = qRgba(80, 20, 12, 100);
        input.fill(pixel);
        const quintptr address = reinterpret_cast<quintptr>(input.constBits());
        ShotWindow window(input, {}, {}, {}, false);
        QCOMPARE(input.format(), QImage::Format_ARGB32);
        QCOMPARE(reinterpret_cast<quintptr>(input.constBits()), address);
        QCOMPARE(input.pixel(0, 0), pixel);
        QCOMPARE(window.m_frozenFrame.pixel(0, 0), qPremultiply(pixel));
    }

    /** 清除画布 DPI 元数据时不改变其他窗口持有的原图，无参数和返回值。 */
    void isolatesSharedDevicePixelRatio_data()
    {
        QTest::addColumn<int>("format");
        QTest::newRow("opaque") << int(QImage::Format_RGB32);
        QTest::newRow("premultiplied") << int(QImage::Format_ARGB32_Premultiplied);
    }

    /** 【画布测试】【共享元数据】按两种可共享格式验证 DPI 隔离，无参数和返回值。 */
    void isolatesSharedDevicePixelRatio()
    {
        QFETCH(int, format);
        QImage input(64, 48, static_cast<QImage::Format>(format));
        input.fill(Qt::white);
        input.setDevicePixelRatio(2.0);
        ShotWindow window(input, {}, {}, {}, false);
        QCOMPARE(input.devicePixelRatio(), 2.0);
        QCOMPARE(window.m_frozenFrame.devicePixelRatio(), 1.0);
        QCOMPARE(window.m_frozenFrame.pixel(0, 0), input.pixel(0, 0));
        QCOMPARE(window.m_frozenFrame.constBits(), input.constBits());
    }

    /** 【画布测试】【贴图编辑】真实高分屏贴图入口共享原像素，关闭贴图后编辑器仍可使用，无参数和返回值。 */
    void pinnedHighDpiEditorKeepsSharedPixelsAlive()
    {
        QImage input(128, 96, QImage::Format_RGB32);
        input.fill(QColor(35, 65, 95));
        input.setDevicePixelRatio(2.0);
        const auto *pixels = input.constBits();
        QScopedPointer<markshot::shot::PinnedImageWindow> pinned(
            new markshot::shot::PinnedImageWindow(std::move(input)));
        QScopedPointer<ShotWindow> editor(pinned->openImageEditor());
        QVERIFY(editor);
        QTRY_VERIFY(editor->m_fullscreenAnnotation);
        QCOMPARE(editor->m_frozenFrame.constBits(), pixels);
        QCOMPARE(editor->m_frozenFrame.devicePixelRatio(), 1.0);
        pinned.reset();
        QCOMPARE(editor->m_frozenFrame.constBits(), pixels);
        QCOMPARE(editor->m_frozenFrame.pixelColor(0, 0), QColor(35, 65, 95));
    }

    /** 就地转换仍保留色彩空间、分辨率及文本元数据，无参数和返回值。 */
    void preservesImageMetadata()
    {
        QImage input(64, 48, QImage::Format_ARGB32);
        input.fill(QColor(30, 80, 120, 150));
        input.setColorSpace(QColorSpace(QColorSpace::DisplayP3));
        input.setDotsPerMeterX(4200);
        input.setDotsPerMeterY(4300);
        input.setText(QStringLiteral("Description"), QStringLiteral("fixture"));
        ShotWindow window(std::move(input), {}, {}, {}, false);
        QCOMPARE(window.m_frozenFrame.format(), QImage::Format_ARGB32_Premultiplied);
        QCOMPARE(window.m_frozenFrame.pixel(0, 0), qPremultiply(qRgba(30, 80, 120, 150)));
        QCOMPARE(window.m_frozenFrame.colorSpace(), QColorSpace(QColorSpace::DisplayP3));
        QCOMPARE(window.m_frozenFrame.dotsPerMeterX(), 4200);
        QCOMPARE(window.m_frozenFrame.dotsPerMeterY(), 4300);
        QCOMPARE(window.m_frozenFrame.text(QStringLiteral("Description")), QStringLiteral("fixture"));
    }
};

/** 隔离配置后执行画布内存测试，argc/argv 为测试参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    CanvasImageMemoryTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "canvas_image_memory_test.moc"
