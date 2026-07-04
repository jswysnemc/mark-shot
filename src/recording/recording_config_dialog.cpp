#include "recording/recording_config_dialog.h"

#include "recording/recording_dialog_config.h"
#include "recording/recording_display_source.h"
#include "recording/recording_file_naming.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QCheckBox>
#include <QComboBox>
#include <QCursor>
#include <QDialogButtonBox>
#include <QFileDialog>
#include <QFormLayout>
#include <QFont>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QScreen>
#include <QVBoxLayout>

namespace markshot::recording {
namespace {

/**
 * 返回录制模式标题。
 * @param mode 录制模式。
 * @return 标题文本。
 */
QString titleForMode(RecordingMode mode)
{
    return mode == RecordingMode::Gif ? MS_TR("GIF Recording") : MS_TR("Video Recording");
}

/**
 * 返回当前显示器。
 * @return 当前显示器，无法判断时返回主显示器。
 */
QScreen *currentScreen()
{
    QScreen *screen = QGuiApplication::screenAt(QCursor::pos());
    return screen ? screen : QGuiApplication::primaryScreen();
}

/**
 * 查找当前显示器在来源列表中的下标。
 * @param sources 显示器来源列表。
 * @return 当前显示器来源下标。
 */
int currentDisplaySourceIndex(const QVector<DisplaySource> &sources)
{
    QScreen *screen = currentScreen();
    if (!screen) {
        return sources.isEmpty() ? -1 : 0;
    }

    for (int i = 0; i < sources.size(); ++i) {
        const DisplaySource &source = sources.at(i);
        if (!source.allOutputs && source.screenName == screen->name()) {
            return i;
        }
    }
    for (int i = 0; i < sources.size(); ++i) {
        const DisplaySource &source = sources.at(i);
        if (!source.allOutputs && source.geometry == screen->geometry()) {
            return i;
        }
    }
    return sources.isEmpty() ? -1 : 0;
}

/**
 * 按持久化键查找显示器来源下标。
 * @param sources 显示器来源列表。
 * @param key 持久化键。
 * @return 匹配下标，找不到时返回 -1。
 */
int displaySourceIndexForKey(const QVector<DisplaySource> &sources, const QString &key)
{
    if (key.trimmed().isEmpty()) {
        return -1;
    }
    for (int i = 0; i < sources.size(); ++i) {
        if (recordingDisplayPersistenceKey(sources.at(i)) == key) {
            return i;
        }
    }
    return -1;
}

/**
 * 给帧率下拉框写入阶梯选项。
 * @param combo 帧率下拉框。
 * @param mode 录制模式。
 * @return 无返回值。
 */
void populateFrameRateOptions(QComboBox *combo, RecordingMode mode, int requestedFps = -1)
{
    if (!combo) {
        return;
    }
    combo->clear();
    const QVector<int> values = mode == RecordingMode::Gif
        ? QVector<int>{6, 8, 10, 12, 15, 20, 24, 30}
        : QVector<int>{15, 24, 30, 48, 60};
    const int fallback = mode == RecordingMode::Gif ? 12 : 30;
    for (int fps : values) {
        combo->addItem(MS_TR("%1 fps").arg(fps), fps);
    }
    const int requestedIndex = combo->findData(requestedFps);
    const int fallbackIndex = combo->findData(fallback);
    combo->setCurrentIndex(requestedIndex >= 0 ? requestedIndex : (fallbackIndex >= 0 ? fallbackIndex : 0));
}

/**
 * 填充采集后端下拉框。
 * @param combo 采集后端下拉框。
 * @param requested 请求后端。
 * @return 无返回值。
 */
void populateBackendOptions(QComboBox *combo, RecordingCaptureBackend requested)
{
    if (!combo) {
        return;
    }
    combo->clear();
    combo->addItem(QStringLiteral("Auto"), static_cast<int>(RecordingCaptureBackend::Auto));
    combo->addItem(QStringLiteral("wlroots screencopy"), static_cast<int>(RecordingCaptureBackend::Wlroots));
    combo->addItem(QStringLiteral("PipeWire"), static_cast<int>(RecordingCaptureBackend::PipeWire));
    combo->addItem(QStringLiteral("Windows Graphics Capture"), static_cast<int>(RecordingCaptureBackend::WindowsWgc));
    combo->addItem(QStringLiteral("Polling"), static_cast<int>(RecordingCaptureBackend::Polling));
    const int index = combo->findData(static_cast<int>(requested));
    combo->setCurrentIndex(index >= 0 ? index : 0);
}

/**
 * 从下拉框数据读取录制模式。
 * @param combo 录制模式下拉框。
 * @param fallback 默认录制模式。
 * @return 录制模式。
 */
RecordingMode modeFromCombo(const QComboBox *combo, RecordingMode fallback)
{
    bool ok = false;
    const int value = combo ? combo->currentData().toInt(&ok) : 0;
    if (!ok) {
        return fallback;
    }
    return value == static_cast<int>(RecordingMode::Video)
        ? RecordingMode::Video
        : RecordingMode::Gif;
}

/**
 * 从下拉框数据读取采集后端。
 * @param combo 采集后端下拉框。
 * @return 采集后端。
 */
RecordingCaptureBackend backendFromCombo(const QComboBox *combo)
{
    bool ok = false;
    const int value = combo ? combo->currentData().toInt(&ok) : 0;
    if (!ok) {
        return RecordingCaptureBackend::Auto;
    }
    switch (static_cast<RecordingCaptureBackend>(value)) {
    case RecordingCaptureBackend::Wlroots:
    case RecordingCaptureBackend::PipeWire:
    case RecordingCaptureBackend::WindowsWgc:
    case RecordingCaptureBackend::Polling:
        return static_cast<RecordingCaptureBackend>(value);
    case RecordingCaptureBackend::Auto:
        break;
    }
    return RecordingCaptureBackend::Auto;
}

}  // namespace

RecordingConfigDialog::RecordingConfigDialog(RecordingMode mode, QWidget *parent)
    : QDialog(parent)
    , m_mode(mode)
    , m_sources(availableDisplaySources())
{
    const RecordingDialogConfig persisted = configuredRecordingDialogConfig(m_mode);
    m_videoFps = persisted.videoFps;
    m_gifFps = persisted.gifFps;
    setWindowTitle(titleForMode(m_mode));
    setModal(true);
    setMinimumWidth(460);

    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(18, 18, 18, 18);
    root->setSpacing(14);

    m_title = new QLabel(titleForMode(m_mode), this);
    m_title->setFont(markshot::theme::uiFont(16, QFont::DemiBold));
    root->addWidget(m_title);

    auto *form = new QFormLayout;
    form->setHorizontalSpacing(16);
    form->setVerticalSpacing(10);
    root->addLayout(form);

    m_modeSelector = new QComboBox(this);
    m_modeSelector->addItem(QStringLiteral("GIF"), static_cast<int>(RecordingMode::Gif));
    m_modeSelector->addItem(MS_TR("Video"), static_cast<int>(RecordingMode::Video));
    const int modeIndex = m_modeSelector->findData(static_cast<int>(m_mode));
    m_modeSelector->setCurrentIndex(modeIndex >= 0 ? modeIndex : 0);
    form->addRow(MS_TR("Recording Type"), m_modeSelector);

    m_fps = new QComboBox(this);
    populateFrameRateOptions(m_fps, m_mode, fpsForMode(m_mode));
    form->addRow(MS_TR("Frame Rate"), m_fps);

    m_audio = new QCheckBox(MS_TR("Record system default audio input"), this);
    m_audio->setChecked(persisted.includeAudio);
    form->addRow(MS_TR("Audio"), m_audio);

    m_display = new QComboBox(this);
    for (int i = 0; i < m_sources.size(); ++i) {
        const DisplaySource &source = m_sources.at(i);
        const QString subtitle = QStringLiteral("%1 x %2").arg(source.geometry.width()).arg(source.geometry.height());
        m_display->addItem(QStringLiteral("%1  %2").arg(source.title, subtitle), i);
    }
    const int savedSourceIndex = displaySourceIndexForKey(m_sources, persisted.displayKey);
    const int currentSourceIndex = savedSourceIndex >= 0 ? savedSourceIndex : currentDisplaySourceIndex(m_sources);
    if (currentSourceIndex >= 0) {
        const int comboIndex = m_display->findData(currentSourceIndex);
        m_display->setCurrentIndex(comboIndex >= 0 ? comboIndex : 0);
    }
    form->addRow(MS_TR("Display"), m_display);

    m_backend = new QComboBox(this);
    populateBackendOptions(m_backend, persisted.backend);
    form->addRow(MS_TR("Recording Backend"), m_backend);

    m_scope = new QComboBox(this);
    m_scope->addItem(MS_TR("Record selected display"), static_cast<int>(RecordingScope::Display));
    m_scope->addItem(MS_TR("Select region after this dialog"), static_cast<int>(RecordingScope::Region));
    const int scopeIndex = m_scope->findData(static_cast<int>(persisted.scope));
    m_scope->setCurrentIndex(scopeIndex >= 0 ? scopeIndex : 0);
    form->addRow(MS_TR("Capture Area"), m_scope);

    m_outputPath = new QLineEdit(persisted.outputPath.isEmpty()
                                     ? defaultRecordingPath(m_mode)
                                     : normalizedRecordingPath(persisted.outputPath, m_mode),
                                 this);
    m_outputPathTouched = false;
    auto *outputRow = new QWidget(this);
    auto *outputLayout = new QHBoxLayout(outputRow);
    outputLayout->setContentsMargins(0, 0, 0, 0);
    outputLayout->setSpacing(8);
    auto *outputBrowse = new QPushButton(MS_TR("Browse"), outputRow);
    outputLayout->addWidget(m_outputPath, 1);
    outputLayout->addWidget(outputBrowse);
    form->addRow(MS_TR("Output"), outputRow);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Cancel | QDialogButtonBox::Ok, this);
    buttons->button(QDialogButtonBox::Ok)->setText(MS_TR("Start"));
    buttons->button(QDialogButtonBox::Cancel)->setText(MS_TR("Cancel"));
    root->addWidget(buttons);

