#include "ocr_result_window_config.h"

#include <QtTest/QtTest>

class OcrResultWindowConfigTest : public QObject {
    Q_OBJECT

private slots:
    /// @brief 验证默认 OCR 窗口可由窗口管理器识别，且不继承钉图置顶开关
    /// @return 无返回值
    void defaultsToManagedWindow()
    {
        QVERIFY(!markshot::shot::ocrResultWindowAlwaysOnTopFromRoot({}));
        const QJsonObject root{{"pinnedWindow", QJsonObject{{"alwaysOnTop", true}}}};
        QVERIFY(!markshot::shot::ocrResultWindowAlwaysOnTopFromRoot(root));
    }

    /// @brief 验证 OCR 独立配置能明确开启和关闭置顶
    /// @return 无返回值
    void usesIndependentSetting()
    {
        const QJsonObject enabled{{"ocrResultWindow", QJsonObject{{"alwaysOnTop", true}}}};
        const QJsonObject disabled{{"ocrResultWindow", QJsonObject{{"alwaysOnTop", false}}}};
        const QJsonObject invalid{{"ocrResultWindow", QJsonObject{{"alwaysOnTop", "invalid"}}}};
        QVERIFY(markshot::shot::ocrResultWindowAlwaysOnTopFromRoot(enabled));
        QVERIFY(!markshot::shot::ocrResultWindowAlwaysOnTopFromRoot(disabled));
        QVERIFY(!markshot::shot::ocrResultWindowAlwaysOnTopFromRoot(invalid));
    }
};

QTEST_APPLESS_MAIN(OcrResultWindowConfigTest)

#include "ocr_result_window_config_test.moc"
