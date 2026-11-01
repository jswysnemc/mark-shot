#include "rapid_onnx_runtime.h"

#include "rapid_runtime_paths.h"

#include <onnxruntime_c_api.h>

#include <QLibrary>
#include <QScopeGuard>

#include <exception>
#include <thread>

namespace markshot::ocr_rapid {

class RapidOnnxRuntime::Impl {
public:
    std::unique_ptr<QLibrary> library;
    const OrtApi *api = nullptr;

    /** 【OCR】【运行库释放】调用方已结束工作线程及原生对象，归还本句柄的加载引用，无参数和返回值。 */
    ~Impl()
    {
        api = nullptr;
        if (library) library->unload();
    }
};

RapidOnnxRuntime::RapidOnnxRuntime() : m_impl(std::make_unique<Impl>()) {}
RapidOnnxRuntime::~RapidOnnxRuntime() = default;

bool RapidOnnxRuntime::load(QString *error)
{
    if (error) error->clear();
    if (m_impl->api) return true;
    QStringList failures;
    for (const QString &path : rapidOnnxRuntimeCandidates()) {
        auto library = std::make_unique<QLibrary>(path);
        const auto unloadUnused = qScopeGuard([&] {
            if (library) library->unload();
        });
        library->setLoadHints(QLibrary::ResolveAllSymbolsHint);
        if (!library->load()) {
            failures.append(library->errorString());
            continue;
        }
        // 1. 【OCR】【接口校验】旧同名库可能没有导出或返回空表，不在版本校验前调用推理接口
        using ApiBaseEntry = const OrtApiBase *(ORT_API_CALL *)();
        const auto entry = reinterpret_cast<ApiBaseEntry>(library->resolve("OrtGetApiBase"));
        const OrtApiBase *base = entry ? entry() : nullptr;
        const OrtApi *api = base && base->GetApi ? base->GetApi(ORT_API_VERSION) : nullptr;
        if (api) {
            m_impl->api = api;
            m_impl->library = std::move(library);
            return true;
        }
        failures.append(QStringLiteral("%1 does not provide ONNX Runtime API %2")
                            .arg(path).arg(ORT_API_VERSION));
    }
    if (error) *error = QStringLiteral("ONNX Runtime is unavailable: %1").arg(failures.join(QStringLiteral("; ")));
    return false;
}

bool RapidOnnxRuntime::run(const std::function<bool(const OrtApi &)> &work, QString *error)
{
    if (error) error->clear();
    if (!m_impl->api || !work) {
        if (error) *error = QStringLiteral("ONNX Runtime is not loaded");
        return false;
    }
    bool result = false;
    QString failure;
    try {
        // 1. 【OCR】【线程寿命】线程局部状态须随线程退出释放，不能交给长期存活的 Qt 工作线程
        std::thread worker([&] {
            try {
                result = work(*m_impl->api);
            } catch (const std::exception &exception) {
                failure = QString::fromUtf8(exception.what());
                if (failure.isEmpty()) failure = QStringLiteral("inference exception");
            } catch (...) {
                failure = QStringLiteral("unknown inference exception");
            }
        });
        worker.join();
    } catch (const std::exception &exception) {
        failure = QString::fromUtf8(exception.what());
    }
    // 2. 【OCR】【异常边界】只返回自有文字，不把异常对象或原生句柄带到运行库卸载之后
    if (!failure.isEmpty()) {
        if (error) *error = QStringLiteral("OCR execution failed: %1").arg(failure);
        return false;
    }
    return result;
}

}  // namespace markshot::ocr_rapid