    connect(outputBrowse, &QPushButton::clicked, this, [this] { browseOutputPath(); });
    connect(m_modeSelector, &QComboBox::currentIndexChanged, this, [this] {
        const RecordingMode nextMode = modeFromCombo(m_modeSelector, m_mode);
        if (nextMode == m_mode) {
            return;
        }
        storeCurrentFpsForMode(m_mode);
        const bool preserveOutput = m_outputPathTouched
            && m_outputPath
            && !m_outputPath->text().trimmed().isEmpty();
        m_mode = nextMode;
        setWindowTitle(titleForMode(m_mode));
        if (m_title) {
            m_title->setText(titleForMode(m_mode));
        }
        populateFrameRateOptions(m_fps, m_mode, fpsForMode(m_mode));
        updateAudioControls();
        if (m_outputPath) {
            m_outputPath->setText(preserveOutput
                                      ? normalizedRecordingPath(m_outputPath->text(), m_mode)
                                      : defaultRecordingPath(m_mode));
        }
    });
    connect(m_outputPath, &QLineEdit::textEdited, this, [this] {
        m_outputPathTouched = true;
    });
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);

    updateAudioControls();
}

RecordingOptions RecordingConfigDialog::options() const
{
    RecordingOptions result;
    result.mode = m_mode;
    bool fpsOk = false;
    const int fallbackFps = m_mode == RecordingMode::Gif ? 12 : 30;
    const int selectedFps = m_fps ? m_fps->currentData().toInt(&fpsOk) : fallbackFps;
    result.fps = fpsOk ? selectedFps : fallbackFps;
    result.includeAudio = m_audio && m_audio->isEnabled() && m_audio->isChecked();
    result.captureBackend = backendFromCombo(m_backend);
    result.scope = static_cast<RecordingScope>(m_scope ? m_scope->currentData().toInt() : static_cast<int>(RecordingScope::Region));
    result.outputPath = normalizedRecordingPath(m_outputPath ? m_outputPath->text() : QString(), m_mode);

    const int sourceIndex = m_display ? m_display->currentData().toInt() : -1;
    if (sourceIndex >= 0 && sourceIndex < m_sources.size()) {
        result.display = m_sources.at(sourceIndex);
    }
    if (result.scope == RecordingScope::Display) {
        result.captureGeometry = result.display.geometry;
    }
    return result;
}

