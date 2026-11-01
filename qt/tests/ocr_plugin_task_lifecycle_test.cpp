#include "markshot/ocr_provider_plugin.h"
#include "providers/ocr/ocr_plugin_task.h"
#include "providers/ocr/ocr_task_executor.h"

#include <QImage>
#include <QSemaphore>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QThreadPool>
#include <QtTest>

#include <atomic>
#include <memory>
#include <mutex>
#include <vector>

using namespace markshot::providers;

namespace {

/** 【OCR测试】【阻塞插件】以真实任务入口模拟串行推理，计数进入插件前已经解码的图片。 */
class GatedPlugin final : public markshot::plugin::OcrProviderPlugin {
public:
    /** 【OCR测试】【任务清理】释放阻塞任务并等待测试工作完成，无参数和返回值。 */
    ~GatedPlugin() override
    {
        unblock();
        QThreadPool::globalInstance()->waitForDone();
        ocrPluginTaskThreadPool()->waitForDone();
    }

    /** 返回固定插件标识，无参数。 */
    QString providerId() const override { return QStringLiteral("gated-ocr"); }
    /** 返回测试插件名称，无参数。 */
    QString displayName() const override { return providerId(); }
    /** 清空 error 并返回可用状态。 */
    bool isAvailable(QString *error) const override
    {
        if (error) error->clear();
        return true;
    }

    /** 阻塞读取 image 后向 tokens 写入固定识别结果，error 未使用，返回图片是否有效。 */
    bool recognize(const QImage &image, QVector<markshot::plugin::OcrToken> *tokens, QString *) override
    {
        const int current = ++active;
        int previous = maximum.load();
        while (previous < current && !maximum.compare_exchange_weak(previous, current)) {}
        ++entered;
        std::lock_guard lock(modelMutex);
        gate.acquire();
        tokens->append({QStringLiteral("text"), QRectF(0, 0, image.width(), image.height()), 0, 0, 1.0});
        --active;
        return !image.isNull();
    }

    /** 放行当前及后续测试任务，无参数和返回值。 */
    void unblock() { gate.release(1000); }

    std::atomic<int> entered{0};
    std::atomic<int> active{0};
    std::atomic<int> maximum{0};

private:
    std::mutex modelMutex;
    QSemaphore gate;
};

/** 【OCR测试】【旧队列】限制旧全局池以确定性地制造排队，析构时恢复原并发配置。 */
class GlobalPoolLimit final {
public:
    /** 暂时把全局线程池设置为 threads 个线程。 */
    explicit GlobalPoolLimit(int threads)
        : previous(QThreadPool::globalInstance()->maxThreadCount())
    {
        QThreadPool::globalInstance()->setMaxThreadCount(threads);
    }
    /** 恢复测试前的全局线程数，无参数和返回值。 */
    ~GlobalPoolLimit() { QThreadPool::globalInstance()->setMaxThreadCount(previous); }
private:
    int previous;
};

}  // namespace

class OcrPluginTaskLifecycleTest final : public QObject {
    Q_OBJECT
private slots:
    /** 【OCR测试】【图片输入】准备本次测试专用图片，无参数和返回值。 */
    void initTestCase()
    {
        QVERIFY(directory.isValid());
        imagePath = directory.filePath(QStringLiteral("ocr.png"));
        const bool large = qEnvironmentVariableIsSet("MARK_SHOT_OCR_TASK_TEST_LARGE");
        QImage image(large ? QSize(3840, 2160) : QSize(64, 48), QImage::Format_RGB32);
        image.fill(Qt::white);
        QVERIFY(image.save(imagePath));
    }

