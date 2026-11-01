#pragma once

class QThreadPool;

namespace markshot::providers {

/**
 * 【OCR】【任务准入】取得单工作线程的插件识别池，排队任务仅保留路径和取消状态。
 * @return 进程内共享线程池，调用方不拥有它，也不得修改并发配置。
 */
QThreadPool *ocrPluginTaskThreadPool();

}  // namespace markshot::providers
