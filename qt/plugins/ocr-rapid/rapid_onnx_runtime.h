#pragma once

#include <QString>

#include <functional>
#include <memory>

struct OrtApi;

namespace markshot::ocr_rapid {

/** 【OCR】【运行库寿命】持有一次任务的运行库；原生对象不得逃逸出 run 回调。 */
class RapidOnnxRuntime final {
public:
    /** 创建未加载的运行库句柄，无参数。 */
    RapidOnnxRuntime();
    /** 释放本次加载引用；须先结束同步 run，无参数和返回值。 */
    ~RapidOnnxRuntime();
    RapidOnnxRuntime(const RapidOnnxRuntime &) = delete;
    RapidOnnxRuntime &operator=(const RapidOnnxRuntime &) = delete;

    /** 校验并加载兼容 C API，失败写入 error；不创建模型，返回是否成功。 */
    bool load(QString *error);
    /** 在短线程执行 work(api)，所有原生对象须在回调返回前析构；等待线程退出，异常写入 error，返回工作结果。 */
    bool run(const std::function<bool(const OrtApi &)> &work, QString *error);

private:
    class Impl;
    std::unique_ptr<Impl> m_impl;
};

}  // namespace markshot::ocr_rapid
