#include "save_path_config.h"

#include <QDir>
#include <QJsonObject>
#include <QTimeZone>
#include <QtTest/QtTest>

namespace {

/**
 * 构造稳定的保存路径测试上下文。
 * @return 保存路径测试上下文。
 */
markshot::SavePathContext testContext()
{
    markshot::SavePathContext context;
    context.timestamp = QDateTime(QDate(2026, 6, 10), QTime(14, 5, 6, 7), QTimeZone(QByteArrayLiteral("UTC")));
    context.selectionRect = QRect(12, 34, 560, 780);
    context.sourceGeometry = QRect(1, 2, 1920, 1080);
    context.imageSize = QSize(1920, 1080);
    context.outputName = QStringLiteral("DP/1:Main");
    context.extension = QStringLiteral("png");
    return context;
}

}  // namespace

class SavePathConfigTest : public QObject {
    Q_OBJECT

private slots:
    void expandsFineGrainedPlaceholders()
    {
        const QString path = markshot::expandedSavePathTemplate(
            QStringLiteral("{yyyy}/{MM}/{dd}/shot-{HH}{mm}{ss}-{zzz}-{selection.geometry}-{source.width}x{source.height}-{image.width}-{name}.{ext}"),
            testContext());

        QCOMPARE(path,
                 QStringLiteral("2026/06/10/shot-140506-007-12,34 560x780-1920x1080-1920-DP-1-Main.png"));
    }

    void expandsCustomDateTimeFormat()
    {
        const QString path = markshot::expandedSavePathTemplate(
            QStringLiteral("shot-{datetime:yyyy-MM-dd_HH-mm-ss-zzz}.png"),
            testContext());

        QCOMPARE(path, QStringLiteral("shot-2026-06-10_14-05-06-007.png"));
    }

    void appendsPngSuffix()
    {
        QJsonObject save;
        save.insert(QStringLiteral("pathTemplate"), QDir::temp().filePath(QStringLiteral("shot-{date}")));
        QJsonObject root;
        root.insert(QStringLiteral("save"), save);

        QCOMPARE(markshot::savePathFromConfigRoot(root, testContext()),
                 QDir::cleanPath(QDir::temp().filePath(QStringLiteral("shot-20260610.png"))));
    }

    void directoryTemplateAddsDefaultFileName()
    {
        QJsonObject save;
        save.insert(QStringLiteral("directory"), QDir::temp().filePath(QStringLiteral("{yyyy}/{MM}")));
        QJsonObject root;
        root.insert(QStringLiteral("save"), save);

        QCOMPARE(markshot::savePathFromConfigRoot(root, testContext()),
                 QDir::cleanPath(QDir::temp().filePath(QStringLiteral("2026/06/mark-shot-20260610-140506.png"))));
    }

    void unknownPlaceholderFallsBackToDefault()
    {
        QJsonObject save;
        save.insert(QStringLiteral("pathTemplate"), QStringLiteral("{pictures}/broken-{missing}.png"));
        QJsonObject root;
        root.insert(QStringLiteral("save"), save);

        QCOMPARE(markshot::savePathFromConfigRoot(root, testContext()),
                 markshot::defaultSavePath(testContext()));
    }
};

QTEST_MAIN(SavePathConfigTest)
#include "save_path_config_test.moc"