void RecordingConfigDialog::browseOutputPath()
{
    const QString filter = m_mode == RecordingMode::Gif
        ? MS_TR("GIF Images (*.gif)")
        : MS_TR("MP4 Videos (*.mp4)");
    const QString path = QFileDialog::getSaveFileName(this,
                                                      MS_TR("Save Recording"),
                                                      m_outputPath ? m_outputPath->text() : defaultRecordingPath(m_mode),
                                                      filter);
    if (!path.isEmpty() && m_outputPath) {
        m_outputPathTouched = true;
        m_outputPath->setText(normalizedRecordingPath(path, m_mode));
    }
}

void RecordingConfigDialog::updateAudioControls()
{
    if (!m_audio) {
        return;
    }
    const bool videoMode = m_mode == RecordingMode::Video;
    m_audio->setEnabled(videoMode);
    if (!videoMode) {
        m_audio->setChecked(false);
        m_audio->setToolTip(MS_TR("GIF recording does not include audio."));
    } else {
        m_audio->setToolTip(QString());
    }
}

/**
 * 读取指定录制模式的帧率状态。
 * @param mode 录制模式。
 * @return 帧率。
 */
int RecordingConfigDialog::fpsForMode(RecordingMode mode) const
{
    return mode == RecordingMode::Gif ? m_gifFps : m_videoFps;
}

/**
 * 保存当前帧率下拉框状态到指定录制模式。
 * @param mode 录制模式。
 * @return 无返回值。
 */
void RecordingConfigDialog::storeCurrentFpsForMode(RecordingMode mode)
{
    if (!m_fps) {
        return;
    }
    bool ok = false;
    const int value = m_fps->currentData().toInt(&ok);
    if (!ok) {
        return;
    }
    if (mode == RecordingMode::Gif) {
        m_gifFps = value;
        return;
    }
    m_videoFps = value;
}

}  // namespace markshot::recording
