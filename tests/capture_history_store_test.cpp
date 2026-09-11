#include "capture_history/history_store.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QLockFile>
#include <QTemporaryDir>
#include <QtTest/QtTest>

using markshot::history::HistoryStore;

namespace {

/// @brief 创建包含透明像素的确定性测试图像
/// @param size 图像像素尺寸
/// @return 可用于逐像素比对的图像
QImage sampleImage(QSize size = QSize(32, 20))
{
    QImage image(size, QImage::Format_ARGB32);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            image.setPixel(x, y, qRgba((x * 31 + y) % 256, (y * 17 + x) % 256,
                                      (x * 13 + y * 7) % 256, 128 + (x + y) % 128));
        }
    }
    return image;
}

/// @brief 创建测试用普通文件
/// @param path 文件绝对路径
/// @param contents 文件内容
/// @return 全部内容写入成功时返回 true
bool writeFile(const QString &path, const QByteArray &contents)
{
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(contents) == contents.size();
}

}

class CaptureHistoryStoreTest : public QObject {
    Q_OBJECT

private slots:
    /// @brief 验证不同存储实例读取相同像素与高分屏比例
    /// @return 无返回值
    void persistsPixelsAndDevicePixelRatio()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QImage original = sampleImage();
        original.setDevicePixelRatio(2.0);
        QString error = QStringLiteral("previous error");
        QVERIFY2(store.append(original, 50, &error), qPrintable(error));
        QVERIFY(error.isEmpty());

