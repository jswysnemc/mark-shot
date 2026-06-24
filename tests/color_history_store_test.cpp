#include "ui/color_history_store.h"

#include <QJsonArray>
#include <QtTest/QtTest>

/// @brief 历史取色状态测试。
class ColorHistoryStoreTest : public QObject
{
    Q_OBJECT

private slots:
    /// @brief 验证颜色可以转换为稳定配置文本。
    void writesStableColorNames()
    {
        QCOMPARE(markshot::ui::colorHistoryConfigName(QColor(1, 2, 3, 4)),
                 QStringLiteral("#01020304"));
    }

    /// @brief 验证颜色配置文本可以解析透明度。
    void parsesColorNames()
    {
        const std::optional<QColor> color =
            markshot::ui::colorHistoryColorFromString(QStringLiteral("#AABBCC80"));
        QVERIFY(color.has_value());
        QCOMPARE(color->red(), 0xAA);
        QCOMPARE(color->green(), 0xBB);
        QCOMPARE(color->blue(), 0xCC);
        QCOMPARE(color->alpha(), 0x80);
    }

    /// @brief 验证新增颜色会前置、去重并限制数量。
    void remembersColorAtFront()
    {
        QVector<QColor> history = {
            QColor(10, 10, 10),
            QColor(20, 20, 20),
            QColor(10, 10, 10),
        };

        const QVector<QColor> updated =
            markshot::ui::colorHistoryWithRememberedColor(history, QColor(20, 20, 20), 2);

        QCOMPARE(updated.size(), 2);
        QCOMPARE(updated.at(0), QColor(20, 20, 20));
        QCOMPARE(updated.at(1), QColor(10, 10, 10));
    }

    /// @brief 验证可以从应用配置根对象读取历史取色。
    void readsConfigRoot()
    {
        QJsonArray history;
        history.append(QStringLiteral("#112233FF"));
        history.append(QStringLiteral("#44556680"));
        history.append(QStringLiteral("invalid"));

        QJsonObject colorPicker;
        colorPicker.insert(QStringLiteral("history"), history);

        QJsonObject root;
        root.insert(QStringLiteral("colorPicker"), colorPicker);

        const QVector<QColor> colors = markshot::ui::colorHistoryFromConfigRoot(root);
        QCOMPARE(colors.size(), 2);
        QCOMPARE(colors.at(0), QColor(0x11, 0x22, 0x33, 0xFF));
        QCOMPARE(colors.at(1), QColor(0x44, 0x55, 0x66, 0x80));
    }
};

/// @brief 历史取色状态测试入口。
/// @param argc 参数数量。
/// @param argv 参数数组。
/// @return 进程退出码。
QTEST_APPLESS_MAIN(ColorHistoryStoreTest)

#include "color_history_store_test.moc"
