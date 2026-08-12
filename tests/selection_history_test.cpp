#include "selection_history.h"

#include <QJsonArray>
#include <QJsonObject>
#include <QtTest/QtTest>

/// @brief 截图选区历史测试。
class SelectionHistoryTest : public QObject
{
    Q_OBJECT

private slots:
    /// @brief 验证选区序列化后可以原样解析回来。
    void roundTripsSelections()
    {
        const QVector<QRect> history = {
            QRect(10, 20, 300, 200),
            QRect(-1920, 0, 1920, 1080),
        };

        const QVector<QRect> parsed = markshot::selectionHistoryFromJsonArray(
            markshot::selectionHistoryToJsonArray(history));

        QCOMPARE(parsed, history);
    }

    /// @brief 验证解析时会丢弃尺寸非法或重复的条目。
    void parsingSkipsInvalidEntries()
    {
        QJsonArray array;
        QJsonObject valid;
        valid.insert(QStringLiteral("x"), 0);
        valid.insert(QStringLiteral("y"), 0);
        valid.insert(QStringLiteral("w"), 100);
        valid.insert(QStringLiteral("h"), 50);
        array.append(valid);
        array.append(valid);  // 重复
        QJsonObject zeroSize = valid;
        zeroSize.insert(QStringLiteral("w"), 0);
        array.append(zeroSize);
        array.append(QStringLiteral("not-an-object"));

        const QVector<QRect> parsed = markshot::selectionHistoryFromJsonArray(array);

        QCOMPARE(parsed.size(), 1);
        QCOMPARE(parsed.first(), QRect(0, 0, 100, 50));
    }

    /// @brief 验证新选区前置、去重并限制条数。
    void remembersSelectionAtFront()
    {
        const QVector<QRect> history = {
            QRect(0, 0, 100, 100),
            QRect(50, 50, 200, 200),
        };

        const QVector<QRect> updated = markshot::selectionHistoryWithSelection(
            history, QRect(50, 50, 200, 200), 2);

        QCOMPARE(updated.size(), 2);
        QCOMPARE(updated.at(0), QRect(50, 50, 200, 200));
        QCOMPARE(updated.at(1), QRect(0, 0, 100, 100));
    }

    /// @brief 验证超出上限时丢弃最旧的条目。
    void dropsOldestBeyondLimit()
    {
        QVector<QRect> history;
        for (int i = 0; i < markshot::kSelectionHistoryLimit; ++i) {
            history.append(QRect(i, i, 10 + i, 10 + i));
        }

        const QVector<QRect> updated =
            markshot::selectionHistoryWithSelection(history, QRect(999, 999, 100, 100));

        QCOMPARE(updated.size(), markshot::kSelectionHistoryLimit);
        QCOMPARE(updated.first(), QRect(999, 999, 100, 100));
        QVERIFY(!updated.contains(history.last()));
    }

    /// @brief 验证空选区不会改动历史。
    void ignoresEmptySelection()
    {
        const QVector<QRect> history = {QRect(0, 0, 100, 100)};
        QCOMPARE(markshot::selectionHistoryWithSelection(history, QRect()), history);
    }
};

QTEST_MAIN(SelectionHistoryTest)
#include "selection_history_test.moc"
