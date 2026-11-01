#include "rapid_onnx_runtime.h"
#include "rapid_runtime_paths.h"

#include <onnxruntime_c_api.h>

#include <QFile>
#include <QScopeGuard>
#include <QTemporaryDir>
#include <QThread>
#include <QtTest>

#include <atomic>
#include <stdexcept>
#include <thread>

using namespace markshot::ocr_rapid;

class RapidOnnxRuntimeTest final : public QObject {
    Q_OBJECT
private:
    QTemporaryDir directory;
    QByteArray oldRuntime;
    QByteArray oldTrace;

    /** 【OCR测试】【释放记录】读取本次原生线程退出记录，返回文件字节，无参数。 */
    QByteArray trace() const
    {
        QFile file(directory.filePath(QStringLiteral("thread.log")));
        if (!file.open(QIODevice::ReadOnly)) return {};
        return file.readAll();
    }

private slots:
    /** 【OCR测试】【隔离配置】保存环境并选用测试运行库，无参数和返回值。 */
    void init()
    {
        QVERIFY(directory.isValid());
        oldRuntime = qgetenv("MARK_SHOT_RAPID_ONNX_RUNTIME");
        oldTrace = qgetenv("MARK_SHOT_TEST_ORT_THREAD_TRACE");
        qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", MARK_SHOT_TEST_ORT_LIBRARY);
        qputenv("MARK_SHOT_TEST_ORT_THREAD_TRACE", directory.filePath(QStringLiteral("thread.log")).toUtf8());
        QFile::remove(directory.filePath(QStringLiteral("thread.log")));
    }

    /** 【OCR测试】【恢复配置】恢复原环境变量，无参数和返回值。 */
    void cleanup()
    {
        for (const auto &item : {qMakePair("MARK_SHOT_RAPID_ONNX_RUNTIME", oldRuntime),
                                 qMakePair("MARK_SHOT_TEST_ORT_THREAD_TRACE", oldTrace)}) {
            if (item.second.isNull()) qunsetenv(item.first);
            else qputenv(item.first, item.second);
        }
    }

    /** 【OCR测试】【显式路径】严格指定的缺失运行库不能静默回退；随后可在同对象中恢复，无参数和返回值。 */
    void recoversFromMissingRuntime()
    {
        const QString missing = directory.filePath(QStringLiteral("missing-runtime"));
        qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", missing.toUtf8());
        QCOMPARE(rapidOnnxRuntimeCandidates(), QStringList{missing});
        RapidOnnxRuntime runtime;
        QString error;
        QVERIFY(!runtime.load(&error));
        QVERIFY(!error.isEmpty());
        bool called = false;
        QVERIFY(!runtime.run([&](const OrtApi &) { called = true; return true; }, &error));
        QVERIFY(!called);
        qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", MARK_SHOT_TEST_ORT_LIBRARY);
        QVERIFY2(runtime.load(&error), qPrintable(error));
        QVERIFY(error.isEmpty());
        QVERIFY(runtime.run([](const OrtApi &api) { return QByteArray(api.GetBuildInfoString()) == "mark-shot-lifetime-test"; }, &error));
        QCOMPARE(trace(), QByteArray("thread-ended\n"));
    }

    /** 【OCR测试】【旧版接口】空 API 基表必须报告错误而不是解引用，无参数和返回值。 */
    void rejectsIncompatibleRuntime()
    {
        qputenv("MARK_SHOT_RAPID_ONNX_RUNTIME", MARK_SHOT_TEST_ORT_INCOMPATIBLE);
        RapidOnnxRuntime runtime;
        QString error;
        QVERIFY(!runtime.load(&error));
        QVERIFY(error.contains(QStringLiteral("does not provide ONNX Runtime API")));
        QVERIFY(trace().isEmpty());
    }

    /** 【OCR测试】【线程归还】每次回调在线程内执行，返回前原生线程状态已经销毁，无参数和返回值。 */
    void joinsNativeThreadBeforeReturning()
    {
        const auto caller = QThread::currentThreadId();
        {
            RapidOnnxRuntime runtime;
            QString error;
            QVERIFY2(runtime.load(&error), qPrintable(error));
            for (int i = 1; i <= 3; ++i) {
                bool otherThread = false;
                QVERIFY(runtime.run([&](const OrtApi &api) {
                    otherThread = QThread::currentThreadId() != caller;
                    return QByteArray(api.GetBuildInfoString()) == "mark-shot-lifetime-test";
                }, &error));
                QVERIFY(otherThread);
                QCOMPARE(trace().count("thread-ended\n"), i);
            }
        }
#ifdef Q_OS_LINUX
        QFile maps(QStringLiteral("/proc/self/maps"));
        QVERIFY(maps.open(QIODevice::ReadOnly));
        QVERIFY(!maps.readAll().contains(QByteArray(MARK_SHOT_TEST_ORT_LIBRARY)));
#endif
    }

    /** 【OCR测试】【异常恢复】回调异常转换为自有错误文字，并归还线程状态，后续调用仍有效，无参数和返回值。 */
    void catchesWorkerExceptions()
    {
        RapidOnnxRuntime runtime;
        QString error;
        QVERIFY2(runtime.load(&error), qPrintable(error));
        QVERIFY(!runtime.run([](const OrtApi &api) -> bool {
            api.GetBuildInfoString();
            throw std::runtime_error("recognition failed");
        }, &error));
        QVERIFY(error.contains(QStringLiteral("recognition failed")));
        QCOMPARE(trace().count("thread-ended\n"), 1);
        QVERIFY(!runtime.run([](const OrtApi &) -> bool { throw 7; }, &error));
        QVERIFY(error.contains(QStringLiteral("unknown inference exception")));
        QVERIFY(runtime.run([](const OrtApi &api) { return QByteArray(api.GetBuildInfoString()) == "mark-shot-lifetime-test"; }, &error));
        QVERIFY(error.isEmpty());
        QCOMPARE(trace().count("thread-ended\n"), 2);
    }

    /** 【OCR测试】【重叠句柄】两个运行库租约重叠时，各自线程和回调保持有效，无参数和返回值。 */
    void supportsOverlappingLibraryHandles()
    {
        QString error;
        RapidOnnxRuntime first, second;
        QVERIFY2(first.load(&error), qPrintable(error));
        QVERIFY2(second.load(&error), qPrintable(error));
        std::atomic<int> arrived{0};
        bool firstResult = false, secondResult = false;
        const auto work = [&](const OrtApi &api) {
            const bool ok = QByteArray(api.GetBuildInfoString()) == "mark-shot-lifetime-test";
            arrived.fetch_add(1);
            while (arrived.load() < 2) std::this_thread::yield();
            return ok;
        };
        std::thread caller([&] { firstResult = first.run(work, nullptr); });
        secondResult = second.run(work, nullptr);
        caller.join();
        QVERIFY(firstResult && secondResult);
        QCOMPARE(trace().count("thread-ended\n"), 2);
    }
};

QTEST_GUILESS_MAIN(RapidOnnxRuntimeTest)
#include "rapid_onnx_runtime_test.moc"
