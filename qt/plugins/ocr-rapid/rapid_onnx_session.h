#pragma once

#include <QString>
#include <QVector>

#include <cstdint>
#include <memory>
#include <vector>

struct OrtApi;

namespace markshot::ocr_rapid {

struct TensorShape {
    std::vector<std::int64_t> dims;
};

/**
 * ONNX Runtime 会话薄封装。
 *
 * 持有单模型 Session 与输入输出名，提供单输入单输出的浮点推理。
 */
class RapidOnnxSession final {
public:
    /** 【OCR】【会话寿命】借用 api 创建会话；会话必须在其运行库释放前销毁。 */
    explicit RapidOnnxSession(const OrtApi &api);
    ~RapidOnnxSession();

    /**
     * 加载 onnx 模型。
     * @param modelPath 模型文件路径。
     * @param error 输出错误信息。
     * @return 加载成功时返回 true。
     */
    bool load(const QString &modelPath, QString *error);

    /**
     * 释放会话与运行时环境，把模型权重交还分配器。
     * @return 无返回值。
     */
    void release();

    /**
     * 执行单输入单输出推理。
     * @param input NCHW 浮点输入数据。
     * @param inputShape 输入形状。
     * @param output 输出浮点数据。
     * @param outputShape 输出形状。
     * @param error 输出错误信息。
     * @return 推理成功时返回 true。
     */
    bool run(const QVector<float> &input,
             const TensorShape &inputShape,
             QVector<float> *output,
             TensorShape *outputShape,
             QString *error);

private:
    class Impl;
    std::unique_ptr<Impl> m_impl;
};

}  // namespace markshot::ocr_rapid
