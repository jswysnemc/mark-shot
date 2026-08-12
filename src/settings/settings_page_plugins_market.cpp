#include "settings/settings_page_plugins.h"

#include "app_config_store.h"
#include "marketplace/plugin_asset_downloader.h"
#include "marketplace/plugin_installer.h"
#include "marketplace/plugin_marketplace_client.h"
#include "settings/settings_page_plugins_model.h"
#include "settings/settings_ui_helpers.h"
#include "ui/i18n.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QHBoxLayout>
#include <QJsonObject>
#include <QLabel>
#include <QLayoutItem>
#include <QPushButton>
#include <QStandardPaths>
#include <QStyle>
#include <QUrl>
#include <QVBoxLayout>

namespace markshot::settings {
namespace {

/// @brief 官方插件索引地址，托管在主仓库 main 分支。
const char kDefaultPluginIndexUrl[] =
    "https://raw.githubusercontent.com/jswysnemc/mark-shot/main/marketplace/plugin-index.json";

/**
 * 读取插件索引地址，支持通过配置 marketplace.indexUrl 覆盖。
 * @return 插件索引 URL。
 */
QUrl pluginIndexUrl()
{
    bool ok = false;
    const QJsonObject root = markshot::readAppConfigRoot(&ok);
    if (ok) {
        const QString configured = root.value(QStringLiteral("marketplace"))
                                       .toObject()
                                       .value(QStringLiteral("indexUrl"))
                                       .toString()
                                       .trimmed();
        if (!configured.isEmpty()) {
            return QUrl(configured);
        }
    }
    return QUrl(QLatin1String(kDefaultPluginIndexUrl));
}

/**
 * 清空布局中的全部条目。
 * @param layout 需要清空的布局。
 * @return 无返回值。
 */
void clearMarketplaceLayout(QLayout *layout)
{
    if (!layout) {
        return;
    }
    while (QLayoutItem *item = layout->takeAt(0)) {
        delete item->widget();
        delete item;
    }
}

/// @brief 一个需要下载的 OCR 模型文件。
struct OcrModelAsset {
    const char *fileName;
    const char *url;
    const char *sha256;
};

/// @brief C++ rapid-onnx OCR 插件需要的 PP-OCRv5 mobile 模型清单。
/// 直接使用 RapidOCR 官方模型发布源（ModelScope），URL 与 SHA-256 取自
/// rapidocr 的 default_models.yaml，与官方 Python 包下载的内容完全一致。
constexpr OcrModelAsset kOcrModelAssets[] = {
    {"ch_PP-OCRv5_det_mobile.onnx",
     "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.8.0/onnx/PP-OCRv5/det/ch_PP-OCRv5_det_mobile.onnx",
     "4d97c44a20d30a81aad087d6a396b08f786c4635742afc391f6621f5c6ae78ae"},
    {"ch_PP-OCRv5_rec_mobile.onnx",
     "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.8.0/onnx/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile.onnx",
     "5825fc7ebf84ae7a412be049820b4d86d77620f204a041697b0494669b1742c5"},
    {"ppocrv5_dict.txt",
     "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.8.0/paddle/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile/ppocrv5_dict.txt",
     "d1979e9f794c464c0d2e0b70a7fe14dd978e9dc644c0e71f14158cdf8342af1b"},
};

/// @brief OCR 模型文件个数。
constexpr int kOcrModelAssetCount = static_cast<int>(sizeof(kOcrModelAssets) / sizeof(kOcrModelAssets[0]));

/**
 * 返回 OCR 模型目录，与 rapid-onnx 插件的搜索约定一致。
 * @return 模型目录路径。
 */
QString ocrModelsDirectory()
{
    const QString envDir = qEnvironmentVariable("MARK_SHOT_OCR_MODEL_DIR").trimmed();
    if (!envDir.isEmpty()) {
        return envDir;
    }
    QString base = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (base.isEmpty()) {
        base = QDir::home().filePath(QStringLiteral(".local/share"));
    }
    return QDir(base).filePath(QStringLiteral("mark-shot/models"));
}

/**
 * 判断全部 OCR 模型文件是否已就绪。
 * @return 模型齐全时返回 true。
 */
bool ocrModelsInstalled()
{
    const QDir modelsDir(ocrModelsDirectory());
    for (const OcrModelAsset &asset : kOcrModelAssets) {
        if (!QFileInfo::exists(modelsDir.filePath(QLatin1String(asset.fileName)))) {
            return false;
        }
    }
    return true;
}

/**
 * 设置状态标签的语义色。
 * @param label 状态标签。
 * @param tone 语义标识。
 * @return 无返回值。
 */
void applyStatusTone(QLabel *label, const QString &tone)
{
    if (!label) {
        return;
    }
    label->setProperty("tone", tone);
    label->style()->unpolish(label);
    label->style()->polish(label);
}

}  // namespace

void SettingsPagePlugins::buildMarketplaceCard(QVBoxLayout *layout)
{
    QFrame *card = createSettingsCard(
        MS_TR("Get Plugins from GitHub"),
        MS_TR("Download official provider plugins (OCR, translation, code scanning) from GitHub "
              "Releases. Downloads are verified with SHA-256 and installed into the user plugin "
              "folder. Restart Mark Shot to load newly installed plugins. Prebuilt plugins are "
              "linked against the build machine's system libraries; if a plugin fails to load "
              "(see diagnostics below), use your distribution package or a local build instead."),
        this);
    QFormLayout *form = settingsCardForm(card);

    auto *statusRow = new QWidget(card);
    auto *statusLayout = new QHBoxLayout(statusRow);
    statusLayout->setContentsMargins(0, 0, 0, 0);
    statusLayout->setSpacing(10);
    m_marketplaceFetch = new QPushButton(MS_TR("Fetch Plugin List"), statusRow);
    m_marketplaceFetch->setCursor(Qt::PointingHandCursor);
    m_marketplaceStatus = new QLabel(MS_TR("Plugin list has not been fetched yet."), statusRow);
    m_marketplaceStatus->setObjectName(QStringLiteral("pluginDiagnosticMeta"));
    m_marketplaceStatus->setWordWrap(true);
    statusLayout->addWidget(m_marketplaceFetch, 0, Qt::AlignTop);
    statusLayout->addWidget(m_marketplaceStatus, 1);
    form->addRow(statusRow);

    m_marketplaceListContainer = new QWidget(card);
    m_marketplaceListLayout = new QVBoxLayout(m_marketplaceListContainer);
    m_marketplaceListLayout->setContentsMargins(0, 0, 0, 0);
    m_marketplaceListLayout->setSpacing(8);
    form->addRow(m_marketplaceListContainer);

    m_marketplaceClient = new markshot::marketplace::PluginMarketplaceClient(this);
    m_assetDownloader = new markshot::marketplace::PluginAssetDownloader(this);

    connect(m_marketplaceFetch, &QPushButton::clicked, this, [this] { fetchMarketplaceIndex(); });
    connect(m_marketplaceClient,
            &markshot::marketplace::PluginMarketplaceClient::indexReady,
            this,
            [this](const markshot::marketplace::PluginIndex &index) {
                if (m_marketplaceFetch) {
                    m_marketplaceFetch->setEnabled(true);
                }
                rebuildMarketplaceList(index);
            });
    connect(m_marketplaceClient,
            &markshot::marketplace::PluginMarketplaceClient::failed,
            this,
            [this](const QString &error) {
                if (m_marketplaceFetch) {
                    m_marketplaceFetch->setEnabled(true);
                }
                setMarketplaceStatus(MS_TR("Failed to fetch plugin list: %1").arg(error),
                                     QStringLiteral("error"));
            });

    layout->addWidget(card);
}

void SettingsPagePlugins::fetchMarketplaceIndex()
{
    if (!m_marketplaceClient || m_marketplaceInstallRunning) {
        return;
    }
    if (m_marketplaceFetch) {
        m_marketplaceFetch->setEnabled(false);
    }
    setMarketplaceStatus(MS_TR("Fetching plugin list from GitHub..."), QStringLiteral("muted"));
    m_marketplaceClient->fetchIndex(pluginIndexUrl());
}

void SettingsPagePlugins::rebuildMarketplaceList(const markshot::marketplace::PluginIndex &index)
{
    if (!m_marketplaceListLayout) {
        return;
    }
    clearMarketplaceLayout(m_marketplaceListLayout);

    int visibleCount = 0;
    for (const markshot::marketplace::PluginIndexEntry &entry : index.plugins) {
        const QVector<markshot::marketplace::PluginIndexAsset> assets =
            markshot::marketplace::assetsForCurrentPlatform(entry);
        if (assets.isEmpty()) {
            continue;
        }
        const markshot::marketplace::PluginIndexAsset asset = assets.first();
        ++visibleCount;

        auto *item = new QFrame(m_marketplaceListContainer);
        item->setObjectName(QStringLiteral("pluginDiagnosticItem"));
        auto *itemLayout = new QHBoxLayout(item);
        itemLayout->setContentsMargins(12, 10, 12, 10);
        itemLayout->setSpacing(10);

        auto *textLayout = new QVBoxLayout;
        textLayout->setContentsMargins(0, 0, 0, 0);
        textLayout->setSpacing(2);
        auto *name = new QLabel(QStringLiteral("%1  %2").arg(entry.name, entry.version), item);
        name->setObjectName(QStringLiteral("pluginDiagnosticProvider"));
        auto *description = new QLabel(entry.description, item);
        description->setObjectName(QStringLiteral("pluginDiagnosticMeta"));
        description->setWordWrap(true);
        textLayout->addWidget(name);
        textLayout->addWidget(description);
        itemLayout->addLayout(textLayout, 1);

        // 已装同名库时提示重新安装，避免用户不确定按钮语义
        const QString libraryName = asset.libraryFileName.isEmpty()
            ? asset.fileName
            : asset.libraryFileName;
        const bool installed = QFile::exists(QDir(userPluginDirectory()).filePath(libraryName));
        auto *install = new QPushButton(installed ? MS_TR("Reinstall") : MS_TR("Install"), item);
        install->setCursor(Qt::PointingHandCursor);
        connect(install, &QPushButton::clicked, this, [this, entry, asset] {
            installMarketplaceAsset(entry, asset);
        });
        itemLayout->addWidget(install, 0, Qt::AlignVCenter);

        m_marketplaceListLayout->addWidget(item);
    }

    setMarketplaceStatus(visibleCount > 0
                             ? MS_TR("%1 plugins are available for this platform.").arg(visibleCount)
                             : MS_TR("No plugins are available for this platform."),
                         visibleCount > 0 ? QStringLiteral("success") : QStringLiteral("muted"));
}

void SettingsPagePlugins::installMarketplaceAsset(
    const markshot::marketplace::PluginIndexEntry &entry,
    const markshot::marketplace::PluginIndexAsset &asset)
{
    if (!m_assetDownloader || m_marketplaceInstallRunning) {
        return;
    }
    m_marketplaceInstallRunning = true;
    setMarketplaceStatus(MS_TR("Downloading %1...").arg(entry.name), QStringLiteral("muted"));

    const QString tempDir = QStandardPaths::writableLocation(QStandardPaths::TempLocation);
    m_pendingInstallTempPath = QDir(tempDir.isEmpty() ? QDir::tempPath() : tempDir)
                                   .filePath(QStringLiteral("mark-shot-plugin-download-%1")
                                                 .arg(asset.fileName));
    QFile::remove(m_pendingInstallTempPath);

    // 单次安装期间只保留本次连接，避免多次点击叠加旧回调
    disconnect(m_assetDownloader, nullptr, this, nullptr);
    connect(m_assetDownloader,
            &markshot::marketplace::PluginAssetDownloader::progress,
            this,
            [this, entry](qint64 received, qint64 total) {
                if (total > 0) {
                    setMarketplaceStatus(MS_TR("Downloading %1... %2%")
                                             .arg(entry.name)
                                             .arg(received * 100 / total),
                                         QStringLiteral("muted"));
                }
            });
    connect(m_assetDownloader,
            &markshot::marketplace::PluginAssetDownloader::finished,
            this,
            [this, entry, asset](const QString &downloadedPath) {
                markshot::marketplace::PluginInstallRequest request;
                request.sourcePath = downloadedPath;
                request.fileName = asset.libraryFileName.isEmpty() ? asset.fileName
                                                                   : asset.libraryFileName;
                request.destinationDirectory = userPluginDirectory();
                request.expectedSha256 = asset.sha256;
                const markshot::marketplace::PluginInstallResult result =
                    markshot::marketplace::installPluginAsset(request);
                QFile::remove(downloadedPath);
                m_pendingInstallTempPath.clear();
                m_marketplaceInstallRunning = false;
                if (result.success) {
                    setMarketplaceStatus(MS_TR("%1 installed. Restart Mark Shot to load it.")
                                             .arg(entry.name),
                                         QStringLiteral("success"));
                    refreshDiagnostics();
                } else {
                    setMarketplaceStatus(MS_TR("Failed to install %1: %2")
                                             .arg(entry.name, result.error),
                                         QStringLiteral("error"));
                }
            });
    connect(m_assetDownloader,
            &markshot::marketplace::PluginAssetDownloader::failed,
            this,
            [this, entry](const QString &error) {
                m_pendingInstallTempPath.clear();
                m_marketplaceInstallRunning = false;
                setMarketplaceStatus(MS_TR("Failed to download %1: %2").arg(entry.name, error),
                                     QStringLiteral("error"));
            });

    markshot::marketplace::PluginAssetDownloadRequest request;
    request.url = QUrl(asset.downloadUrl);
    request.sha256 = asset.sha256;
    request.destinationPath = m_pendingInstallTempPath;
    m_assetDownloader->download(request);
}

void SettingsPagePlugins::setMarketplaceStatus(const QString &text, const QString &tone)
{
    if (!m_marketplaceStatus) {
        return;
    }
    m_marketplaceStatus->setText(text);
    applyStatusTone(m_marketplaceStatus, tone);
}

void SettingsPagePlugins::buildOcrModelCard(QVBoxLayout *layout)
{
    QFrame *card = createSettingsCard(
        MS_TR("OCR Models (PP-OCRv5)"),
        MS_TR("The built-in OCR plugin (PP-OCR with ONNX Runtime) needs the detection and "
              "recognition models plus the dictionary. They are downloaded from the official "
              "RapidOCR model releases with SHA-256 verification. No Python required."),
        this);
    QFormLayout *form = settingsCardForm(card);

    auto *statusRow = new QWidget(card);
    auto *statusLayout = new QHBoxLayout(statusRow);
    statusLayout->setContentsMargins(0, 0, 0, 0);
    statusLayout->setSpacing(10);
    m_ocrModelDownloadButton = new QPushButton(MS_TR("Download Models (~21 MB)"), statusRow);
    m_ocrModelDownloadButton->setCursor(Qt::PointingHandCursor);
    m_ocrModelStatus = new QLabel(statusRow);
    m_ocrModelStatus->setObjectName(QStringLiteral("pluginDiagnosticMeta"));
    m_ocrModelStatus->setWordWrap(true);
    statusLayout->addWidget(m_ocrModelDownloadButton, 0, Qt::AlignTop);
    statusLayout->addWidget(m_ocrModelStatus, 1);
    form->addRow(statusRow);

    m_ocrModelDownloader = new markshot::marketplace::PluginAssetDownloader(this);
    connect(m_ocrModelDownloadButton, &QPushButton::clicked, this, [this] {
        startOcrModelDownload();
    });

    refreshOcrModelStatus();
    layout->addWidget(card);
}

void SettingsPagePlugins::refreshOcrModelStatus()
{
    if (!m_ocrModelStatus) {
        return;
    }
    const bool downloading = m_ocrModelQueueIndex >= 0;
    if (downloading) {
        return;
    }
    if (ocrModelsInstalled()) {
        m_ocrModelStatus->setText(MS_TR("Models installed at %1").arg(ocrModelsDirectory()));
        applyStatusTone(m_ocrModelStatus, QStringLiteral("success"));
        if (m_ocrModelDownloadButton) {
            m_ocrModelDownloadButton->setText(MS_TR("Download Again"));
            m_ocrModelDownloadButton->setEnabled(true);
        }
    } else {
        m_ocrModelStatus->setText(MS_TR("Models are not downloaded yet."));
        applyStatusTone(m_ocrModelStatus, QStringLiteral("muted"));
        if (m_ocrModelDownloadButton) {
            m_ocrModelDownloadButton->setText(MS_TR("Download Models (~21 MB)"));
            m_ocrModelDownloadButton->setEnabled(true);
        }
    }
}

void SettingsPagePlugins::startOcrModelDownload()
{
    if (!m_ocrModelDownloader || m_ocrModelQueueIndex >= 0) {
        return;
    }
    m_ocrModelQueueIndex = 0;
    if (m_ocrModelDownloadButton) {
        m_ocrModelDownloadButton->setEnabled(false);
    }
    downloadNextOcrModel();
}

void SettingsPagePlugins::downloadNextOcrModel()
{
    if (m_ocrModelQueueIndex < 0) {
        return;
    }
    if (m_ocrModelQueueIndex >= kOcrModelAssetCount) {
        // 全部文件就绪。插件在首次加载失败后会缓存错误，提示重启一次。
        m_ocrModelQueueIndex = -1;
        m_ocrModelStatus->setText(
            MS_TR("OCR models are ready at %1. Restart Mark Shot if OCR was already used in "
                  "this session.")
                .arg(ocrModelsDirectory()));
        applyStatusTone(m_ocrModelStatus, QStringLiteral("success"));
        if (m_ocrModelDownloadButton) {
            m_ocrModelDownloadButton->setText(MS_TR("Download Again"));
            m_ocrModelDownloadButton->setEnabled(true);
        }
        refreshDiagnostics();
        return;
    }

    const OcrModelAsset &asset = kOcrModelAssets[m_ocrModelQueueIndex];
    const QString fileName = QLatin1String(asset.fileName);
    m_ocrModelStatus->setText(MS_TR("Downloading %1...").arg(fileName));
    applyStatusTone(m_ocrModelStatus, QStringLiteral("muted"));

    disconnect(m_ocrModelDownloader, nullptr, this, nullptr);
    connect(m_ocrModelDownloader,
            &markshot::marketplace::PluginAssetDownloader::progress,
            this,
            [this, fileName](qint64 received, qint64 total) {
                if (total > 0) {
                    m_ocrModelStatus->setText(MS_TR("Downloading %1... %2%")
                                                  .arg(fileName)
                                                  .arg(received * 100 / total));
                }
            });
    connect(m_ocrModelDownloader,
            &markshot::marketplace::PluginAssetDownloader::finished,
            this,
            [this](const QString &) {
                ++m_ocrModelQueueIndex;
                downloadNextOcrModel();
            });
    connect(m_ocrModelDownloader,
            &markshot::marketplace::PluginAssetDownloader::failed,
            this,
            [this, fileName](const QString &error) {
                m_ocrModelQueueIndex = -1;
                m_ocrModelStatus->setText(MS_TR("Failed to download %1: %2").arg(fileName, error));
                applyStatusTone(m_ocrModelStatus, QStringLiteral("error"));
                if (m_ocrModelDownloadButton) {
                    m_ocrModelDownloadButton->setEnabled(true);
                }
            });

    markshot::marketplace::PluginAssetDownloadRequest request;
    request.url = QUrl(QLatin1String(asset.url));
    request.sha256 = QLatin1String(asset.sha256);
    request.destinationPath = QDir(ocrModelsDirectory()).filePath(fileName);
    m_ocrModelDownloader->download(request);
}

}  // namespace markshot::settings
