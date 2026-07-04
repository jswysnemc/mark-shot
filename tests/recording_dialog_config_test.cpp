#include "recording/recording_dialog_config.h"

#include <QDir>
#include <QFileInfo>
#include <QtTest/QtTest>

namespace {

/**
 * 生成平台可用的临时目录文件路径。
 * @param fileName 文件名。
 * @return 临时目录下的完整文件路径。
 */
QString temporaryFilePath(const QString &fileName)
{
    return QDir::temp().filePath(fileName);
}

/**
 * 读取文件路径所在目录的绝对路径。
 * @param path 文件路径。
 * @return 文件所在目录的绝对路径。
 */
QString absoluteDirectoryPath(const QString &path)
{
    return QFileInfo(path).absolutePath();
}

}  // namespace

class RecordingDialogConfigTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证录制启动界面配置可以从 JSON 恢复。
     * @return 无返回值。
     */
    void readsDialogConfigFromRoot()
    {
        const QString inputPath =
            temporaryFilePath(QStringLiteral("mark-shot-test.mp4"));
        QJsonObject dialog;
        dialog.insert(QStringLiteral("mode"), QStringLiteral("video"));
        dialog.insert(QStringLiteral("scope"), QStringLiteral("display"));
        dialog.insert(QStringLiteral("backend"), QStringLiteral("wlroots-screencopy"));
        dialog.insert(QStringLiteral("fps"), 48);
        dialog.insert(QStringLiteral("includeAudio"), true);
        dialog.insert(QStringLiteral("outputPath"), inputPath);
        dialog.insert(QStringLiteral("displayKey"), QStringLiteral("screen:DP-1"));

        QJsonObject recording;
        recording.insert(QStringLiteral("dialog"), dialog);
        QJsonObject root;
        root.insert(QStringLiteral("recording"), recording);

        const markshot::recording::RecordingDialogConfig config =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Video);

        QCOMPARE(config.mode, markshot::recording::RecordingMode::Video);
        QCOMPARE(config.scope, markshot::recording::RecordingScope::Display);
        QCOMPARE(config.backend, markshot::recording::RecordingCaptureBackend::Wlroots);
        QCOMPARE(config.fps, 48);
        QCOMPARE(config.videoFps, 48);
        QCOMPARE(config.gifFps, 12);
        QVERIFY(config.includeAudio);
        QCOMPARE(absoluteDirectoryPath(config.outputPath), absoluteDirectoryPath(inputPath));
        QVERIFY(config.outputPath.endsWith(QStringLiteral(".mp4")));
        QVERIFY(config.outputPath != inputPath);
        QCOMPARE(config.displayKey, QStringLiteral("screen:DP-1"));
    }

    /**
     * 验证旧版完整输出路径会按路由模式迁移为目录并重新生成文件名。
     * @return 无返回值。
     */
    void migratesLegacyCompoundOutputPathToDirectory()
    {
        const QString inputPath =
            temporaryFilePath(QStringLiteral("mark-shot-recording-20260704-220015.mp4.gif"));
        QJsonObject dialog;
        dialog.insert(QStringLiteral("mode"), QStringLiteral("gif"));
        dialog.insert(QStringLiteral("outputPath"), inputPath);

        QJsonObject recording;
        recording.insert(QStringLiteral("dialog"), dialog);
        QJsonObject root;
        root.insert(QStringLiteral("recording"), recording);

        const markshot::recording::RecordingDialogConfig config =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Video);

        QCOMPARE(config.mode, markshot::recording::RecordingMode::Video);
        QCOMPARE(absoluteDirectoryPath(config.outputPath), absoluteDirectoryPath(inputPath));
        QVERIFY(config.outputPath.endsWith(QStringLiteral(".mp4")));
        QVERIFY(!config.outputPath.endsWith(QStringLiteral(".mp4.gif")));
        QVERIFY(config.outputPath != inputPath);
    }

    /**
     * 验证 GIF 和视频录制帧率从独立配置键读取。
     * @return 无返回值。
     */
    void readsSeparateFrameRatesForRecordingModes()
    {
        QJsonObject dialog;
        dialog.insert(QStringLiteral("fps"), 10);
        dialog.insert(QStringLiteral("mode"), QStringLiteral("gif"));
        dialog.insert(QStringLiteral("videoFps"), 60);
        dialog.insert(QStringLiteral("gifFps"), 12);

        QJsonObject recording;
        recording.insert(QStringLiteral("dialog"), dialog);
        QJsonObject root;
        root.insert(QStringLiteral("recording"), recording);

        const markshot::recording::RecordingDialogConfig videoConfig =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Video);
        const markshot::recording::RecordingDialogConfig gifConfig =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Gif);

        QCOMPARE(videoConfig.fps, 60);
        QCOMPARE(videoConfig.videoFps, 60);
        QCOMPARE(videoConfig.gifFps, 12);
        QCOMPARE(gifConfig.fps, 12);
        QCOMPARE(gifConfig.videoFps, 60);
        QCOMPARE(gifConfig.gifFps, 12);
    }

    /**
     * 验证旧版单帧率只迁移到旧模式对应的帧率状态。
     * @return 无返回值。
     */
    void migratesLegacyFrameRateByLegacyMode()
    {
        QJsonObject dialog;
        dialog.insert(QStringLiteral("mode"), QStringLiteral("gif"));
        dialog.insert(QStringLiteral("fps"), 24);

        QJsonObject recording;
        recording.insert(QStringLiteral("dialog"), dialog);
        QJsonObject root;
        root.insert(QStringLiteral("recording"), recording);

        const markshot::recording::RecordingDialogConfig videoConfig =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Video);
        const markshot::recording::RecordingDialogConfig gifConfig =
            markshot::recording::recordingDialogConfigFromRoot(
                root,
                markshot::recording::RecordingMode::Gif);

        QCOMPARE(videoConfig.fps, 30);
        QCOMPARE(videoConfig.videoFps, 30);
        QCOMPARE(videoConfig.gifFps, 24);
        QCOMPARE(gifConfig.fps, 24);
        QCOMPARE(gifConfig.videoFps, 30);
        QCOMPARE(gifConfig.gifFps, 24);
    }

    /**
     * 验证录制选项可以转为启动界面持久化配置。
     * @return 无返回值。
     */
    void createsDialogConfigFromOptions()
    {
        const QString inputPath = temporaryFilePath(QStringLiteral("sample.gif"));
        markshot::recording::RecordingOptions options;
        options.mode = markshot::recording::RecordingMode::Gif;
        options.scope = markshot::recording::RecordingScope::Region;
        options.captureBackend = markshot::recording::RecordingCaptureBackend::PipeWire;
        options.fps = 12;
        options.includeAudio = false;
        options.outputPath = inputPath;
        options.display.screenName = QStringLiteral("HDMI-A-1");

        const markshot::recording::RecordingDialogConfig config =
            markshot::recording::recordingDialogConfigFromOptions(options);

        QCOMPARE(config.mode, markshot::recording::RecordingMode::Gif);
        QCOMPARE(config.scope, markshot::recording::RecordingScope::Region);
        QCOMPARE(config.backend, markshot::recording::RecordingCaptureBackend::PipeWire);
        QCOMPARE(config.fps, 12);
        QCOMPARE(config.videoFps, 30);
        QCOMPARE(config.gifFps, 12);
        QVERIFY(!config.includeAudio);
        QCOMPARE(absoluteDirectoryPath(config.outputPath), absoluteDirectoryPath(inputPath));
        QVERIFY(config.outputPath.endsWith(QStringLiteral(".gif")));
        QVERIFY(config.outputPath != inputPath);
        QCOMPARE(config.displayKey, QStringLiteral("screen:HDMI-A-1"));
    }
};

QTEST_APPLESS_MAIN(RecordingDialogConfigTest)
#include "recording_dialog_config_test.moc"
