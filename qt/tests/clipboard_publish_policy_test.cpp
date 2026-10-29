#include "clipboard_publish_policy.h"

#include <QtTest/QtTest>

class ClipboardPublishPolicyTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证持久剪贴板进程可用时只走该路径。
     * @return 无返回值。
     */
    void prefersPersistentOwnerWhenAvailable()
    {
        QCOMPARE(markshot::chooseClipboardPublishPath(true, true),
                 markshot::ClipboardPublishPath::PersistentOwner);
        QCOMPARE(markshot::chooseClipboardPublishPath(true, false),
                 markshot::ClipboardPublishPath::PersistentOwner);
    }

    /**
     * 验证没有持久进程时才使用 Qt 剪贴板。
     * @return 无返回值。
     */
    void fallsBackToQtClipboard()
    {
        QCOMPARE(markshot::chooseClipboardPublishPath(false, true),
                 markshot::ClipboardPublishPath::QtClipboard);
    }

    /**
     * 验证两条路径都不可用时返回空。
     * @return 无返回值。
     */
    void returnsNoneWhenNoOwnerExists()
    {
        QCOMPARE(markshot::chooseClipboardPublishPath(false, false),
                 markshot::ClipboardPublishPath::None);
    }

    /**
     * 验证托盘常驻进程的图片改由 Qt 剪贴板持有，单次进程保持原有路径。
     * @return 无返回值。
     */
    void longLivedProcessKeepsImageOwnership()
    {
        QCOMPARE(markshot::chooseImageClipboardPublishPath(true, true, true),
                 markshot::ClipboardPublishPath::QtClipboard);
        QCOMPARE(markshot::chooseImageClipboardPublishPath(true, true, false),
                 markshot::ClipboardPublishPath::PersistentOwner);
        QCOMPARE(markshot::chooseImageClipboardPublishPath(true, false, true),
                 markshot::ClipboardPublishPath::PersistentOwner);
        QCOMPARE(markshot::chooseImageClipboardPublishPath(false, false, true),
                 markshot::ClipboardPublishPath::None);
    }
};

QTEST_APPLESS_MAIN(ClipboardPublishPolicyTest)

#include "clipboard_publish_policy_test.moc"
