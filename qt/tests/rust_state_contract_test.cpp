#include "annotation_state_store.h"
#include "annotation_style_codec.h"
#include "app_config_defaults.h"
#include "app_config_store.h"
#include "json_bridge.h"
#include "startup_config.h"
#include "support/isolated_config.h"
#include "window_detection.h"

#include <QFile>
#include <QtTest>

namespace {

/** 将 bytes 写入明确的 path，返回是否完整写入；仅供本测试的临时目录使用。 */
bool writeFile(const QString &path, const QByteArray &bytes)
{
    if (!QDir().mkpath(QFileInfo(path).absolutePath())) {
        return false;
    }
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}

/** 从 path 读取测试文件，返回字节；读取失败时返回空字节。 */
QByteArray readFile(const QString &path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray{};
}

} // namespace

/// 使用原 Qt 固定样本和隔离文件系统验证实际配置、样式及启动调用链。
class RustStateContractTest final : public QObject {
    Q_OBJECT

private slots:
    /** 为每项测试分配独立配置路径，无参数、无返回值。 */
    void init()
    {
        QVERIFY(m_directory.isValid());
        const QString path = m_directory.filePath(QStringLiteral("case-%1").arg(++m_case));
        QVERIFY(QDir().mkpath(path));
        qputenv("MARK_SHOT_CONFIG_DIR", path.toUtf8());
    }

    /** 【配置测试】【默认配置】保留迁移前字段基准，并核对新增区域识别默认值，无参数、无返回值。 */
    void defaultsMatchOriginalQtObject()
    {
        // 1. 【配置测试】【默认配置】保留迁移前 Qt 配置基准及平台默认值
        QJsonObject expected = QJsonDocument::fromJson(readFile(QStringLiteral(MARK_SHOT_DEFAULTS_FIXTURE))).object();
        QVERIFY(!expected.isEmpty());
#if defined(Q_OS_WIN)
        auto windows = expected.value(QStringLiteral("windows")).toObject();
        windows.insert(QStringLiteral("tray"), QJsonObject{{QStringLiteral("enabled"), true}});
        expected.insert(QStringLiteral("windows"), windows);
#endif
        // 2. 【配置测试】【默认配置】仅补充新增字段，不改写历史迁移样本
        auto capture = expected.value(QStringLiteral("capture")).toObject();
        capture.insert(QStringLiteral("regionDetection"), QJsonObject{{QStringLiteral("enabled"), true}});
        expected.insert(QStringLiteral("capture"), capture);
        QCOMPARE(markshot::defaultAppConfigRoot(QStringLiteral("test-window-helper")), expected);
    }

    /** 验证真实配置入口的初始化、局部更新、删除与未知字段保留，无参数、无返回值。 */
    void applicationStoreKeepsUnknownFieldsAndNullSemantics()
    {
        bool ok = false;
        QJsonObject root = markshot::readAppConfigRoot(&ok);
        QVERIFY(ok);
        root.insert(QStringLiteral("unknown"), QJsonArray{1, 2, 3});
        QString error = QStringLiteral("old error");
        QVERIFY(markshot::writeAppConfigRoot(root, &error));
        QVERIFY(error.isEmpty());
        QVERIFY(markshot::writeAppConfigValue({QStringLiteral("export"), QStringLiteral("imageFrame"), QStringLiteral("padding")}, 17, &error));
        QVERIFY(markshot::writeAppConfigValue({QStringLiteral("nullable")}, QJsonValue(QJsonValue::Null), &error));
        root = markshot::readAppConfigRoot(&ok);
        QVERIFY(ok);
        QCOMPARE(root.value(QStringLiteral("unknown")), QJsonValue(QJsonArray{1, 2, 3}));
        QVERIFY(root.contains(QStringLiteral("nullable")));
        QVERIFY(root.value(QStringLiteral("nullable")).isNull());
        QCOMPARE(root.value(QStringLiteral("export")).toObject().value(QStringLiteral("imageFrame")).toObject().value(QStringLiteral("padding")).toInt(), 17);
        QVERIFY(markshot::writeAppConfigValue({QStringLiteral("nullable")}, QJsonValue(QJsonValue::Undefined), &error));
        QVERIFY(!markshot::readAppConfigRoot().contains(QStringLiteral("nullable")));

        const QByteArray before = readFile(markshot::appConfigPath());
        QVERIFY(!markshot::writeAppConfigValue({}, 1, &error));
        QVERIFY(!error.isEmpty());
        QCOMPARE(readFile(markshot::appConfigPath()), before);
    }

