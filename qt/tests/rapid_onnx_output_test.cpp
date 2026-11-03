#include "rapid_model_paths.h"
#include "rapid_onnx_runtime.h"
#include "rapid_onnx_session.h"

#include <onnxruntime_c_api.h>

#include <QScopeGuard>
#include <QtTest>

#include <stdexcept>

using namespace markshot::ocr_rapid;

namespace {
struct OutputTrace {
    const OrtApi *api = nullptr;
    OrtValue *tensor = nullptr;
    void *data = nullptr;
    int releases = 0;
};
thread_local OutputTrace *trace = nullptr;

/** 【OCR测试】【原生地址】读取 value 原生输出到 data 并记录借用地址，返回原生状态。 */
OrtStatus *ORT_API_CALL readTensorData(OrtValue *value, void **data) noexcept
{
    OrtStatus *status = trace->api->GetTensorMutableData(value, data);
    if (!status) {
        trace->tensor = value;
        trace->data = *data;
    }
    return status;
}

/** 【OCR测试】【输出释放】记录 value 对应输出的释放时机，并调用原生释放，无返回值。 */
void ORT_API_CALL releaseValue(OrtValue *value) noexcept
{
    if (value == trace->tensor) {
        ++trace->releases;
        trace->tensor = nullptr;
        trace->data = nullptr;
    }
    trace->api->ReleaseValue(value);
}
}  // namespace

class RapidOnnxOutputTest final : public QObject {
    Q_OBJECT
private slots:
    /** 【OCR测试】【回调路径】分别覆盖正常、拒绝和异常回调，无参数和返回值。 */
    void borrowsUntilCallbackEnds_data()
    {
        QTest::addColumn<int>("mode");
        QTest::newRow("success") << 0;
        QTest::newRow("reject") << 1;
        QTest::newRow("exception") << 2;
    }

    /** 【OCR测试】【张量寿命】以真实运行库核对地址、释放顺序及同一会话恢复，无参数和返回值。 */
    void borrowsUntilCallbackEnds()
    {
        QFETCH(int, mode);
        const auto models = locateRapidModels();
        if (!models.isComplete()) {
            if (qEnvironmentVariableIsSet("MARK_SHOT_TEST_REQUIRE_OCR_MODELS")) QFAIL("Required models are missing");
            QSKIP("PP-OCR models are not available on this machine");
        }
        RapidOnnxRuntime runtime;
        QString error, failure;
        QVERIFY2(runtime.load(&error), qPrintable(error));
        bool borrowed = false, released = false, recovered = false, outcome = false;
        const bool ran = runtime.run([&](const OrtApi &api) {
            OutputTrace state{&api};
            trace = &state;
            const auto restore = qScopeGuard([] { trace = nullptr; });
            OrtApi observingApi = api;
            observingApi.GetTensorMutableData = readTensorData;
            observingApi.ReleaseValue = releaseValue;
            RapidOnnxSession session(observingApi);
            if (!session.load(models.recModel, &failure)) return false;
            const QVector<float> input(3 * 48 * 32, 0.0f);
            try {
                const bool accepted = session.visitOutput(input, {{1, 3, 48, 32}},
                    [&](const float *data, qsizetype count, const TensorShape &shape, QString *) {
                        borrowed = data == state.data && state.tensor && state.releases == 0
                            && count > 0 && shape.dims.size() == 3;
                        if (mode == 2) throw std::runtime_error("test consumer");
                        return mode == 0;
                    }, &failure);
                outcome = mode != 2 && accepted == (mode == 0);
            } catch (const std::runtime_error &) {
                outcome = mode == 2;
            }
            released = state.releases == 1 && !state.tensor && !state.data;
            recovered = session.visitOutput(input, {{1, 3, 48, 32}},
                [](const float *, qsizetype count, const TensorShape &, QString *) { return count > 0; }, &failure);
            return true;
        }, &error);
        QVERIFY2(ran, qPrintable(error + failure));
        QVERIFY(borrowed);
        QVERIFY(released);
        QVERIFY(recovered);
        QVERIFY(outcome);
    }
};

QTEST_GUILESS_MAIN(RapidOnnxOutputTest)
#include "rapid_onnx_output_test.moc"
