#include "capture_history/history_preview_cache.h"

#include <QFile>
#include <QTemporaryDir>
#include <QtTest>

using namespace markshot::history;

class HistoryPreviewCacheTest final : public QObject {
    Q_OBJECT
private slots:
    /** 【历史测试】【缓存命中】预览重复读取共享像素，不改变原图的尺寸和 DPR，无参数和返回值。 */
    void sharesPreviewWithoutChangingOriginal()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QImage original(1200, 800, QImage::Format_ARGB32);
        original.fill(QColor(30, 80, 120, 180));
        for (int y = 0; y < original.height(); ++y) {
            for (int x = 0; x < original.width(); ++x) {
                if ((x / 7 + y / 5) % 2) original.setPixelColor(x, y, QColor(150, 30, 200, 90));
            }
        }
        original.setDevicePixelRatio(2.0);
        QVERIFY(store.append(original));
        const QString name = store.entries().first().fileName;
        HistoryPreviewCache cache;
        const QImage first = cache.image(store, name, QSize(300, 200));
        QCOMPARE(first.size(), QSize(300, 200));
        QCOMPARE(first.devicePixelRatio(), 1.0);
        // 1. 参考图走存储层的缩放读取：Qt 6.8 前 PNG 插件自带盒式缩放，与 QImage::scaled 的平滑插值像素不同
        QImage reference = store.image(name, QSize(300, 200));
        reference.setDevicePixelRatio(1.0);
        QCOMPARE(first.convertToFormat(QImage::Format_ARGB32), reference.convertToFormat(QImage::Format_ARGB32));
        QCOMPARE(cache.image(store, name, QSize(300, 200)).cacheKey(), first.cacheKey());
        QCOMPARE(store.image(name).size(), original.size());
        QCOMPARE(store.image(name).devicePixelRatio(), 2.0);
        const QImage different = cache.image(store, name, QSize(150, 100));
        QCOMPARE(different.size(), QSize(150, 100));
        QVERIFY(different.cacheKey() != first.cacheKey());
        QVERIFY(cache.costKiB() <= 8192);
        QVERIFY(cache.image(store, name, {}).isNull());
        QVERIFY(cache.image(store, QStringLiteral("../outside.png"), QSize(10, 10)).isNull());
    }

    /** 【历史测试】【缓存失效】同名文件变化、删除或替换为符号链接后不返回旧预览，无参数和返回值。 */
    void invalidatesChangedAndRemovedFiles()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QImage original(600, 400, QImage::Format_RGB32);
        original.fill(Qt::red);
        QVERIFY(store.append(original));
        const QString name = store.entries().first().fileName;
        const QString path = QDir(directory.path()).filePath(name);
        HistoryPreviewCache cache;
        QCOMPARE(cache.image(store, name, QSize(60, 40)).pixelColor(0, 0), QColor(Qt::red));
        original.fill(Qt::blue);
        QVERIFY(original.save(path));
        QFile file(path);
        QVERIFY(file.open(QIODevice::ReadWrite));
        QVERIFY(file.setFileTime(QDateTime::currentDateTimeUtc().addSecs(2), QFileDevice::FileModificationTime));
        file.close();
        QCOMPARE(cache.image(store, name, QSize(60, 40)).pixelColor(0, 0), QColor(Qt::blue));
        QVERIFY(store.remove(name));
        QVERIFY(cache.image(store, name, QSize(60, 40)).isNull());
#ifdef Q_OS_UNIX
        const QString other = directory.filePath(QStringLiteral("other.png"));
        QVERIFY(original.save(other));
        QVERIFY(QFile::link(other, path));
        QVERIFY(cache.image(store, name, QSize(60, 40)).isNull());
#endif
        cache.retain({});
        QCOMPARE(cache.costKiB(), 0);
    }

    /** 【历史测试】【容量上限】大预览不进入小缓存，保留外部引用也不破坏预算或像素，无参数和返回值。 */
    void enforcesBudget()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QImage original(600, 400, QImage::Format_RGB32);
        original.fill(Qt::green);
        QVERIFY(store.append(original));
        const QString name = store.entries().first().fileName;
        HistoryPreviewCache cache(16);
        const QImage large = cache.image(store, name, QSize(300, 200));
        QVERIFY(!large.isNull());
        QCOMPARE(cache.costKiB(), 0);
        QVERIFY(cache.image(store, name, QSize(300, 200)).cacheKey() != large.cacheKey());
        for (int width = 20; width < 100; width += 5) {
            QVERIFY(!cache.image(store, name, QSize(width, 80)).isNull());
            QVERIFY(cache.costKiB() <= 16);
        }
        QCOMPARE(large.pixelColor(0, 0), QColor(Qt::green));
        cache.retain({});
        QCOMPARE(cache.costKiB(), 0);
    }
};

QTEST_GUILESS_MAIN(HistoryPreviewCacheTest)
#include "history_preview_cache_test.moc"
