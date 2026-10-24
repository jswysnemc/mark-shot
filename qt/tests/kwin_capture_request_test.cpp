#include "display_capture/kwin_capture_request.h"

#include <QtTest>

class KWinCaptureRequestTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【KWin测试】【方法选择】提供混合缩放、区域及无效输出场景，无参数和返回值。 */
    void selectsCaptureMethod_data()
    {
        QTest::addColumn<QString>("outputName");
        QTest::addColumn<QRect>("geometry");
        QTest::addColumn<bool>("allOutputs");
        QTest::addColumn<QString>("method");
        QTest::newRow("scale-one-output") << QStringLiteral("HDMI-A-1")
            << QRect(1707, 0, 1920, 1080) << false << QStringLiteral("CaptureScreen");
        QTest::newRow("fractional-scale-output") << QStringLiteral("eDP-1")
            << QRect(0, 0, 1707, 1067) << false << QStringLiteral("CaptureScreen");
        QTest::newRow("region") << QStringLiteral("HDMI-A-1")
            << QRect(1800, 20, 320, 240) << false << QStringLiteral("CaptureArea");
        QTest::newRow("all-outputs") << QStringLiteral("HDMI-A-1")
            << QRect(1707, 0, 1920, 1080) << true << QStringLiteral("CaptureArea");
        QTest::newRow("empty-name") << QString()
            << QRect(1707, 0, 1920, 1080) << false << QStringLiteral("CaptureArea");
        QTest::newRow("stale-name-at-existing-output") << QStringLiteral("disconnected-output")
            << QRect(1707, 0, 1920, 1080) << false << QStringLiteral("CaptureArea");
        QTest::newRow("name-geometry-mismatch") << QStringLiteral("eDP-1")
            << QRect(1707, 0, 1920, 1080) << false << QStringLiteral("CaptureArea");
        QTest::newRow("empty-output-geometry") << QStringLiteral("empty-output")
            << QRect(1707, 0, 1920, 1080) << false << QStringLiteral("CaptureArea");
        QTest::newRow("negative-origin-output") << QStringLiteral("DP-1")
            << QRect(-1920, 0, 1920, 1080) << false << QStringLiteral("CaptureScreen");
        QTest::newRow("negative-origin-region") << QStringLiteral("DP-1")
            << QRect(-1900, 20, 320, 240) << false << QStringLiteral("CaptureArea");
        QTest::newRow("normalized-whole-output") << QStringLiteral("HDMI-A-1")
            << QRect(3627, 1080, -1920, -1080) << false << QStringLiteral("CaptureScreen");
    }

    /** 【KWin测试】【方法选择】验证实际请求方法和 D-Bus 坐标参数类型，无参数和返回值。 */
    void selectsCaptureMethod()
    {
        QFETCH(QString, outputName);
        QFETCH(QRect, geometry);
        QFETCH(bool, allOutputs);
        QFETCH(QString, method);
        const QMap<QString, QRect> outputs{
            {QStringLiteral("HDMI-A-1"), QRect(1707, 0, 1920, 1080)},
            {QStringLiteral("eDP-1"), QRect(0, 0, 1707, 1067)},
            {QStringLiteral("DP-1"), QRect(-1920, 0, 1920, 1080)},
            {QStringLiteral("empty-output"), {}},
        };
        CaptureRequest request;
        request.preferredOutputName = outputName;
        request.sourceGeometry = geometry;
        request.allOutputs = allOutputs;

        // 1. 【KWin测试】【方法选择】只允许有效名称对应的完整几何区域使用 CaptureScreen
        const auto captureRequest = markshot::capture::buildKWinCaptureRequest(request, outputs);
        QCOMPARE(captureRequest.method, method);
        if (method == QStringLiteral("CaptureScreen")) {
            QCOMPARE(captureRequest.arguments.size(), 2);
            QCOMPARE(captureRequest.arguments.first().toString(), outputName);
        } else {
            // 2. 【KWin测试】【区域参数】确保坐标为有符号整数，尺寸为 D-Bus 要求的无符号整数
            QCOMPARE(captureRequest.arguments.size(), 5);
            QCOMPARE(captureRequest.arguments.at(0).metaType().id(), QMetaType::Int);
            QCOMPARE(captureRequest.arguments.at(1).metaType().id(), QMetaType::Int);
            QCOMPARE(captureRequest.arguments.at(2).metaType().id(), QMetaType::UInt);
            QCOMPARE(captureRequest.arguments.at(3).metaType().id(), QMetaType::UInt);
            const QRect normalized = geometry.normalized();
            QCOMPARE(captureRequest.arguments.at(0).toInt(), normalized.x());
            QCOMPARE(captureRequest.arguments.at(1).toInt(), normalized.y());
            QCOMPARE(captureRequest.arguments.at(2).toUInt(), static_cast<uint>(normalized.width()));
            QCOMPARE(captureRequest.arguments.at(3).toUInt(), static_cast<uint>(normalized.height()));
        }
        const QVariantMap options = captureRequest.arguments.last().toMap();
        QCOMPARE(options.value(QStringLiteral("native-resolution")), QVariant(true));
        QCOMPARE(options.value(QStringLiteral("include-cursor")), QVariant(false));
        QVERIFY(!options.contains(QStringLiteral("hide-caller-windows")));
    }

    /** 【KWin测试】【捕获选项】为两种方法提供鼠标及自身窗口策略组合，无参数和返回值。 */
    void preservesCaptureOptions_data()
    {
        QTest::addColumn<bool>("wholeOutput");
        QTest::addColumn<bool>("includeCursor");
        QTest::addColumn<bool>("hideOwnWindows");
        for (bool wholeOutput : {false, true}) {
            for (bool includeCursor : {false, true}) {
                for (bool hideOwnWindows : {false, true}) {
                    const QByteArray name = QByteArray::number(wholeOutput) + '-'
                        + QByteArray::number(includeCursor) + '-' + QByteArray::number(hideOwnWindows);
                    QTest::newRow(name.constData()) << wholeOutput << includeCursor << hideOwnWindows;
                }
            }
        }
    }

    /** 【KWin测试】【捕获选项】验证方法切换不改变原生分辨率及鼠标、自身窗口策略，无参数和返回值。 */
    void preservesCaptureOptions()
    {
        QFETCH(bool, wholeOutput);
        QFETCH(bool, includeCursor);
        QFETCH(bool, hideOwnWindows);
        CaptureRequest request;
        request.preferredOutputName = QStringLiteral("HDMI-A-1");
        request.sourceGeometry = wholeOutput ? QRect(0, 0, 1920, 1080) : QRect(50, 60, 320, 240);
        request.includeCursor = includeCursor;
        request.hideOwnWindows = hideOwnWindows;
        const auto captureRequest = markshot::capture::buildKWinCaptureRequest(
            request, {{request.preferredOutputName, QRect(0, 0, 1920, 1080)}});
        const QVariantMap options = captureRequest.arguments.last().toMap();
        QCOMPARE(options.value(QStringLiteral("native-resolution")), QVariant(true));
        QCOMPARE(options.value(QStringLiteral("include-cursor")), QVariant(includeCursor));
        QCOMPARE(options.contains(QStringLiteral("hide-caller-windows")), !hideOwnWindows);
        if (!hideOwnWindows) {
            QCOMPARE(options.value(QStringLiteral("hide-caller-windows")), QVariant(false));
        }
    }
};

QTEST_APPLESS_MAIN(KWinCaptureRequestTest)
#include "kwin_capture_request_test.moc"
