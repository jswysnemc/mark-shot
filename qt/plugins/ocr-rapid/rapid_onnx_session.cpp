#include "rapid_onnx_session.h"

#include "rapid_onnx_handle.h"

#include <QFile>
#include <QScopeGuard>

#include <algorithm>
#include <limits>
#include <string>
#include <thread>

namespace markshot::ocr_rapid {
namespace {

/** 【OCR】【名称所有权】仅借用默认 allocator；归还它分配的名称，不释放 allocator 本体。 */
struct AllocatedName {
    OrtAllocator *allocator = nullptr;
    char *value = nullptr;
    /** 在运行库有效期间归还名称，无参数和返回值。 */
    ~AllocatedName()
    {
        if (value) allocator->Free(allocator, value);
    }
};

}  // namespace

class RapidOnnxSession::Impl {
public:
    const OrtApi &api;
    RapidOnnxHandle<OrtEnv> env;
    RapidOnnxHandle<OrtSession> session;
    std::string inputName;
    std::string outputName;

    /** 【OCR】【原生所有权】将运行库 api 绑定到 Session 和 Env 的释放函数，逆序析构先释放 Session。 */
    explicit Impl(const OrtApi &api) : api(api), env(api.ReleaseEnv), session(api.ReleaseSession) {}

    /** 【OCR】【原生错误】消耗 status，失败时复制其消息到 error，返回调用是否成功。 */
    bool check(OrtStatus *status, QString *error) const
    {
        RapidOnnxHandle<OrtStatus> owner(api.ReleaseStatus);
        owner.reset(status);
        if (!status) return true;
        if (error) *error = QString::fromUtf8(api.GetErrorMessage(status));
        return false;
    }
};

RapidOnnxSession::RapidOnnxSession(const OrtApi &api) : m_impl(std::make_unique<Impl>(api)) {}
RapidOnnxSession::~RapidOnnxSession() = default;

bool RapidOnnxSession::load(const QString &modelPath, QString *error)
{
    if (error) error->clear();
    release();
    if (!QFile::exists(modelPath)) {
        if (error) *error = QStringLiteral("model file does not exist: %1").arg(modelPath);
        return false;
    }
    bool loaded = false;
    const auto cleanup = qScopeGuard([&] {
        if (!loaded) release();
    });
    const OrtApi &api = m_impl->api;

    // 1. 【OCR】【模型加载】所有原生句柄使用当前运行库的 C API，不缓存全局函数表
    if (!m_impl->check(api.CreateEnv(ORT_LOGGING_LEVEL_ERROR, "mark-shot-ocr-rapid", m_impl->env.put()), error)) {
        return false;
    }
    RapidOnnxHandle<OrtSessionOptions> options(api.ReleaseSessionOptions);
    if (!m_impl->check(api.CreateSessionOptions(options.put()), error)) return false;
    const unsigned int threads = std::max(1u, std::thread::hardware_concurrency() / 2);
    if (!m_impl->check(api.SetIntraOpNumThreads(options.get(), static_cast<int>(threads)), error)
        || !m_impl->check(api.SetSessionGraphOptimizationLevel(options.get(), ORT_ENABLE_ALL), error)
        || !m_impl->check(api.DisableCpuMemArena(options.get()), error)
        || !m_impl->check(api.DisableMemPattern(options.get()), error)) {
        return false;
    }
#ifdef Q_OS_WIN
    const std::wstring nativePath = modelPath.toStdWString();
    const auto path = nativePath.c_str();
#else
    const QByteArray nativePath = QFile::encodeName(modelPath);
    const auto path = nativePath.constData();
#endif
    if (!m_impl->check(api.CreateSession(m_impl->env.get(), path, options.get(), m_impl->session.put()), error)) {
        return false;
    }

    // 2. 【OCR】【模型名称】在 allocator 和运行库仍有效时复制名称，退出此作用域即归还原生字符串
    OrtAllocator *allocator = nullptr;
    if (!m_impl->check(api.GetAllocatorWithDefaultOptions(&allocator), error)) return false;
    AllocatedName input{allocator}, output{allocator};
    if (!m_impl->check(api.SessionGetInputName(m_impl->session.get(), 0, allocator, &input.value), error)
        || !m_impl->check(api.SessionGetOutputName(m_impl->session.get(), 0, allocator, &output.value), error)) {
        return false;
    }
    if (!input.value || !output.value) {
        if (error) *error = QStringLiteral("onnx model input or output name is missing");
        return false;
    }
    m_impl->inputName = input.value;
    m_impl->outputName = output.value;
    loaded = true;
    return true;
}

void RapidOnnxSession::release()
{
    m_impl->session.reset();
    m_impl->env.reset();
    m_impl->inputName.clear();
    m_impl->outputName.clear();
}

bool RapidOnnxSession::run(const QVector<float> &input, const TensorShape &inputShape,
                           QVector<float> *output, TensorShape *outputShape, QString *error)
{
    if (error) error->clear();
    if (!m_impl->session.get()) {
        if (error) *error = QStringLiteral("onnx session is not loaded");
        return false;
    }
    const OrtApi &api = m_impl->api;
    RapidOnnxHandle<OrtMemoryInfo> memoryInfo(api.ReleaseMemoryInfo);
    RapidOnnxHandle<OrtValue> inputTensor(api.ReleaseValue);
    RapidOnnxHandle<OrtValue> outputTensor(api.ReleaseValue);
    RapidOnnxHandle<OrtTensorTypeAndShapeInfo> info(api.ReleaseTensorTypeAndShapeInfo);
    // 1. 【OCR】【输入张量】借用现有浮点数组；原生 C API 的输入容量按字节而非元素计数
    if (static_cast<size_t>(input.size()) > std::numeric_limits<size_t>::max() / sizeof(float)) {
        if (error) *error = QStringLiteral("onnx input tensor is too large");
        return false;
    }
    if (!m_impl->check(api.CreateCpuMemoryInfo(OrtArenaAllocator, OrtMemTypeDefault, memoryInfo.put()), error)
        || !m_impl->check(api.CreateTensorWithDataAsOrtValue(
                              memoryInfo.get(), const_cast<float *>(input.constData()),
                              static_cast<size_t>(input.size()) * sizeof(float),
                              inputShape.dims.data(), inputShape.dims.size(),
                              ONNX_TENSOR_ELEMENT_DATA_TYPE_FLOAT, inputTensor.put()), error)) {
        return false;
    }
    const char *inputNames[] = {m_impl->inputName.c_str()};
    const char *outputNames[] = {m_impl->outputName.c_str()};
    const OrtValue *inputs[] = {inputTensor.get()};
    if (!m_impl->check(api.Run(m_impl->session.get(), nullptr, inputNames, inputs, 1,
                              outputNames, 1, outputTensor.put()), error)) {
        return false;
    }
    if (!outputTensor.get()) {
        if (error) *error = QStringLiteral("onnx inference returned no tensor");
        return false;
    }
    int isTensor = 0;
    if (!m_impl->check(api.IsTensor(outputTensor.get(), &isTensor), error)) return false;
    if (!isTensor) {
        if (error) *error = QStringLiteral("onnx inference returned no tensor");
        return false;
    }

    // 2. 【OCR】【输出张量】核对类型和容量后复制输出，原生数据不离开本次运行库作用域
    size_t rank = 0, count = 0;
    ONNXTensorElementDataType type = ONNX_TENSOR_ELEMENT_DATA_TYPE_UNDEFINED;
    if (!m_impl->check(api.GetTensorTypeAndShape(outputTensor.get(), info.put()), error)
        || !m_impl->check(api.GetDimensionsCount(info.get(), &rank), error)
        || !m_impl->check(api.GetTensorShapeElementCount(info.get(), &count), error)
        || !m_impl->check(api.GetTensorElementType(info.get(), &type), error)) {
        return false;
    }
    if (type != ONNX_TENSOR_ELEMENT_DATA_TYPE_FLOAT
        || count > static_cast<size_t>(std::numeric_limits<qsizetype>::max()) / sizeof(float)) {
        if (error) *error = QStringLiteral("unsupported onnx output tensor type or size");
        return false;
    }
    outputShape->dims.resize(rank);
    if (!m_impl->check(api.GetDimensions(info.get(), outputShape->dims.data(), rank), error)) return false;
    void *data = nullptr;
    if (!m_impl->check(api.GetTensorMutableData(outputTensor.get(), &data), error)) return false;
    output->resize(static_cast<qsizetype>(count));
    if (count) {
        if (!data) {
            if (error) *error = QStringLiteral("onnx output tensor data is missing");
            return false;
        }
        const auto *values = static_cast<const float *>(data);
        std::copy(values, values + count, output->data());
    }
    return true;
}

}  // namespace markshot::ocr_rapid