    /** 验证损坏配置不会被默认值或失败更新覆盖，无参数、无返回值。 */
    void malformedConfigRemainsUntouched()
    {
        const QString path = markshot::appConfigPath();
        QVERIFY(writeFile(path, "broken"));
        bool ok = true;
        QVERIFY(markshot::readAppConfigRoot(&ok).isEmpty());
        QVERIFY(!ok);
        QString error;
        QVERIFY(!markshot::writeAppConfigValue({QStringLiteral("new")}, 1, &error));
        QVERIFY(!error.isEmpty());
        QCOMPARE(readFile(path), QByteArray("broken"));
    }

    /** 对照原 Qt 样式加载和保存结果，包含缺失、损坏、旧字段及边界输入，无参数、无返回值。 */
    void annotationStateMatchesOriginalQtFixtures()
    {
        const auto fixture = QJsonDocument::fromJson(readFile(QStringLiteral(MARK_SHOT_STATE_FIXTURE))).object();
        const auto cases = fixture.value(QStringLiteral("cases")).toArray();
        QVERIFY(!cases.isEmpty());
        const QString path = markshot::annotationStateFilePath();
        for (const auto &value : cases) {
            const QJsonObject item = value.toObject();
            const QJsonValue input = item.value(QStringLiteral("input"));
            if (input.isNull()) {
                QFile::remove(path);
            } else {
                QVERIFY(writeFile(path, input.toString().toUtf8()));
            }
            const auto state = markshot::loadAnnotationState();
            const auto actual = markshot::rust::encodeAnnotationStyle(state);
            const auto expected = item.value(QStringLiteral("expected")).toObject();
            QVERIFY2(actual == expected, qPrintable(item.value(QStringLiteral("name")).toString()
                + QStringLiteral(": ") + QString::fromUtf8(QJsonDocument(actual).toJson(QJsonDocument::Compact))));
            if (input.isNull()) {
                QVERIFY(!QFileInfo::exists(path));
            } else {
                QCOMPARE(readFile(path), input.toString().toUtf8());
            }
            QVERIFY(markshot::saveAnnotationState(state));
            QCOMPARE(QJsonDocument::fromJson(readFile(path)).object(), expected);
        }
    }

    /** 按状态文件的 8 位 ARGB 语义对照 QColor 名称与颜色格式，无参数、无返回值。 */
    void colorsMatchQtNamesAndPrecision()
    {
        QStringList names = QColor::colorNames();
        names.append({QStringLiteral("Light Grey"), QStringLiteral("r e d"), QStringLiteral("blue\t"),
                      QStringLiteral("\nred"), QStringLiteral("red\n"), QStringLiteral("\u00a0red"),
                      QStringLiteral("#abc"), QStringLiteral("#80123456"), QStringLiteral("#123abc456"),
                      QStringLiteral("#11118000ffff"), QStringLiteral(" #fff"), QStringLiteral("#fff ")});
        for (const QString &name : names) {
            const auto bytes = QJsonDocument(QJsonObject{{QStringLiteral("currentColor"), name}}).toJson(QJsonDocument::Compact);
            QVERIFY(writeFile(markshot::annotationStateFilePath(), bytes));
            const QColor color(name);
            const QColor expected = color.isValid() ? color : markshot::AnnotationState{}.currentColor;
            QCOMPARE(markshot::loadAnnotationState().currentColor.rgba(), expected.rgba());
        }
    }

    /** 验证启动环境读取也遵守显式目录，缺失时不读取备用 HOME 配置，无参数、无返回值。 */
    void startupEnvironmentNeverFallsBackFromExplicitDirectory()
    {
        const QByteArray marker("MARK_SHOT_CONTRACT_ISOLATION");
        qunsetenv(marker.constData());
        const auto fallback = QDir::home().filePath(QStringLiteral(".config/mark-shot/config.json"));
        const QByteArray fallbackJson = R"({"env":{"MARK_SHOT_CONTRACT_ISOLATION":"fallback"}})";
        QVERIFY(writeFile(fallback, fallbackJson));
        markshot::applyConfiguredEnvironment();
        QVERIFY(!qEnvironmentVariableIsSet(marker.constData()));

        QVERIFY(writeFile(markshot::appConfigPath(), R"({"env":{"MARK_SHOT_CONTRACT_ISOLATION":"explicit"}})"));
        markshot::applyConfiguredEnvironment();
        QCOMPARE(qgetenv(marker.constData()), QByteArray("explicit"));
        QCOMPARE(readFile(fallback), fallbackJson);
        qunsetenv(marker.constData());
    }

private:
    QTemporaryDir m_directory;
    int m_case = 0;
};

/** 在独立 HOME 和配置目录执行测试，argc/argv 为参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) {
        return 1;
    }
    QApplication application(argc, argv);
    RustStateContractTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "rust_state_contract_test.moc"