    /** 【OCR测试】【图片准入】阻塞推理时只允许一张已解码图片进入插件，无参数和返回值。 */
    void serializesBeforeDecodedImagesAccumulate()
    {
        GlobalPoolLimit limit(4);
        GatedPlugin plugin;
        std::vector<std::unique_ptr<OcrPluginTask>> tasks;
        int finished = 0;
        for (int i = 0; i < 8; ++i) {
            auto task = std::make_unique<OcrPluginTask>(&plugin, imagePath);
            connect(task.get(), &ProviderTask::finished, this, [&](const TaskResult &result) {
                if (result.ok) ++finished;
            });
            task->start(0);
            tasks.push_back(std::move(task));
        }
        QTRY_VERIFY_WITH_TIMEOUT(plugin.entered.load() >= 1, 2000);
        QTest::qWait(100);
        QCOMPARE(plugin.maximum.load(), 1);
        plugin.unblock();
        QTRY_COMPARE_WITH_TIMEOUT(finished, 8, 5000);
        QCOMPARE(plugin.entered.load(), 8);
        QCOMPARE(plugin.maximum.load(), 1);
    }

    /** 【OCR测试】【排队取消】覆盖显式取消、对象销毁和超时，无参数和返回值。 */
    void skipsAbandonedQueuedWork_data()
    {
        QTest::addColumn<QString>("action");
        QTest::newRow("cancel") << QStringLiteral("cancel");
        QTest::newRow("destroy") << QStringLiteral("destroy");
        QTest::newRow("timeout") << QStringLiteral("timeout");
    }

    /** 【OCR测试】【排队取消】已放弃任务不得再次调用模型，且不能阻塞后续有效任务，无参数和返回值。 */
    void skipsAbandonedQueuedWork()
    {
        QFETCH(QString, action);
        GlobalPoolLimit limit(1);
        GatedPlugin plugin;
        OcrPluginTask first(&plugin, imagePath);
        first.start(0);
        QTRY_COMPARE_WITH_TIMEOUT(plugin.entered.load(), 1, 2000);
        auto abandoned = std::make_unique<OcrPluginTask>(&plugin, imagePath);
        QSignalSpy abandonedSignals(abandoned.get(), &ProviderTask::finished);
        abandoned->start(action == QStringLiteral("timeout") ? 25 : 0);
        if (action == QStringLiteral("cancel")) {
            abandoned->cancel();
        } else if (action == QStringLiteral("destroy")) {
            abandoned.reset();
        } else {
            QTRY_COMPARE_WITH_TIMEOUT(abandonedSignals.size(), 1, 1000);
            QCOMPARE(qvariant_cast<TaskResult>(abandonedSignals[0][0]).error, TaskError::Timeout);
        }
        OcrPluginTask last(&plugin, imagePath);
        QSignalSpy lastSignals(&last, &ProviderTask::finished);
        last.start(0);
        plugin.unblock();
        QTRY_COMPARE_WITH_TIMEOUT(lastSignals.size(), 1, 3000);
        QVERIFY(qvariant_cast<TaskResult>(lastSignals[0][0]).ok);
        QCOMPARE(plugin.entered.load(), 2);
        QCOMPARE(abandonedSignals.size(), action == QStringLiteral("timeout") ? 1 : 0);
    }

    /** 【OCR测试】【执行中取消】已开始的同步插件允许完成，但不能再向关闭窗口交付结果，无参数和返回值。 */
    void suppressesCancelledRunningResult()
    {
        GatedPlugin plugin;
        OcrPluginTask task(&plugin, imagePath);
        QSignalSpy completed(&task, &ProviderTask::finished);
        task.start(0);
        QTRY_COMPARE_WITH_TIMEOUT(plugin.entered.load(), 1, 2000);
        task.cancel();
        plugin.unblock();
        QTRY_COMPARE_WITH_TIMEOUT(plugin.active.load(), 0, 2000);
        QTest::qWait(30);
        QCOMPARE(completed.size(), 0);
    }

    /** 【OCR测试】【启动前取消】取消后的任务不能解码或调用插件，无参数和返回值。 */
    void ignoresStartAfterCancellation()
    {
        GatedPlugin plugin;
        OcrPluginTask task(&plugin, imagePath);
        QSignalSpy completed(&task, &ProviderTask::finished);
        task.cancel();
        task.start(10);
        plugin.unblock();
        QTest::qWait(50);
        QCOMPARE(plugin.entered.load(), 0);
        QCOMPARE(completed.size(), 0);
    }

