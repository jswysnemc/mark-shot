#include "providers/ocr/ocr_task_executor.h"

#include <QThreadPool>

namespace markshot::providers {

QThreadPool *ocrPluginTaskThreadPool()
{
    // 1. 【OCR】【任务准入】在图片解码前串行准入，不让多个完整像素缓冲等待插件内部锁
    static QThreadPool pool;
    static const bool configured = [] {
        pool.setMaxThreadCount(1);
        pool.setExpiryTimeout(1000);
        return true;
    }();
    Q_UNUSED(configured)
    return &pool;
}

}  // namespace markshot::providers
