#include "app_config_defaults.h"
#include "app_config_store.h"
#include "settings/settings_config.h"
#include "settings/settings_page_capture.h"
#include "shot_window.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QCheckBox>
#include <QTemporaryDir>
#include <QtTest>

class RegionDetectionToggleTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【截图测试】【区域识别】每个用例恢复独立默认配置，无参数和返回值。 */
    void init()
    {
        QVERIFY(markshot::writeAppConfigRoot(markshot::defaultAppConfigRoot({})));
    }

    /** 【设置测试】【区域识别】验证统一开关、中文文案及配置保存，无参数和返回值。 */
    void settingsRoundTrip()
    {
        using namespace markshot::settings;
        markshot::i18n::setLanguage(markshot::i18n::Language::Chinese);
        SettingsConfig settings = readSettingsConfig();
        QVERIFY(settings.capture.regionDetectionEnabled);
        const bool windowDetectionEnabled = settings.advanced.windowDetectionEnabled;
        SettingsPageCapture page;
        page.setConfig(settings);
        auto *toggle = page.findChild<QCheckBox *>(QStringLiteral("captureRegionDetection"));
        QVERIFY(toggle);
        QVERIFY(toggle->isChecked());

        // 1. 【设置测试】【区域识别】关闭图像分析后保存，独立窗口识别保持原值
        toggle->setChecked(false);
        page.updateConfig(&settings);
        QString error;
        QVERIFY2(writeSettingsConfig(settings, &error), qPrintable(error));
        settings = readSettingsConfig();
        QVERIFY(!settings.capture.regionDetectionEnabled);
        QCOMPARE(settings.advanced.windowDetectionEnabled, windowDetectionEnabled);
        QCOMPARE(markshot::i18n::translate(QStringLiteral("Region Detection")), QStringLiteral("区域识别"));

        // 2. 【设置测试】【区域识别】重新打开后保存，后续读取恢复图像分析
        toggle->setChecked(true);
        page.updateConfig(&settings);
        QVERIFY2(writeSettingsConfig(settings, &error), qPrintable(error));
        QVERIFY(readSettingsConfig().capture.regionDetectionEnabled);
    }

    /** 【截图测试】【区域识别】提供图像分析与窗口识别的独立开关组合，无参数和返回值。 */
    void detectionModes_data()
    {
        QTest::addColumn<bool>("regionEnabled");
        QTest::addColumn<bool>("windowEnabled");
        QTest::newRow("regions-and-windows") << true << true;
        QTest::newRow("regions-only") << true << false;
        QTest::newRow("windows-only") << false << true;
        QTest::newRow("manual-only") << false << false;
    }

    /** 【截图测试】【区域识别】验证任务调度、窗口回退和区域滚轮互不混淆，无参数和返回值。 */
    void detectionModes()
    {
        QFETCH(bool, regionEnabled);
        QFETCH(bool, windowEnabled);
        auto settings = markshot::settings::readSettingsConfig();
        settings.capture.regionDetectionEnabled = regionEnabled;
        settings.advanced.windowDetectionEnabled = windowEnabled;
        QString error;
        QVERIFY2(markshot::settings::writeSettingsConfig(settings, &error), qPrintable(error));

        // 1. 【截图测试】【区域识别】使用已知窗口矩形，避免测试依赖实际合成器
        QImage frame(320, 240, QImage::Format_RGB32);
        frame.fill(Qt::white);
        markshot::WindowInfo windowInfo;
        windowInfo.rect = QRect(40, 50, 120, 90);
        ShotWindow window(frame, {}, QRect(0, 0, 320, 240), {windowInfo}, windowEnabled);
        window.initializeRegionDetection();
        QCOMPARE(window.m_windowInfos.size(), windowEnabled ? 1 : 0);

        // 2. 【截图测试】【区域识别】开启时等待分析结果，关闭时确认没有任务或图像候选
        if (regionEnabled) {
            QVERIFY(window.m_regionDetectorWatcher);
            QTRY_VERIFY(window.m_regionDetector != nullptr);
            QCOMPARE(window.m_regionDetector->bounds(), frame.rect());
        } else {
            QVERIFY(!window.m_regionDetectorWatcher);
            QVERIFY(!window.m_regionDetector);
            const std::optional<QRect> expected = windowEnabled
                ? std::optional<QRect>(windowInfo.rect) : std::nullopt;
            QCOMPARE(window.hoveredRegionAt(QPoint(60, 70)), expected);
            QVERIFY(!window.hoveredRegionAt(QPoint(10, 10)));
            QVERIFY(!window.cycleHoveredRegion(QPoint(60, 70), 1));
        }
    }
};

/**
 * 【截图测试】【配置隔离】在临时配置文件下运行区域识别交互测试。
 * @param argc 命令行参数数量。
 * @param argv 命令行参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    if (!isolated.isValid()) {
        return 1;
    }
    qputenv("MARK_SHOT_CONFIG", isolated.filePath(QStringLiteral("config.json")).toUtf8());
    QApplication app(argc, argv);
    RegionDetectionToggleTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "region_detection_toggle_test.moc"
