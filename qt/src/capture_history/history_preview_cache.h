#pragma once

#include "capture_history/history_store.h"

#include <QCache>
#include <QSet>

namespace markshot::history {

/** 【截图历史】【预览缓存】仅缓存显示尺寸图像，容量按 KiB 计，不缓存操作所需的原图。 */
class HistoryPreviewCache final {
public:
    /** 创建最大保留 budgetKiB 的缓存，默认 8MiB。 */
    explicit HistoryPreviewCache(int budgetKiB = 8192);
    /** 返回 store 中 fileName 的文件版本；缺失、符号链接或非法路径返回空。 */
    static QString revision(const HistoryStore &store, const QString &fileName);
    /** 返回 fileName 在 size 内的预览；从 store 读取，文件版本变化时自动失效。 */
    QImage image(const HistoryStore &store, const QString &fileName, QSize size);
    /** 清除不属于 names 的历史预览，无返回值。 */
    void retain(const QSet<QString> &names);
    /** 返回缓存持有图像的总 KiB，不包含调用方继续持有的共享引用，无参数。 */
    int costKiB() const { return m_images.totalCost(); }

private:
    struct Entry {
        QString fileName;
        QString revision;
        QImage image;
    };
    QCache<QString, Entry> m_images;
};

}  // namespace markshot::history
