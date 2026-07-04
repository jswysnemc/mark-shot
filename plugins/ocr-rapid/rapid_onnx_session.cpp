#include "rapid_onnx_session.h"

#include <onnxruntime_cxx_api.h>

#include <QFile>

#include <algorithm>
#include <thread>

namespace markshot::ocr_rapid {

class RapidOnnxSession::Impl {
public:
    Ort::Env env{ORT_LOGGING_LEVEL_ERROR, "mark-shot-ocr-rapid"};
    std::unique_ptr<Ort::Session> session;
    std::string inputName;
    std::string outputName;
};

RapidOnnxSession::RapidOnnxSession()
    : m_impl(std::make_unique<Impl>())
{
}

RapidOnnxSession::~RapidOnnxSession() = default;

bool RapidOnnxSession::load(const QString &modelPath, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!QFile::exists(modelPath)) {
        if (error) {
            *error = QStringLiteral("model file does not exist: %1").arg(modelPath);
        }
        return false;
    }

    try {
        Ort::SessionOptions options;
        // 1. 推理线程数取物理核数一半，避免抢占 UI 与编码线程
        const unsigned int threads = std::max(1u, std::thread::hardware_concurrency() / 2);
        options.SetIntraOpNumThreads(static_cast<int>(threads));
        options.SetGraphOptimizationLevel(GraphOptimizationLevel::ORT_ENABLE_ALL);

        const QByteArray pathBytes = QFile::encodeName(modelPath);
        m_impl->session = std::make_unique<Ort::Session>(m_impl->env, pathBytes.constData(), options);

        // 2. 缓存输入输出名，PP-OCR 模型均为单输入单输出
        Ort::AllocatorWithDefaultOptions allocator;
        m_impl->inputName = m_impl->session->GetInputNameAllocated(0, allocator).get();
        m_impl->outputName = m_impl->session->GetOutputNameAllocated(0, allocator).get();
        return true;
    } catch (const Ort::Exception &exception) {
        if (error) {
            *error = QStringLiteral("failed to load onnx model: %1")
                         .arg(QString::fromUtf8(exception.what()));
        }
        m_impl->session.reset();
        return false;
    }
}

bool RapidOnnxSession::run(const QVector<float> &input,
                           const TensorShape &inputShape,
                           QVector<float> *output,
                           TensorShape *outputShape,
                           QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_impl->session) {
        if (error) {
            *error = QStringLiteral("onnx session is not loaded");
        }
        return false;
    }

    try {
        const Ort::MemoryInfo memoryInfo =
            Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);
        Ort::Value inputTensor = Ort::Value::CreateTensor<float>(
            memoryInfo,
            const_cast<float *>(input.constData()),
            static_cast<size_t>(input.size()),
            inputShape.dims.data(),
            inputShape.dims.size());

        const char *inputNames[] = {m_impl->inputName.c_str()};
        const char *outputNames[] = {m_impl->outputName.c_str()};
        auto outputs = m_impl->session->Run(Ort::RunOptions{nullptr},
                                            inputNames,
                                            &inputTensor,
                                            1,
                                            outputNames,
                                            1);
        if (outputs.empty() || !outputs.front().IsTensor()) {
            if (error) {
                *error = QStringLiteral("onnx inference returned no tensor");
            }
            return false;
        }

        const auto info = outputs.front().GetTensorTypeAndShapeInfo();
        const auto shape = info.GetShape();
        outputShape->dims.clear();
        qint64 count = 1;
        for (const int64_t dim : shape) {
            outputShape->dims.push_back(dim);
            count *= dim;
        }
        const float *data = outputs.front().GetTensorData<float>();
        output->resize(count);
        std::copy(data, data + count, output->data());
        return true;
    } catch (const Ort::Exception &exception) {
        if (error) {
            *error = QStringLiteral("onnx inference failed: %1")
                         .arg(QString::fromUtf8(exception.what()));
        }
        return false;
    }
}

}  // namespace markshot::ocr_rapid
