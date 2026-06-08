#pragma once

#include "shot_window.h"
#include "scroll/scroll_session_window.h"

#include <QColor>
#include <QCursor>
#include <QImage>
#include <QJsonValue>
#include <QKeySequence>
#include <QPainterPath>
#include <QPoint>
#include <QProcessEnvironment>
#include <QRect>
#include <QRectF>
#include <QString>
#include <QStringList>
#include <QVector>

#include <array>
#include <optional>

class QPainter;
class QWheelEvent;
class QWidget;

namespace markshot::shot {

inline constexpr qreal kMinSelectionSize = 8.0;
inline constexpr qreal kToolbarMargin = 10.0;
inline constexpr qreal kMinStrokeWidth = 1.0;
inline constexpr qreal kMaxStrokeWidth = 24.0;
inline constexpr qreal kMinNumberWidth = 1.0;
inline constexpr qreal kMaxNumberWidth = 72.0;
inline constexpr qreal kMinMosaicBlockSize = 4.0;
inline constexpr qreal kMaxMosaicBlockSize = 48.0;
inline constexpr qreal kMinLaserWidth = 4.0;
inline constexpr qreal kMaxLaserWidth = 48.0;
inline constexpr qint64 kLaserLifetimeMs = 1800;
inline constexpr qreal kTextBackgroundPaddingX = 6.0;
inline constexpr qreal kTextBackgroundPaddingY = 4.0;
inline constexpr qreal kMinImageZoom = 0.25;
inline constexpr qreal kMaxImageZoom = 64.0;
inline constexpr qint64 kCtrlDoubleTapMs = 360;
inline constexpr int kPinnedOcrTimeoutMs = 30000;
inline constexpr int kPinnedTranslationTimeoutMs = 60000;
inline constexpr int kImageScrollBarExtent = 14;
inline constexpr int kImageWindowMinimumToolbarPadding = 24;
inline constexpr qsizetype kMaxSharpViewportPixels = 50'000'000;
inline constexpr double kSharpKernelRadius = 2.5;
inline constexpr int kMinSharpRowsPerThread = 64;
inline constexpr qreal kMinStartupColorLoupeSize = 72.0;
inline constexpr qreal kMaxStartupColorLoupeSize = 260.0;
inline constexpr qreal kDefaultMagnifierScale = 2.75;
inline constexpr qreal kMinMagnifierScale = 1.25;
inline constexpr qreal kMaxMagnifierScale = 6.0;
inline constexpr qreal kMinMagnifierDiameter = 72.0;
inline constexpr qreal kMaxMagnifierDiameter = 320.0;
inline constexpr qreal kMagnifierDragScale = 1.05;
inline constexpr qreal kMinMagnifierDragDistance = 64.0;

struct PinnedWindowConfig {
    bool alwaysOnTop = true;
    bool ocrEnabled = true;
    bool autoOcr = false;
    QString ocrBackend = QStringLiteral("auto");
    QString ocrCommand;
    int ocrTimeoutMs = kPinnedOcrTimeoutMs;
    QString translationCommand;
    QString translationTargetLanguage = QStringLiteral("Simplified Chinese");
    int translationTimeoutMs = kPinnedTranslationTimeoutMs;
    bool autoTranslateAfterOcr = false;
    bool borderEnabled = true;
    QColor borderColor = markshot::theme::kAccent;
    qreal borderWidth = 2.0;
};

qreal clampedMagnifierScale(qreal scale);
int magnifierScaleSliderValue(qreal scale);
qreal magnifierScaleFromSliderValue(int value);
QString magnifierScaleText(qreal scale);
/// @brief Creates the high-contrast cross cursor used during capture.
/// @return Cursor used for selection and annotation drawing modes.
QCursor captureCrossCursor();
qreal normalizedRotationDegrees(qreal degrees);
std::optional<bool> boolFromText(QString value);
std::optional<bool> boolFromConfigValue(const QJsonValue &value);
std::optional<bool> envFlagValue(const QProcessEnvironment &env, const QStringList &names);
bool sendDesktopNotification(const QString &summary, const QString &body, int timeoutMs = 2500);
QColor propertyIconInkForFill(const QColor &fillColor);
QString colorHexRgb(QColor color);
QString colorHexRgba(QColor color);
QString alphaText(QColor color);
QString normalizedColorChannel(int value);
int colorHueOrZero(int hue);
int rulerTickStep(qreal pixels);
void drawRoundedLabel(QPainter &painter,
                      QRectF rect,
                      const QString &text,
                      QColor fill = QColor(8, 13, 19, 225));
QRectF normalizedRect(QPointF a, QPointF b);
bool ellipseContainsPoint(QRectF ellipse, QPointF point, qreal tolerance);
QRect windowGeometryToImageRect(QRect windowGeometry, QRect sourceGeometry, QSize imageSize);
QPainterPath smoothedStrokePath(const QVector<QPointF> &points);
qreal imageNavigationWheelFactor(const QWheelEvent *event);
QImage renderSharpViewport(const QImage &source, const QRectF &sourceRect, const QSize &targetSize);
QString desktopEntryValue(const QStringList &lines, const QString &key);
bool desktopEntryBool(const QStringList &lines, const QString &key);
bool desktopEntrySupportsImage(const QStringList &lines);
QStringList desktopSearchDirs();
QString markShotPicturesDir();
QString markShotConfigDir();
QString extensionCommandsConfigPath();
QString appConfigPath();
QString markShotDataDir();
QString slurpGeometry(QRect geometry);
QString expandUserPath(const QString &path);
QString shellQuote(QString value);
bool extensionCommandUsesImagePlaceholder(const QString &command);
bool replaceExtensionImagePlaceholders(QString *command, const QString &imagePath);
bool replaceExtensionSlurpPlaceholder(QString *command, const QString &geometry);
void replaceShellPlaceholder(QString *command,
                             const QString &placeholder,
                             const QString &value,
                             bool *replaced = nullptr);
QStringList expandDesktopExec(const ShotWindow::DesktopApp &app, const QString &imagePath);
QString helperProgramPath(const QString &programName);
PinnedWindowConfig pinnedWindowConfig();
bool ocrOutputReportsMissingBackend(const QByteArray &stdoutData,
                                    const QByteArray &stderrData,
                                    const QString &configuredBackend);
bool ocrResultPanelEnabled();
bool annotationAutoSelectAfterDrawEnabled();
std::array<bool, static_cast<int>(ShotWindow::Tool::Laser) + 1> annotationAutoSelectAfterDrawTools();
markshot::scroll::ScrollSessionUiConfig scrollSessionUiConfig();
QWidget *createOcrResultWindow(QString text);
QWidget *createPinnedImageWindow(QImage image);

}  // namespace markshot::shot
