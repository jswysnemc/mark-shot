#include "capture_double_click_action.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class CaptureDoubleClickActionTest : public QObject {
    Q_OBJECT

private slots:
    void parseActionAliases()
    {
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("copy")).value(),
                 markshot::CaptureDoubleClickAction::Copy);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("copy_and_close")).value(),
                 markshot::CaptureDoubleClickAction::Copy);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("quick save")).value(),
                 markshot::CaptureDoubleClickAction::Save);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("save-as")).value(),
                 markshot::CaptureDoubleClickAction::SaveAs);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("pin to screen")).value(),
                 markshot::CaptureDoubleClickAction::Pin);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("discard")).value(),
                 markshot::CaptureDoubleClickAction::Cancel);
        QCOMPARE(markshot::captureDoubleClickActionFromText(QStringLiteral("off")).value(),
                 markshot::CaptureDoubleClickAction::None);
    }

    void invalidTextIsEmpty()
    {
        QVERIFY(!markshot::captureDoubleClickActionFromText(QStringLiteral("invalid")).has_value());
        QVERIFY(!markshot::captureDoubleClickActionFromText(QString()).has_value());
    }

    void configRootReadsNestedCaptureValue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("doubleClickAction"), QStringLiteral("pin"));
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::captureDoubleClickActionFromConfigRoot(root),
                 markshot::CaptureDoubleClickAction::Pin);
    }

    void configRootAcceptsBooleanDisable()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("doubleClickAction"), false);
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::captureDoubleClickActionFromConfigRoot(root),
                 markshot::CaptureDoubleClickAction::None);
    }

    void configRootDefaultsToCopy()
    {
        QCOMPARE(markshot::defaultCaptureDoubleClickAction(),
                 markshot::CaptureDoubleClickAction::Copy);
        QCOMPARE(markshot::captureDoubleClickActionFromConfigRoot(QJsonObject()),
                 markshot::CaptureDoubleClickAction::Copy);
    }

    void configNamesRoundTrip()
    {
        const QVector<markshot::CaptureDoubleClickAction> actions {
            markshot::CaptureDoubleClickAction::None,
            markshot::CaptureDoubleClickAction::Copy,
            markshot::CaptureDoubleClickAction::Save,
            markshot::CaptureDoubleClickAction::SaveAs,
            markshot::CaptureDoubleClickAction::Pin,
            markshot::CaptureDoubleClickAction::Cancel,
        };
        for (const markshot::CaptureDoubleClickAction action : actions) {
            const QString name = markshot::captureDoubleClickActionName(action);
            QCOMPARE(markshot::captureDoubleClickActionFromText(name).value(), action);
        }
    }
};

QTEST_MAIN(CaptureDoubleClickActionTest)
#include "capture_double_click_action_test.moc"
