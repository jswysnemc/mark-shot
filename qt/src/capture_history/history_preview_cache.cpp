#include "capture_history/history_preview_cache.h"

#include <QDir>
#include <QFileInfo>

#include <algorithm>

namespace markshot::history {

HistoryPreviewCache::HistoryPreviewCache(int budgetKiB) : m_images(std::max(0, budgetKiB)) {}

QString HistoryPreviewCache::revision(const HistoryStore &store, const QString &fileName)
{
    const QFileInfo file(QDir(store.directory()).filePath(fileName));
    if (fileName.isEmpty() || file.fileName() != fileName || !file.isFile() || file.isSymLink()) return {};
    return file.absoluteFilePath() + QLatin1Char('|') + QString::number(file.size())
        + QLatin1Char('|') + QString::number(file.lastModified().toMSecsSinceEpoch())
        + QLatin1Char('|') + QString::number(file.metadataChangeTime().toMSecsSinceEpoch());
}

QImage HistoryPreviewCache::image(const HistoryStore &store, const QString &fileName, QSize size)
{
    if (size.isEmpty()) return {};
    const QString version = revision(store, fileName);
    if (version.isEmpty()) return {};
    const QString key = fileName + QStringLiteral("|%1x%2").arg(size.width()).arg(size.height());
    if (const Entry *cached = m_images.object(key); cached && cached->revision == version) {
        return cached->image;
    }
    // 1. 【截图历史】【预览缓存】原图读取始终经过存储层的文件校验，预览不继承截图的显示倍率
    QImage preview = store.image(fileName, size);
    preview.setDevicePixelRatio(1.0);
    m_images.remove(key);
    const qsizetype cost = preview.sizeInBytes() / 1024 + (preview.sizeInBytes() % 1024 != 0);
    if (!preview.isNull() && cost <= m_images.maxCost()) {
        m_images.insert(key, new Entry{fileName, version, preview}, static_cast<int>(cost));
    }
    return preview;
}

void HistoryPreviewCache::retain(const QSet<QString> &names)
{
    for (const QString &key : m_images.keys()) {
        if (!names.contains(m_images.object(key)->fileName)) m_images.remove(key);
    }
}

}  // namespace markshot::history