    /** 【OCR测试】【重复启动】运行前后重复启动只允许一次推理和一次结果，无参数和返回值。 */
    void startsOnlyOnce()
    {
        GatedPlugin plugin;
        OcrPluginTask task(&plugin, imagePath);
        QSignalSpy completed(&task, &ProviderTask::finished);
        task.start(0);
        task.start(0);
        QTRY_COMPARE_WITH_TIMEOUT(plugin.entered.load(), 1, 2000);
        plugin.unblock();
        QTRY_COMPARE_WITH_TIMEOUT(completed.size(), 1, 2000);
        task.start(0);
        QTest::qWait(50);
        QCOMPARE(plugin.entered.load(), 1);
        QCOMPARE(completed.size(), 1);
    }

    /** 【OCR测试】【执行中失效】为超时和对象销毁准备数据，无参数和返回值。 */
    void abandonsRunningWork_data()
    {
        QTest::addColumn<bool>("destroy");
        QTest::newRow("destroy") << true;
        QTest::newRow("timeout") << false;
    }

    /** 【OCR测试】【执行中失效】活动任务失效不产生迟到结果，也不阻断后续有效任务，无参数和返回值。 */
    void abandonsRunningWork()
    {
        QFETCH(bool, destroy);
        GatedPlugin plugin;
        auto task = std::make_unique<OcrPluginTask>(&plugin, imagePath);
        QSignalSpy completed(task.get(), &ProviderTask::finished);
        task->start(destroy ? 0 : 200);
        QTRY_COMPARE_WITH_TIMEOUT(plugin.entered.load(), 1, 2000);
        if (destroy) {
            task.reset();
        } else {
            QTRY_COMPARE_WITH_TIMEOUT(completed.size(), 1, 2000);
            QCOMPARE(qvariant_cast<TaskResult>(completed[0][0]).error, TaskError::Timeout);
        }
        OcrPluginTask next(&plugin, imagePath);
        QSignalSpy nextCompleted(&next, &ProviderTask::finished);
        next.start(0);
        plugin.unblock();
        QTRY_COMPARE_WITH_TIMEOUT(nextCompleted.size(), 1, 2000);
        QVERIFY(qvariant_cast<TaskResult>(nextCompleted[0][0]).ok);
        QCOMPARE(plugin.entered.load(), 2);
        QCOMPARE(completed.size(), destroy ? 0 : 1);
    }

    /** 【OCR测试】【无效输入】缺失图像和插件必须各失败一次，不调用推理，无参数和返回值。 */
    void rejectsInvalidInput()
    {
        GatedPlugin plugin;
        OcrPluginTask missingImage(&plugin, directory.filePath(QStringLiteral("missing.png")));
        QSignalSpy failedImage(&missingImage, &ProviderTask::finished);
        missingImage.start(0);
        QTRY_COMPARE_WITH_TIMEOUT(failedImage.size(), 1, 2000);
        QCOMPARE(qvariant_cast<TaskResult>(failedImage[0][0]).error, TaskError::Failed);
        QCOMPARE(plugin.entered.load(), 0);
        OcrPluginTask missingPlugin(nullptr, imagePath);
        QSignalSpy failedPlugin(&missingPlugin, &ProviderTask::finished);
        missingPlugin.start(0);
        missingPlugin.start(0);
        QCOMPARE(failedPlugin.size(), 1);
        QCOMPARE(qvariant_cast<TaskResult>(failedPlugin[0][0]).error, TaskError::StartFailed);
    }

private:
    QTemporaryDir directory;
    QString imagePath;
};

QTEST_GUILESS_MAIN(OcrPluginTaskLifecycleTest)
#include "ocr_plugin_task_lifecycle_test.moc"
