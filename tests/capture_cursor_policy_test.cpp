#include "capture_cursor_policy.h"

#include <QJsonObject>
#include <QtTest/QtTest>

class CaptureCursorPolicyTest : public QObject {
    Q_OBJECT

private slots:
    void configRootReadsNestedCaptureValue()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("includeCursor"), true);
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::captureIncludeCursorFromConfigRoot(root), true);
    }

    void configRootReadsAliases()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("includeMouse"), QStringLiteral("on"));
        QJsonObject root;
        root.insert(QStringLiteral("screenCapture"), capture);

        QCOMPARE(markshot::captureIncludeCursorFromConfigRoot(root), true);
    }

    void configRootReadsRootFallback()
    {
        QJsonObject root;
        root.insert(QStringLiteral("captureIncludeCursor"), 1);

        QCOMPARE(markshot::captureIncludeCursorFromConfigRoot(root), true);
    }

    void configRootDefaultsToFalse()
    {
        QCOMPARE(markshot::defaultCaptureIncludeCursor(), false);
        QCOMPARE(markshot::captureIncludeCursorFromConfigRoot(QJsonObject()), false);
    }

    void invalidValueDefaultsToFalse()
    {
        QJsonObject capture;
        capture.insert(QStringLiteral("includePointer"), QJsonObject());
        QJsonObject root;
        root.insert(QStringLiteral("capture"), capture);

        QCOMPARE(markshot::captureIncludeCursorFromConfigRoot(root), false);
    }
};

QTEST_MAIN(CaptureCursorPolicyTest)
#include "capture_cursor_policy_test.moc"