        HistoryStore reopened(directory.path());
        const auto entries = reopened.entries();
        QCOMPARE(entries.size(), 1);
        QCOMPARE(entries.first().imageSize, original.size());
        const QImage restored = reopened.image(entries.first().fileName);
        QCOMPARE(restored.devicePixelRatio(), 2.0);
        QCOMPARE(restored.convertToFormat(QImage::Format_RGBA8888),
                 original.convertToFormat(QImage::Format_RGBA8888));
        QCOMPARE(reopened.image(entries.first().fileName, QSize(16, 16)).size(), QSize(16, 10));
    }

    /// @brief 验证连续保存按时间倒序排列并淘汰最早的截图
    /// @return 无返回值
    void retainsNewestEntriesWithoutTimestampTies()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QVector<QString> identifiers;
        QVector<QDateTime> times;
        for (int index = 0; index < 8; ++index) {
            QImage image(8, 8, QImage::Format_RGB32);
            image.fill(QColor(index, index, index));
            QVERIFY(store.append(image, 3));
            const auto entries = store.entries();
            identifiers.append(entries.first().fileName);
            times.append(entries.first().createdAt);
            if (index > 0) {
                QVERIFY(times.at(index) > times.at(index - 1));
            }
        }
        const auto entries = store.entries();
        QCOMPARE(entries.size(), 3);
        for (int index = 0; index < 3; ++index) {
            QCOMPARE(entries.at(index).fileName, identifiers.at(7 - index));
            QCOMPARE(store.image(entries.at(index).fileName).pixelColor(0, 0), QColor(7 - index, 7 - index, 7 - index));
        }
        QVERIFY(!QFileInfo::exists(QDir(directory.path()).filePath(identifiers.first())));
    }

    /// @brief 验证总大小限制保留最新截图，并在拒绝超大图片时保留旧记录
    /// @return 无返回值
    void enforcesByteBudgetWithoutLosingExistingImages()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QImage small = sampleImage(QSize(8, 8));
        HistoryStore store(directory.path());
        QVERIFY(store.append(small));
        const qint64 imageBytes = QFileInfo(QDir(directory.path()).filePath(store.entries().first().fileName)).size();
        QVERIFY(imageBytes > 0);

        HistoryStore limited(directory.path(), imageBytes * 2);
        QVERIFY(limited.append(small));
        QVERIFY(limited.append(small));
        const auto before = limited.entries();
        QCOMPARE(before.size(), 2);
        QString error;
        QVERIFY(!limited.append(sampleImage(QSize(256, 256)), 50, &error));
        QVERIFY(!error.isEmpty());
        const auto after = limited.entries();
        QCOMPARE(after.size(), before.size());
        for (int index = 0; index < after.size(); ++index) {
            QCOMPARE(after.at(index).fileName, before.at(index).fileName);
            QVERIFY(!limited.image(after.at(index).fileName).isNull());
        }
        qint64 total = 0;
        for (const auto &file : QDir(directory.path()).entryInfoList(QDir::Files)) {
            total += file.size();
        }
        QVERIFY(total <= imageBytes * 2);
    }

    /// @brief 验证路径穿越和非历史文件无法通过历史接口读取或删除
    /// @return 无返回值
    void rejectsForeignPaths()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString name = QStringLiteral("capture-20260911010203000-0123456789abcdef0123456789abcdef.png");
        const QString outside = QDir(directory.path()).filePath(name);
        QVERIFY(sampleImage().save(outside, "PNG"));
        const QString storagePath = QDir(directory.path()).filePath(QStringLiteral("history"));
        QVERIFY(QDir().mkpath(storagePath));
        HistoryStore store(storagePath);
        const QStringList paths{QStringLiteral("../") + name, outside, QStringLiteral("other.png"), QString()};
        for (const QString &path : paths) {
            QVERIFY(store.image(path).isNull());
            QString error;
            QVERIFY(!store.remove(path, &error));
            QVERIFY(!error.isEmpty());
        }
        QVERIFY(QFileInfo::exists(outside));
    }

    /// @brief 验证清空包含损坏记录，但不会删除其他文件
    /// @return 无返回值
    void clearsOnlyOwnedFiles()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QVERIFY(store.append(sampleImage()));
        const QString corrupt = QDir(directory.path()).filePath(
            QStringLiteral("capture-20260911010203000-0123456789abcdef0123456789abcdef.png"));
        const QString unrelated = QDir(directory.path()).filePath(QStringLiteral("personal.png"));
        QVERIFY(writeFile(corrupt, QByteArrayLiteral("invalid png")));
        QVERIFY(writeFile(unrelated, QByteArrayLiteral("keep this file")));
        QCOMPARE(store.entries().size(), 1);
        QVERIFY(store.clear());
        QVERIFY(store.entries().isEmpty());
        QVERIFY(!QFileInfo::exists(corrupt));
        QVERIFY(QFileInfo::exists(unrelated));
    }

    /// @brief 验证历史接口不会读取或删除符号链接指向的文件
    /// @return 无返回值
    void ignoresSymbolicLinks()
    {
#ifdef Q_OS_WIN
        QSKIP("Windows QFile::link creates shortcuts instead of POSIX symbolic links");
#else
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        const QString source = QDir(directory.path()).filePath(QStringLiteral("source.png"));
        QVERIFY(sampleImage().save(source, "PNG"));
        const QString name = QStringLiteral("capture-20260911010203000-0123456789abcdef0123456789abcdef.png");
        const QString link = QDir(directory.path()).filePath(name);
        QVERIFY(QFile::link(source, link));
        HistoryStore store(directory.path());
        QVERIFY(store.entries().isEmpty());
        QVERIFY(store.image(name).isNull());
        QVERIFY(!store.remove(name));
        QVERIFY(store.clear());
        QVERIFY(QFileInfo::exists(source));
        QVERIFY(QFileInfo(link).isSymLink());
#endif
    }

    /// @brief 验证写入锁冲突会明确失败且不会创建截图文件
    /// @return 无返回值
    void reportsLockContention()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        QLockFile lock(QDir(directory.path()).filePath(QStringLiteral(".history.lock")));
        QVERIFY(lock.tryLock());
        HistoryStore store(directory.path());
        QString error;
        QVERIFY(!store.append(sampleImage(), 50, &error));
        QVERIFY(!error.isEmpty());
        QVERIFY(store.entries().isEmpty());
    }

    /// @brief 验证空图像或非法保留数量不会触发写入
    /// @return 无返回值
    void rejectsInvalidCaptures()
    {
        QTemporaryDir directory;
        QVERIFY(directory.isValid());
        HistoryStore store(directory.path());
        QVERIFY(!store.append({}));
        QVERIFY(!store.append(sampleImage(), 0));
        QVERIFY(store.entries().isEmpty());
    }

    /// @brief 验证历史默认开关和可配置数量边界
    /// @return 无返回值
    void readsRecordingPolicy()
    {
        using markshot::history::historyConfigFromRoot;
        const auto defaults = historyConfigFromRoot({});
        QVERIFY(defaults.enabled);
        QCOMPARE(defaults.maximumEntries, 50);
        const auto disabled = historyConfigFromRoot({{"captureHistory", QJsonObject{{"enabled", false}, {"limit", 12}}}});
        QVERIFY(!disabled.enabled);
        QCOMPARE(disabled.maximumEntries, 12);
        QCOMPARE(historyConfigFromRoot({{"captureHistory", QJsonObject{{"limit", -1}}}}).maximumEntries, 1);
        QCOMPARE(historyConfigFromRoot({{"captureHistory", QJsonObject{{"limit", 999}}}}).maximumEntries, 200);
    }
};

QTEST_APPLESS_MAIN(CaptureHistoryStoreTest)

#include "capture_history_store_test.moc"
