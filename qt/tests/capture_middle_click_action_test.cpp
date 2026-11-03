#include "capture_middle_click_action.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class CaptureMiddleClickActionTest : public QObject {
    Q_OBJECT

private slots:
    /** 验证中键单击沿用双击动作的别名，无参数和返回值。 */
    void parseSharedActionAliases()
    {
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("pin")).value(),
                 markshot::CaptureClickAction::Pin);
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("pin to screen")).value(),
                 markshot::CaptureClickAction::Pin);
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("copy_and_close")).value(),
                 markshot::CaptureClickAction::Copy);
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("quick save")).value(),
                 markshot::CaptureClickAction::Save);
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("save-as")).value(),
                 markshot::CaptureClickAction::SaveAs);
        QCOMPARE(markshot::captureMiddleClickActionFromText(QStringLiteral("discard")).value(),
                 markshot::CaptureClickAction::Cancel);
    }

    /** 验证 default、zoom、pan、none 等名称都保留中键内置行为，无参数和返回值。 */
    void builtinBehaviourAliases()
    {
        for (const QString &text : {QStringLiteral("default"), QStringLiteral("Zoom"), QStringLiteral("pan"),
                                    QStringLiteral("built-in"), QStringLiteral("none"), QStringLiteral("off")}) {
            QCOMPARE(markshot::captureMiddleClickActionFromText(text).value(),
                     markshot::CaptureClickAction::None);
        }
    }

    /** 验证非法和空文本返回空值，无参数和返回值。 */
    void invalidTextIsEmpty()
    {
        QVERIFY(!markshot::captureMiddleClickActionFromText(QStringLiteral("invalid")).has_value());
        QVERIFY(!markshot::captureMiddleClickActionFromText(QString()).has_value());
    }

    /** 验证从 capture.middleClickAction 读取配置，无参数和返回值。 */
    void configRootReadsNestedCaptureValue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("middleClickAction"), QStringLiteral("pin"));
        capture.insert(QStringLiteral("doubleClickAction"), QStringLiteral("copy"));
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::captureMiddleClickActionFromConfigRoot(root),
                 markshot::CaptureClickAction::Pin);
    }

    /** 验证缺失、非法值和布尔值都回到内置行为，且不受双击配置影响，无参数和返回值。 */
    void configRootDefaultsToBuiltinBehaviour()
    {
        QCOMPARE(markshot::defaultCaptureMiddleClickAction(), markshot::CaptureClickAction::None);
        QCOMPARE(markshot::captureMiddleClickActionFromConfigRoot(QJsonObject()),
                 markshot::CaptureClickAction::None);

        QJsonObject capture;
        capture.insert(QStringLiteral("doubleClickAction"), QStringLiteral("pin"));
        capture.insert(QStringLiteral("middleClickAction"), QStringLiteral("invalid"));
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);
        QCOMPARE(markshot::captureMiddleClickActionFromConfigRoot(root),
                 markshot::CaptureClickAction::None);

        capture.insert(QStringLiteral("middleClickAction"), false);
        root.insert(QStringLiteral("capture"), capture);
        QCOMPARE(markshot::captureMiddleClickActionFromConfigRoot(root),
                 markshot::CaptureClickAction::None);
    }

    /** 验证规范名称可以原样读回，内置行为写作 default，无参数和返回值。 */
    void configNamesRoundTrip()
    {
        QCOMPARE(markshot::captureMiddleClickActionName(markshot::CaptureClickAction::None),
                 QStringLiteral("default"));
        const QVector<markshot::CaptureClickAction> actions {
            markshot::CaptureClickAction::None,
            markshot::CaptureClickAction::Copy,
            markshot::CaptureClickAction::Save,
            markshot::CaptureClickAction::SaveAs,
            markshot::CaptureClickAction::Pin,
            markshot::CaptureClickAction::Cancel,
        };
        for (const markshot::CaptureClickAction action : actions) {
            const QString name = markshot::captureMiddleClickActionName(action);
            QCOMPARE(markshot::captureMiddleClickActionFromText(name).value(), action);
        }
    }
};

QTEST_MAIN(CaptureMiddleClickActionTest)
#include "capture_middle_click_action_test.moc"
