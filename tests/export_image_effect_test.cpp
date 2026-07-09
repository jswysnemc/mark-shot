#include "export_image_effect.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class ExportImageEffectTest : public QObject
{
    Q_OBJECT

private slots:
    void defaultConfigLeavesImageUnchanged()
    {
        QImage source(12, 8, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(20, 120, 220, 255));

        const markshot::ExportImageEffectConfig config =
            markshot::exportImageEffectConfigFromRoot(QJsonObject());
        const QImage output = markshot::applyExportImageEffect(source, config);

        QVERIFY(!config.enabled);
        QCOMPARE(output.size(), source.size());
        QCOMPARE(output.pixelColor(source.width() / 2, source.height() / 2),
                 QColor(20, 120, 220, 255));
    }

    void disabledConfigLeavesImageUnchanged()
    {
        QImage source(12, 8, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(20, 120, 220, 255));
        source.setPixelColor(3, 4, QColor(250, 10, 30, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = false;
        const QImage output = markshot::applyExportImageEffect(source, config);

        QCOMPARE(output.size(), source.size());
        QCOMPARE(output.pixelColor(3, 4), QColor(250, 10, 30, 255));
        QCOMPARE(output.pixelColor(0, 0), QColor(20, 120, 220, 255));
    }

    void enabledEffectAddsTransparentFrameAndKeepsContent()
    {
        QImage source(20, 12, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(255, 40, 30, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 8;
        config.cornerRadius = 4.0;
        config.shadowRadius = 6;
        config.shadowOffsetY = 3;
        config.shadowOpacity = 0.45;

        const QImage output = markshot::applyExportImageEffect(source, config);

        QCOMPARE(output.size(), QSize(36, 28));
        QCOMPARE(output.pixelColor(0, 0).alpha(), 0);
        QCOMPARE(output.pixelColor(output.width() - 1, 0).alpha(), 0);
        QCOMPARE(output.pixelColor(18, 14), QColor(255, 40, 30, 255));
    }

    void shadowAppearsOutsideContentWithoutCoveringContent()
    {
        QImage source(30, 16, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(80, 200, 100, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 10;
        config.cornerRadius = 3.0;
        config.shadowRadius = 8;
        config.shadowOffsetY = 5;
        config.shadowOpacity = 0.6;

        const QImage output = markshot::applyExportImageEffect(source, config);

        const QColor contentPixel = output.pixelColor(config.padding + source.width() / 2,
                                                      config.padding + source.height() / 2);
        const QColor shadowPixel = output.pixelColor(config.padding + source.width() / 2,
                                                     config.padding + source.height()
                                                         + config.shadowOffsetY);

        QCOMPARE(contentPixel, QColor(80, 200, 100, 255));
        QVERIFY(shadowPixel.alpha() > 0);
        QVERIFY(shadowPixel.alpha() < 255);
    }

    void shadowIsBottomDominant()
    {
        QImage source(40, 24, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(245, 245, 245, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 24;
        config.cornerRadius = 6.0;
        config.shadowRadius = 18;
        config.shadowOffsetY = 10;
        config.shadowOpacity = 0.55;

        const QImage output = markshot::applyExportImageEffect(source, config);
        const int centerX = config.padding + source.width() / 2;
        const int centerY = config.padding + source.height() / 2;
        const int leftOfContent = config.padding - 4;
        const int aboveContent = config.padding - 4;
        const int belowContent = config.padding + source.height() + config.shadowOffsetY;

        const int topAlpha = output.pixelColor(centerX, aboveContent).alpha();
        const int sideAlpha = output.pixelColor(leftOfContent, centerY).alpha();
        const int bottomAlpha = output.pixelColor(centerX, belowContent).alpha();

        QVERIFY(topAlpha > 0);
        QVERIFY(sideAlpha > 0);
        QVERIFY2(bottomAlpha > 20,
                 qPrintable(QStringLiteral("bottom alpha: %1, top alpha: %2, side alpha: %3")
                                .arg(bottomAlpha)
                                .arg(topAlpha)
                                .arg(sideAlpha)));
        QVERIFY2(bottomAlpha > topAlpha * 2,
                 qPrintable(QStringLiteral("bottom alpha: %1, top alpha: %2")
                                .arg(bottomAlpha)
                                .arg(topAlpha)));
        QVERIFY2(bottomAlpha > sideAlpha * 2,
                 qPrintable(QStringLiteral("bottom alpha: %1, side alpha: %2")
                                .arg(bottomAlpha)
                                .arg(sideAlpha)));
    }

    void bottomShadowDoesNotSpillPastContentWidth()
    {
        QImage source(40, 24, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(245, 245, 245, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 24;
        config.cornerRadius = 6.0;
        config.shadowRadius = 18;
        config.shadowOffsetY = 10;
        config.shadowOpacity = 0.55;

        const QImage output = markshot::applyExportImageEffect(source, config);
        const int bottomShadowY = config.padding + source.height() + config.shadowOffsetY;

        const int centerAlpha =
            output.pixelColor(config.padding + source.width() / 2, bottomShadowY).alpha();
        const int leftAlpha = output.pixelColor(config.padding - 4, bottomShadowY).alpha();
        const int rightAlpha =
            output.pixelColor(config.padding + source.width() + 4, bottomShadowY).alpha();

        QVERIFY2(centerAlpha > 20,
                 qPrintable(QStringLiteral("center alpha: %1, left alpha: %2, right alpha: %3")
                                .arg(centerAlpha)
                                .arg(leftAlpha)
                                .arg(rightAlpha)));
        QVERIFY(leftAlpha < centerAlpha);
        QVERIFY(rightAlpha < centerAlpha);
    }

    void bottomShadowHasSoftHorizontalEdges()
    {
        QImage source(80, 48, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(245, 245, 245, 255));

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 32;
        config.cornerRadius = 10.0;
        config.shadowRadius = 24;
        config.shadowOffsetY = 12;
        config.shadowOpacity = 0.55;

        const QImage output = markshot::applyExportImageEffect(source, config);
        const int bottomShadowY = config.padding + source.height() + config.shadowOffsetY;
        const int centerX = config.padding + source.width() / 2;
        const int leftEdgeX = config.padding - 8;
        const int rightEdgeX = config.padding + source.width() + 7;

        const int centerAlpha = output.pixelColor(centerX, bottomShadowY).alpha();
        QVERIFY(centerAlpha > 20);
        QVERIFY2(output.pixelColor(leftEdgeX, bottomShadowY).alpha() < centerAlpha / 2,
                 "bottom shadow should fade before the left content edge");
        QVERIFY2(output.pixelColor(rightEdgeX, bottomShadowY).alpha() < centerAlpha / 2,
                 "bottom shadow should fade before the right content edge");
    }

    void roundedContentDropsCapturedEdgeFringe()
    {
        QImage source(28, 18, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(245, 245, 245, 255));
        for (int y = 0; y < source.height(); ++y) {
            source.setPixelColor(0, y, QColor(230, 20, 200, 255));
        }
        for (int x = 0; x < source.width(); ++x) {
            source.setPixelColor(x, source.height() - 1, QColor(40, 160, 230, 255));
        }

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 10;
        config.cornerRadius = 6.0;
        config.shadowRadius = 8;
        config.shadowOffsetY = 4;
        config.shadowOpacity = 0.4;

        const QImage output = markshot::applyExportImageEffect(source, config);

        const QColor leftEdge =
            output.pixelColor(config.padding, config.padding + source.height() / 2);
        const QColor bottomEdge =
            output.pixelColor(config.padding + source.width() / 2,
                              config.padding + source.height() - 1);
        QVERIFY(leftEdge.alpha() < 80);
        QVERIFY(leftEdge.red() < 32);
        QVERIFY(leftEdge.blue() < 32);
        QVERIFY2(bottomEdge.alpha() < 160,
                 qPrintable(QStringLiteral("bottom edge alpha: %1").arg(bottomEdge.alpha())));
        QVERIFY(bottomEdge.red() < 32);
        QVERIFY(bottomEdge.blue() < 32);
        QCOMPARE(output.pixelColor(config.padding + source.width() / 2,
                                   config.padding + source.height() / 2),
                 QColor(245, 245, 245, 255));
    }

    void roundedContentDropsTwoPixelWindowBorder()
    {
        QImage source(34, 24, QImage::Format_ARGB32_Premultiplied);
        source.fill(QColor(245, 245, 245, 255));
        for (int y = 0; y < source.height(); ++y) {
            source.setPixelColor(0, y, QColor(80, 210, 255, 255));
            source.setPixelColor(1, y, QColor(80, 210, 255, 255));
            source.setPixelColor(source.width() - 2, y, QColor(80, 210, 255, 255));
            source.setPixelColor(source.width() - 1, y, QColor(80, 210, 255, 255));
        }
        for (int x = 0; x < source.width(); ++x) {
            source.setPixelColor(x, 0, QColor(80, 210, 255, 255));
            source.setPixelColor(x, 1, QColor(80, 210, 255, 255));
            source.setPixelColor(x, source.height() - 2, QColor(80, 210, 255, 255));
            source.setPixelColor(x, source.height() - 1, QColor(80, 210, 255, 255));
        }

        markshot::ExportImageEffectConfig config;
        config.enabled = true;
        config.padding = 12;
        config.cornerRadius = 8.0;
        config.shadowRadius = 10;
        config.shadowOffsetY = 5;
        config.shadowOpacity = 0.4;

        const QImage output = markshot::applyExportImageEffect(source, config);

        const QColor leftEdge =
            output.pixelColor(config.padding + 1, config.padding + source.height() / 2);
        const QColor topEdge =
            output.pixelColor(config.padding + source.width() / 2, config.padding + 1);
        QVERIFY(leftEdge.alpha() < 80);
        QVERIFY(leftEdge.red() < 32);
        QVERIFY(leftEdge.green() < 32);
        QVERIFY(leftEdge.blue() < 32);
        QVERIFY(topEdge.alpha() < 80);
        QVERIFY(topEdge.red() < 32);
        QVERIFY(topEdge.green() < 32);
        QVERIFY(topEdge.blue() < 32);
        QCOMPARE(output.pixelColor(config.padding + source.width() / 2,
                                   config.padding + source.height() / 2),
                 QColor(245, 245, 245, 255));
    }

    void configParsingClampsInvalidValues()
    {
        QJsonObject imageFrame;
        imageFrame.insert(QStringLiteral("enabled"), true);
        imageFrame.insert(QStringLiteral("padding"), -12);
        imageFrame.insert(QStringLiteral("cornerRadius"), -7);
        imageFrame.insert(QStringLiteral("shadowRadius"), 999);
        imageFrame.insert(QStringLiteral("shadowOffsetY"), -4);
        imageFrame.insert(QStringLiteral("shadowOpacity"), 2.5);

        QJsonObject exportObject;
        exportObject.insert(QStringLiteral("imageFrame"), imageFrame);
        QJsonObject root;
        root.insert(QStringLiteral("export"), exportObject);

        const markshot::ExportImageEffectConfig config =
            markshot::exportImageEffectConfigFromRoot(root);

        QVERIFY(config.enabled);
        QCOMPARE(config.padding, 0);
        QCOMPARE(config.cornerRadius, 0.0);
        QCOMPARE(config.shadowRadius, 128);
        QCOMPARE(config.shadowOffsetY, 0);
        QCOMPARE(config.shadowOpacity, 1.0);
    }

    void booleanImageFrameTogglesDefaultEffect()
    {
        QJsonObject exportObject;
        exportObject.insert(QStringLiteral("imageFrame"), true);
        QJsonObject root;
        root.insert(QStringLiteral("export"), exportObject);

        const markshot::ExportImageEffectConfig config =
            markshot::exportImageEffectConfigFromRoot(root);

        QVERIFY(config.enabled);
        QCOMPARE(config.padding, 112);
        QCOMPARE(config.cornerRadius, 18.0);
        QCOMPARE(config.shadowRadius, 72);
        QCOMPARE(config.shadowOffsetY, 28);
        QCOMPARE(config.shadowOpacity, 0.32);
    }
};

QTEST_MAIN(ExportImageEffectTest)

#include "export_image_effect_test.moc"
