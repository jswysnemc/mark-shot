#include "shot_window_internal.h"

#include <QJsonObject>
#include <QJsonValue>
#include <QtTest/QtTest>

#include <optional>

namespace markshot::shot {

/**
 * 提供配置解析测试不需要访问的配置文件路径。
 * @return 空配置路径。
 */
QString appConfigPath()
{
    return {};
}

/**
 * 提供配置解析测试不需要访问的颜色解析桩。
 * @param value 配置颜色值。
 * @return 空颜色结果。
 */
std::optional<QColor> colorFromConfigValue(const QJsonValue &value)
{
    Q_UNUSED(value);
    return std::nullopt;
}

/**
 * 提供配置解析测试使用的数值解析桩，只识别 JSON 数字。
 * @param value 配置数值。
 * @return JSON 数字返回对应数值，其他类型返回空结果。
 */
std::optional<qreal> realFromConfigValue(const QJsonValue &value)
{
    if (value.isDouble()) {
        return value.toDouble();
    }
    return std::nullopt;
}

}  // namespace markshot::shot

class PinnedWindowConfigTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证默认配置允许鼠标拖选复制文本。
     * @return 无返回值。
     */
    void defaultsAllowTextSelectionCopy()
    {
        const markshot::shot::PinnedWindowConfig config =
            markshot::shot::pinnedWindowConfigFromRoot(QJsonObject());

        QCOMPARE(config.textSelectionCopyEnabled, true);
    }

    /**
     * 验证置顶图片配置可以关闭鼠标拖选复制。
     * @return 无返回值。
     */
    void readsPinnedTextSelectionCopyFlag()
    {
        QJsonObject pinned;
        pinned.insert(QStringLiteral("textSelectionCopyEnabled"), false);
        QJsonObject root;
        root.insert(QStringLiteral("pinnedWindow"), pinned);

        const markshot::shot::PinnedWindowConfig config =
            markshot::shot::pinnedWindowConfigFromRoot(root);

        QCOMPARE(config.textSelectionCopyEnabled, false);
    }

    /**
     * 验证拖选复制别名兼容人工编辑的配置。
     * @return 无返回值。
     */
    void readsTextSelectionCopyAliases()
    {
        QJsonObject pinned;
        pinned.insert(QStringLiteral("allowTextSelectionCopy"), false);
        QJsonObject root;
        root.insert(QStringLiteral("pin"), pinned);

        const markshot::shot::PinnedWindowConfig config =
            markshot::shot::pinnedWindowConfigFromRoot(root);

        QCOMPARE(config.textSelectionCopyEnabled, false);
    }

    /**
     * 验证贴图阴影默认关闭，并可由配置打开。
     * @return 无返回值。
     */
    void readsPinnedShadowFlag()
    {
        const markshot::shot::PinnedWindowConfig defaults =
            markshot::shot::pinnedWindowConfigFromRoot(QJsonObject());
        QCOMPARE(defaults.shadowEnabled, false);

        QJsonObject pinned;
        pinned.insert(QStringLiteral("shadowEnabled"), true);
        QJsonObject root;
        root.insert(QStringLiteral("pinnedWindow"), pinned);
        const markshot::shot::PinnedWindowConfig config =
            markshot::shot::pinnedWindowConfigFromRoot(root);
        QCOMPARE(config.shadowEnabled, true);
    }

    /**
     * 验证失焦边框不透明度有默认值，读取配置时截断到 0 到 1。
     * @return 无返回值。
     */
    void readsInactiveBorderOpacity()
    {
        const markshot::shot::PinnedWindowConfig defaults =
            markshot::shot::pinnedWindowConfigFromRoot(QJsonObject());
        QCOMPARE(defaults.inactiveBorderOpacity, markshot::shot::kPinnedInactiveBorderOpacityDefault);

        QJsonObject pinned;
        pinned.insert(QStringLiteral("inactiveBorderOpacity"), 0.25);
        QJsonObject root;
        root.insert(QStringLiteral("pinnedWindow"), pinned);
        QCOMPARE(markshot::shot::pinnedWindowConfigFromRoot(root).inactiveBorderOpacity, 0.25);

        pinned.insert(QStringLiteral("inactiveBorderOpacity"), 3.0);
        root.insert(QStringLiteral("pinnedWindow"), pinned);
        QCOMPARE(markshot::shot::pinnedWindowConfigFromRoot(root).inactiveBorderOpacity, 1.0);
    }
};

QTEST_APPLESS_MAIN(PinnedWindowConfigTest)

#include "pinned_window_config_test.moc"
