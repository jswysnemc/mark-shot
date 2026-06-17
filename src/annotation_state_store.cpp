#include "annotation_state_store.h"

#include "window_detection.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QSaveFile>

#include <algorithm>

namespace {

constexpr const char *kFileName = "annotation-state.json";

constexpr const char *kKeyCurrentColor = "currentColor";
constexpr const char *kKeyStrokeWidth = "strokeWidth";
constexpr const char *kKeyHighlighterWidth = "highlighterWidth";
constexpr const char *kKeyTextSize = "textSize";
constexpr const char *kKeyPenWidth = "penWidth";
constexpr const char *kKeyShapeWidth = "shapeWidth";
constexpr const char *kKeyNumberWidth = "numberWidth";
constexpr const char *kKeyMosaicBlockSize = "mosaicBlockSize";
constexpr const char *kKeyShapeFilled = "shapeFilled";
constexpr const char *kKeyRectangleCornerRadius = "rectangleCornerRadius";
constexpr const char *kKeyRectangleStyle = "rectangleStyle";
constexpr const char *kKeyMagnifierScale = "magnifierScale";
constexpr const char *kKeyMagnifierShape = "magnifierShape";
constexpr const char *kKeyArrowStyle = "arrowStyle";
constexpr const char *kKeyHighlighterStyle = "highlighterStyle";
constexpr const char *kKeyNumberStyle = "numberStyle";
constexpr const char *kKeyTextFontFamily = "textFontFamily";
constexpr const char *kKeyTextBackgroundColor = "textBackgroundColor";

/// @brief 读取 JSON 中颜色字符串为 QColor
/// @param value JSON 颜色字段
/// @param fallback 解析失败时的回退值
/// @return 解析后的颜色或 fallback
QColor colorFromJson(const QJsonValue &value, const QColor &fallback)
{
    if (!value.isString()) {
        return fallback;
    }
    QColor color(value.toString());
    return color.isValid() ? color : fallback;
}

/// @brief 把 QColor 序列化为 #AARRGGBB 字符串以保留 alpha
/// @param color 待序列化颜色
/// @return 形如 "#AARRGGBB" 的字符串
QString colorToJson(const QColor &color)
{
    return color.name(QColor::HexArgb);
}

/// @brief 读取 JSON 中数字字段并夹紧到合法范围
/// @param value JSON 数字字段
/// @param fallback 缺失或非数字时的回退值
/// @param minValue 允许的最小值
/// @param maxValue 允许的最大值
/// @return 夹紧后的数值
qreal clampedDouble(const QJsonValue &value, qreal fallback, qreal minValue, qreal maxValue)
{
    if (!value.isDouble()) {
        return std::clamp(fallback, minValue, maxValue);
    }
    return std::clamp<qreal>(value.toDouble(fallback), minValue, maxValue);
}

template <typename Enum>
Enum enumFromInt(const QJsonValue &value, Enum fallback, int minValue, int maxValue)
{
    if (!value.isDouble()) {
        return fallback;
    }
    const int raw = value.toInt(static_cast<int>(fallback));
    if (raw < minValue || raw > maxValue) {
        return fallback;
    }
    return static_cast<Enum>(raw);
}

}  // namespace

namespace markshot {

QString annotationStateFilePath()
{
    return QDir(markShotConfigDir()).filePath(QString::fromLatin1(kFileName));
}

AnnotationState loadAnnotationState()
{
    AnnotationState state;

    QFile file(annotationStateFilePath());
    if (!file.exists()) {
        return state;
    }
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return state;
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        return state;
    }

    const QJsonObject root = document.object();

    // 1. 颜色字段
    state.currentColor = colorFromJson(root.value(QString::fromLatin1(kKeyCurrentColor)), state.currentColor);
    state.textBackgroundColor =
        colorFromJson(root.value(QString::fromLatin1(kKeyTextBackgroundColor)), state.textBackgroundColor);

    // 2. 笔宽与尺寸字段(范围与 shot_window_internal.h 中常量保持一致)
    const qreal legacyPenWidth =
        clampedDouble(root.value(QString::fromLatin1(kKeyPenWidth)), 2.0, 1.0, 24.0);
    const qreal legacyShapeWidth =
        clampedDouble(root.value(QString::fromLatin1(kKeyShapeWidth)), 3.0, 1.0, 24.0);
    state.strokeWidth =
        clampedDouble(root.value(QString::fromLatin1(kKeyStrokeWidth)), legacyShapeWidth, 1.0, 24.0);
    state.highlighterWidth =
        clampedDouble(root.value(QString::fromLatin1(kKeyHighlighterWidth)), legacyPenWidth * 2.0, 1.0, 48.0);
    state.numberWidth = clampedDouble(root.value(QString::fromLatin1(kKeyNumberWidth)), state.numberWidth, 1.0, 72.0);
    state.textSize = clampedDouble(root.value(QString::fromLatin1(kKeyTextSize)), state.textSize, 1.0, 1000.0);
    state.mosaicBlockSize =
        clampedDouble(root.value(QString::fromLatin1(kKeyMosaicBlockSize)), state.mosaicBlockSize, 4.0, 48.0);

