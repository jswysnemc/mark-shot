#pragma once

#include "marketplace/plugin_index_parser.h"
#include "settings/settings_config.h"

#include <QWidget>

class QComboBox;
class QLabel;
class QPlainTextEdit;
class QPushButton;
class QVBoxLayout;
class QWidget;

namespace markshot::marketplace {
class PluginAssetDownloader;
class PluginMarketplaceClient;
}  // namespace markshot::marketplace

namespace markshot::settings {

class SettingsPagePlugins final : public QWidget {
public:
    /// @brief 创建插件管理设置页。
    /// @param parent 父控件。
    explicit SettingsPagePlugins(QWidget *parent = nullptr);

    /// @brief 将配置加载到页面控件。
    /// @param config 设置配置。
    void setConfig(const SettingsConfig &config);

    /// @brief 将页面控件值写回配置。
    /// @param config 需要更新的设置配置。
    void updateConfig(SettingsConfig *config) const;

private:
    /// @brief 构建 provider 选择卡片。
    /// @param layout 页面根布局。
    void buildProviderCard(QVBoxLayout *layout);

    /// @brief 构建 GitHub 插件市场卡片。
    /// @param layout 页面根布局。
    void buildMarketplaceCard(QVBoxLayout *layout);

    /// @brief 构建 OCR 模型下载卡片。
    /// @param layout 页面根布局。
    void buildOcrModelCard(QVBoxLayout *layout);

    /// @brief 构建插件目录卡片。
    /// @param layout 页面根布局。
    void buildDirectoriesCard(QVBoxLayout *layout);

    /// @brief 构建插件诊断卡片。
    /// @param layout 页面根布局。
    void buildDiagnosticsCard(QVBoxLayout *layout);

    /// @brief 刷新诊断表格。
    void refreshDiagnostics();

    /// @brief 从 GitHub 拉取插件索引并重建插件列表。
    void fetchMarketplaceIndex();

    /// @brief 用解析后的索引重建市场插件列表。
    /// @param index 插件索引。
    void rebuildMarketplaceList(const markshot::marketplace::PluginIndex &index);

    /// @brief 下载并安装一个市场插件资产。
    /// @param entry 插件索引条目。
    /// @param asset 当前平台资产。
    void installMarketplaceAsset(const markshot::marketplace::PluginIndexEntry &entry,
                                 const markshot::marketplace::PluginIndexAsset &asset);

    /// @brief 更新市场状态行文本。
    /// @param text 状态文本。
    /// @param tone 状态语义（muted/success/error）。
    void setMarketplaceStatus(const QString &text, const QString &tone);

    /// @brief 刷新 OCR 模型状态标签与下载按钮。
    void refreshOcrModelStatus();

    /// @brief 开始下载 OCR 模型文件。
    void startOcrModelDownload();

    /// @brief 下载队列中的下一个 OCR 模型文件。
    void downloadNextOcrModel();

    QComboBox *m_ocrProvider = nullptr;
    QComboBox *m_translationProvider = nullptr;
    QComboBox *m_codeScanProvider = nullptr;
    QPlainTextEdit *m_directories = nullptr;
    QPushButton *m_openUserDirectory = nullptr;
    QWidget *m_diagnosticsContainer = nullptr;
    QVBoxLayout *m_diagnosticsLayout = nullptr;

    // GitHub 插件市场。
    markshot::marketplace::PluginMarketplaceClient *m_marketplaceClient = nullptr;
    markshot::marketplace::PluginAssetDownloader *m_assetDownloader = nullptr;
    QLabel *m_marketplaceStatus = nullptr;
    QPushButton *m_marketplaceFetch = nullptr;
    QWidget *m_marketplaceListContainer = nullptr;
    QVBoxLayout *m_marketplaceListLayout = nullptr;
    QString m_pendingInstallTempPath;
    bool m_marketplaceInstallRunning = false;

    // OCR 模型下载（服务于 C++ rapid-onnx 插件，不依赖 Python）。
    markshot::marketplace::PluginAssetDownloader *m_ocrModelDownloader = nullptr;
    QLabel *m_ocrModelStatus = nullptr;
    QPushButton *m_ocrModelDownloadButton = nullptr;
    int m_ocrModelQueueIndex = -1;
};

}  // namespace markshot::settings
