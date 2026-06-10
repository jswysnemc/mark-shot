#include "kde_capture_config.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class KdeCaptureConfigTest : public QObject {
    Q_OBJECT

private slots:
    void configRootDefaultsToEnabled()
    {
        QCOMPARE(markshot::defaultKdeKWinScreenshotEnabled(), true);
        QCOMPARE(markshot::kdeKWinScreenshotEnabledFromConfigRoot(QJsonObject()), true);
    }

    void configRootReadsDetailedDisabledValue()
    {
        QJsonObject kwinScreenshot;
        kwinScreenshot.insert(QStringLiteral("enabled"), false);

        QJsonObject kde;
        kde.insert(QStringLiteral("kwinScreenshot"), kwinScreenshot);

        QJsonObject wayland;
        wayland.insert(QStringLiteral("kde"), kde);

        QJsonObject capture;
        capture.insert(QStringLiteral("wayland"), wayland);

        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::kdeKWinScreenshotEnabledFromConfigRoot(root), false);
    }

    void configRootReadsDetailedEnabledValue()
    {
        QJsonObject kwinScreenshot;
        kwinScreenshot.insert(QStringLiteral("enabled"), true);

        QJsonObject kde;
        kde.insert(QStringLiteral("kwinScreenshot"), kwinScreenshot);

        QJsonObject wayland;
        wayland.insert(QStringLiteral("kde"), kde);

        QJsonObject capture;
        capture.insert(QStringLiteral("wayland"), wayland);

        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::kdeKWinScreenshotEnabledFromConfigRoot(root), true);
    }

    void invalidValueDefaultsToEnabled()
    {
        QJsonObject kwinScreenshot;
        kwinScreenshot.insert(QStringLiteral("enabled"), QJsonObject());

        QJsonObject kde;
        kde.insert(QStringLiteral("kwinScreenshot"), kwinScreenshot);

        QJsonObject wayland;
        wayland.insert(QStringLiteral("kde"), kde);

        QJsonObject capture;
        capture.insert(QStringLiteral("wayland"), wayland);

        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::kdeKWinScreenshotEnabledFromConfigRoot(root), true);
    }
};

QTEST_MAIN(KdeCaptureConfigTest)
#include "kde_capture_config_test.moc"