    // 3. 矩形相关
    const QJsonValue filledValue = root.value(QString::fromLatin1(kKeyShapeFilled));
    if (filledValue.isBool()) {
        state.shapeFilled = filledValue.toBool();
    }
    state.rectangleCornerRadius = clampedDouble(
        root.value(QString::fromLatin1(kKeyRectangleCornerRadius)), state.rectangleCornerRadius, 0.0, 96.0);
    state.rectangleStyle = enumFromInt<ShotWindow::RectangleStyle>(
        root.value(QString::fromLatin1(kKeyRectangleStyle)),
        state.rectangleStyle,
        static_cast<int>(ShotWindow::RectangleStyle::Stroke),
        static_cast<int>(ShotWindow::RectangleStyle::Invert));

    // 4. 放大镜相关
    state.magnifierScale =
        clampedDouble(root.value(QString::fromLatin1(kKeyMagnifierScale)), state.magnifierScale, 1.25, 6.0);
    state.magnifierShape = enumFromInt<ShotWindow::MagnifierShape>(
        root.value(QString::fromLatin1(kKeyMagnifierShape)),
        state.magnifierShape,
        static_cast<int>(ShotWindow::MagnifierShape::Circle),
        static_cast<int>(ShotWindow::MagnifierShape::Rectangle));

    // 5. 各风格枚举
    state.arrowStyle = enumFromInt<ShotWindow::ArrowStyle>(
        root.value(QString::fromLatin1(kKeyArrowStyle)),
        state.arrowStyle,
        static_cast<int>(ShotWindow::ArrowStyle::Fletched),
        static_cast<int>(ShotWindow::ArrowStyle::BidirectionalKde));
    state.highlighterStyle = enumFromInt<ShotWindow::HighlighterStyle>(
        root.value(QString::fromLatin1(kKeyHighlighterStyle)),
        state.highlighterStyle,
        static_cast<int>(ShotWindow::HighlighterStyle::Freehand),
        static_cast<int>(ShotWindow::HighlighterStyle::StraightLine));
    state.numberStyle = enumFromInt<ShotWindow::NumberStyle>(
        root.value(QString::fromLatin1(kKeyNumberStyle)),
        state.numberStyle,
        static_cast<int>(ShotWindow::NumberStyle::Arabic),
        static_cast<int>(ShotWindow::NumberStyle::Chinese));

    // 6. 文本字体
    const QJsonValue fontValue = root.value(QString::fromLatin1(kKeyTextFontFamily));
    if (fontValue.isString()) {
        state.textFontFamily = fontValue.toString();
    }

    return state;
}

bool saveAnnotationState(const AnnotationState &state)
{
    const QString path = annotationStateFilePath();
    if (path.isEmpty()) {
        return false;
    }

    QDir dir(QFileInfo(path).absolutePath());
    if (!dir.exists() && !dir.mkpath(QStringLiteral("."))) {
        return false;
    }

    // 1. 组装 JSON 根对象
    QJsonObject root;
    root.insert(QString::fromLatin1(kKeyCurrentColor), colorToJson(state.currentColor));
    root.insert(QString::fromLatin1(kKeyStrokeWidth), state.strokeWidth);
    root.insert(QString::fromLatin1(kKeyHighlighterWidth), state.highlighterWidth);
    root.insert(QString::fromLatin1(kKeyNumberWidth), state.numberWidth);
    root.insert(QString::fromLatin1(kKeyTextSize), state.textSize);
    root.insert(QString::fromLatin1(kKeyMosaicBlockSize), state.mosaicBlockSize);
    root.insert(QString::fromLatin1(kKeyShapeFilled), state.shapeFilled);
    root.insert(QString::fromLatin1(kKeyRectangleCornerRadius), state.rectangleCornerRadius);
    root.insert(QString::fromLatin1(kKeyRectangleStyle), static_cast<int>(state.rectangleStyle));
    root.insert(QString::fromLatin1(kKeyMagnifierScale), state.magnifierScale);
    root.insert(QString::fromLatin1(kKeyMagnifierShape), static_cast<int>(state.magnifierShape));
    root.insert(QString::fromLatin1(kKeyArrowStyle), static_cast<int>(state.arrowStyle));
    root.insert(QString::fromLatin1(kKeyHighlighterStyle), static_cast<int>(state.highlighterStyle));
    root.insert(QString::fromLatin1(kKeyNumberStyle), static_cast<int>(state.numberStyle));
    root.insert(QString::fromLatin1(kKeyTextFontFamily), state.textFontFamily);
    root.insert(QString::fromLatin1(kKeyTextBackgroundColor), colorToJson(state.textBackgroundColor));

    // 2. 通过 QSaveFile 原子写入,避免崩溃时残留半截 JSON
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return false;
    }
    file.write(QJsonDocument(root).toJson(QJsonDocument::Indented));
    file.write("\n");
    return file.commit();
}

}  // namespace markshot
