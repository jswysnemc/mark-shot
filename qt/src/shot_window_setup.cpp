#include "shot_window_module.h"

#include "app_config_store.h"
#include "selection_loupe_config.h"
#include "ui/canvas_scrollbar_style.h"
#include "ui/compact_toolbar.h"
#include "ui/toolbar_panel_style.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

namespace {

using ToolbarAction = ShotWindow::Action;

/// @brief 获取工具类动作对应的工具，用于读取工具快捷键
/// @param action 工具栏动作
/// @return 工具类动作返回对应工具；其他动作返回空
std::optional<ShotWindow::Tool> toolForToolbarAction(ToolbarAction action)
{
    switch (action) {
    case ToolbarAction::ToolMove: return ShotWindow::Tool::Move;
    case ToolbarAction::ToolSelect: return ShotWindow::Tool::Select;
    case ToolbarAction::ToolPen: return ShotWindow::Tool::Pen;
    case ToolbarAction::ToolLine: return ShotWindow::Tool::Line;
    case ToolbarAction::ToolHighlighter: return ShotWindow::Tool::Highlighter;
    case ToolbarAction::ToolRectangle: return ShotWindow::Tool::Rectangle;
    case ToolbarAction::ToolEllipse: return ShotWindow::Tool::Ellipse;
    case ToolbarAction::ToolMarker: return ShotWindow::Tool::Marker;
    case ToolbarAction::ToolArrow: return ShotWindow::Tool::Arrow;
    case ToolbarAction::ToolText: return ShotWindow::Tool::Text;
    case ToolbarAction::ToolNumber: return ShotWindow::Tool::Number;
    case ToolbarAction::ToolMosaic: return ShotWindow::Tool::Mosaic;
    case ToolbarAction::ToolMagnifier: return ShotWindow::Tool::Magnifier;
    case ToolbarAction::ToolLaser: return ShotWindow::Tool::Laser;
    case ToolbarAction::ToolRuler: return ShotWindow::Tool::Ruler;
    default: return std::nullopt;
    }
}

/// @brief 获取动作未配置快捷键时在提示中显示的回退文本
/// @param action 工具栏动作
/// @return 回退文本；无回退时返回空字符串
QString toolbarShortcutFallback(ToolbarAction action)
{
    switch (action) {
    case ToolbarAction::Clear: return QStringLiteral("Clear");
    case ToolbarAction::Redo: return QStringLiteral("Ctrl+Shift+Z");
    case ToolbarAction::OpenWith: return QStringLiteral("Open");
    case ToolbarAction::Extensions: return QStringLiteral("Ext");
    case ToolbarAction::ScrollCapture: return QStringLiteral("Scroll");
    case ToolbarAction::OcrCopy: return QStringLiteral("OCR");
    case ToolbarAction::Translate: return QStringLiteral("Ctrl+T");
    case ToolbarAction::Save: return QStringLiteral("Save As");
    case ToolbarAction::Upload: return QStringLiteral("Upload");
    case ToolbarAction::Settings: return QStringLiteral("Settings");
    case ToolbarAction::ToggleToolbarLayout: return QStringLiteral("Layout");
    default: return {};
    }
}

// 主工具栏常驻分组：选择、绘制、形状、标记、历史
constexpr ToolbarAction kMainSelectGroup[] = {ToolbarAction::ToolMove, ToolbarAction::ToolSelect};
constexpr ToolbarAction kMainDrawGroup[] = {ToolbarAction::ToolPen, ToolbarAction::ToolLine,
                                            ToolbarAction::ToolHighlighter};
constexpr ToolbarAction kMainShapeGroup[] = {ToolbarAction::ToolRectangle, ToolbarAction::ToolEllipse,
                                             ToolbarAction::ToolMarker, ToolbarAction::ToolArrow};
constexpr ToolbarAction kMainMarkupGroup[] = {ToolbarAction::ToolText, ToolbarAction::ToolNumber,
                                              ToolbarAction::ToolMosaic, ToolbarAction::ToolMagnifier,
                                              ToolbarAction::ToolRuler, ToolbarAction::ToolLaser};
constexpr ToolbarAction kMainHistoryGroup[] = {ToolbarAction::Clear, ToolbarAction::Undo, ToolbarAction::Redo};
// 全屏标注时主工具栏追加的分组：视图、输出、会话
constexpr ToolbarAction kFullscreenViewGroup[] = {ToolbarAction::ToggleCaptureScope,
                                                  ToolbarAction::ToggleToolbarLayout};
constexpr ToolbarAction kFullscreenOutputGroup[] = {ToolbarAction::OpenWith, ToolbarAction::Extensions,
                                                    ToolbarAction::ScrollCapture, ToolbarAction::Pin,
                                                    ToolbarAction::OcrCopy, ToolbarAction::Translate,
                                                    ToolbarAction::Copy,
                                                    ToolbarAction::Save, ToolbarAction::Upload};
// 设置与关闭各自成组，组件在两者之间绘制分隔线
constexpr ToolbarAction kSessionSettingsGroup[] = {ToolbarAction::Settings};
constexpr ToolbarAction kSessionCloseGroup[] = {ToolbarAction::Cancel};
// 选区副工具栏分组：视图、扩展、输出、会话
constexpr ToolbarAction kActionScopeGroup[] = {ToolbarAction::ToggleCaptureScope};
constexpr ToolbarAction kActionExtendGroup[] = {ToolbarAction::OpenWith, ToolbarAction::Extensions,
                                                ToolbarAction::ScrollCapture, ToolbarAction::Pin,
                                                ToolbarAction::OcrCopy, ToolbarAction::Translate};
constexpr ToolbarAction kActionOutputGroup[] = {ToolbarAction::Copy, ToolbarAction::Save, ToolbarAction::Upload};

}


std::optional<ShotWindow::Tool> ShotWindow::toolFromName(QString name)
{
    QString key = name.trimmed().toLower();
    key.replace(QLatin1Char('_'), QLatin1Char('-'));

    if (key == QStringLiteral("move")) {
        return Tool::Move;
    }
    if (key == QStringLiteral("select") || key == QStringLiteral("selection") || key == QStringLiteral("cursor")) {
        return Tool::Select;
    }
    if (key == QStringLiteral("pen")) {
        return Tool::Pen;
    }
    if (key == QStringLiteral("line")) {
        return Tool::Line;
    }
    if (key == QStringLiteral("highlighter") || key == QStringLiteral("highlight")) {
        return Tool::Highlighter;
    }
    if (key == QStringLiteral("rectangle") || key == QStringLiteral("rect")) {
        return Tool::Rectangle;
    }
    if (key == QStringLiteral("ellipse") || key == QStringLiteral("oval") || key == QStringLiteral("circle")) {
        return Tool::Ellipse;
    }
    if (key == QStringLiteral("arrow")) {
        return Tool::Arrow;
    }
    if (key == QStringLiteral("text")) {
        return Tool::Text;
    }
    if (key == QStringLiteral("number") || key == QStringLiteral("counter")) {
        return Tool::Number;
    }
    if (key == QStringLiteral("magnifier") || key == QStringLiteral("magnify")
        || key == QStringLiteral("loupe") || key == QStringLiteral("zoom")) {
        return Tool::Magnifier;
    }
    if (key == QStringLiteral("mosaic") || key == QStringLiteral("blur")) {
        return Tool::Mosaic;
    }
    if (key == QStringLiteral("laser")) {
        return Tool::Laser;
    }
    if (key == QStringLiteral("marker") || key == QStringLiteral("shape")
        || key == QStringLiteral("stamp") || key == QStringLiteral("shape-marker")) {
        return Tool::Marker;
    }
    if (key == QStringLiteral("ruler") || key == QStringLiteral("measure")) {
        return Tool::Ruler;
    }
    return std::nullopt;
}

QStringList ShotWindow::supportedToolNames()
{
    return {
        QStringLiteral("move"),
        QStringLiteral("select"),
        QStringLiteral("pen"),
        QStringLiteral("line"),
        QStringLiteral("highlighter"),
        QStringLiteral("rectangle"),
        QStringLiteral("ellipse"),
        QStringLiteral("arrow"),
        QStringLiteral("text"),
        QStringLiteral("number"),
        QStringLiteral("magnifier"),
        QStringLiteral("mosaic"),
        QStringLiteral("laser"),
        QStringLiteral("marker"),
        QStringLiteral("ruler"),
    };
}

ShotWindow::ShotWindow(QImage frozenFrame,
                       QString outputName,
                       QRect sourceGeometry,
                       QVector<markshot::WindowInfo> windowInfos,
                       bool windowDetectionEnabled,
                       QWidget *parent)
    : QWidget(parent)
    , m_frozenFrame(std::move(frozenFrame))
    , m_outputName(std::move(outputName))
    , m_sourceGeometry(sourceGeometry)
{
    const shortcuts::ShortcutConfig shortcutConfig = shortcuts::configuredShortcuts(appConfigPath());
    m_actionShortcuts = shortcutConfig.actions;
    m_toolShortcuts = shortcutConfig.tools;
    m_startupColorPickerShortcut = shortcutConfig.startupColorPicker;
    m_startupRulerShortcut = shortcutConfig.startupRuler;
    m_startupCodeScannerShortcut = shortcutConfig.startupCodeScanner;
    m_startupDisplayCaptureShortcut = shortcutConfig.startupDisplayCapture;
    m_startupGifRecorderShortcut = shortcutConfig.startupGifRecorder;
    m_startupVideoRecorderShortcut = shortcutConfig.startupVideoRecorder;
    m_autoSelectAfterDrawByTool = annotationAutoSelectAfterDrawTools();
    bool appConfigOk = false;
    const QJsonObject appConfigRoot = markshot::readAppConfigRoot(&appConfigOk);
    if (appConfigOk) {
        m_toolbarAppearance = markshot::toolbarAppearanceFromConfigRoot(appConfigRoot);
        m_doubleClickAction = markshot::captureDoubleClickActionFromConfigRoot(appConfigRoot);
        m_controls.m_toolbarPosition = markshot::captureToolbarPositionFromConfigRoot(appConfigRoot);
        m_selectionLoupeEnabled = markshot::selectionLoupeEnabledFromConfigRoot(appConfigRoot);
    }
    // 【截图工具栏】【主题】按设置中的界面主题解析工具栏配色，跟随系统时读取当前系统明暗
    m_toolbarAppearance.palette = markshot::ui::toolbarPalette(markshot::ui::effectiveUiThemeMode(
        appConfigOk ? markshot::ui::uiThemeModeFromConfigRoot(appConfigRoot) : markshot::ui::UiThemeMode::System));
    const QColor panelInk = m_toolbarAppearance.palette.text;
    // 在初始化 UI 之前先加载上次会话的工具默认值,使后续 toolbar/属性面板按
    // 持久化的状态显示;loadAnnotationStateFromDisk 仅修改 m_* 默认值字段,
    // 不会触碰任何尚未构造完成的控件
    loadAnnotationStateFromDisk();

    // 1. 【标注核心】【状态保存】合并高频改动，减少 Rust 原子保存期间的主线程等待
    m_annotationStateTimer = new QTimer(this);
    m_annotationStateTimer->setSingleShot(true);
    m_annotationStateTimer->setInterval(250);
    connect(m_annotationStateTimer, &QTimer::timeout, this, [this] { flushAnnotationStateNow(); });

    // A full-screen selection overlay is expensive to rasterize.  Pointer
    // events may arrive at 200 Hz while the display only needs a bounded UI
    // cadence, so batch initial-selection damage into at most 120 repaints/s.
    m_initialSelectionRepaintTimer = new QTimer(this);
    m_initialSelectionRepaintTimer->setSingleShot(true);
    m_initialSelectionRepaintTimer->setInterval(8);
    connect(m_initialSelectionRepaintTimer, &QTimer::timeout, this, [this] {
        const QRegion pending = std::exchange(m_pendingInitialSelectionRepaint, QRegion());
        if (!pending.isEmpty()) {
            update(pending);
        }
    });

    // 选中标注的滚轮改粗细需要进入撤销历史,但连续滚轮只应对应一次撤销
    m_annotationWidthWheelHistoryTimer = new QTimer(this);
    m_annotationWidthWheelHistoryTimer->setSingleShot(true);
    m_annotationWidthWheelHistoryTimer->setInterval(350);
    connect(m_annotationWidthWheelHistoryTimer, &QTimer::timeout, this, [this] {
        commitAnnotationWidthWheelHistory();
    });

    setWindowTitle(MS_TR("Mark Shot"));
    setCursor(captureCrossCursor(devicePixelRatioF()));
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    markshot::windows::setExcludedFromCapture(this);
    if (m_frozenFrame.format() != QImage::Format_ARGB32_Premultiplied
        && m_frozenFrame.format() != QImage::Format_RGB32) {
        // 1. 【画布】【图像内存】RGB32 无需预乘，可与贴图共享；其他格式优先就地转换
        m_frozenFrame = std::move(m_frozenFrame).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    }
    // Annotation and selection geometry are stored in image pixels. Clear any
    // high-DPI metadata from screen grabs so Qt painting does not apply a second
    // device-pixel-ratio scale on top of our explicit image-to-widget mapping.
    m_frozenFrame.setDevicePixelRatio(1.0);

    initializeToolbar();
    initializeImageScrollBars();
    initializeActionToolbar();
    restoreConfiguredToolbarLayout();
    initializeShortcuts();
    initializeActiveRecordingOverlay();

    m_controls.m_annotationPropertyPanel = new QWidget(this);
    m_controls.m_annotationPropertyPanel->setObjectName(QStringLiteral("annotationPropertyPanel"));
    m_controls.m_annotationPropertyPanel->setCursor(Qt::ArrowCursor);
    m_controls.m_annotationPropertyPanel->setStyleSheet(m_controls.m_toolbar->styleSheet());
    const qreal propertyScale = std::max<qreal>(1.0, m_toolbarAppearance.fontSize / 11.0);
    const int propertyMarginX = std::max(8, qRound(8 * propertyScale));
    const int propertyMarginY = std::max(6, qRound(6 * propertyScale));
    const int propertySpacing = std::max(6, qRound(6 * propertyScale));
    const int propertyMinorSpacing = std::max(2, qRound(2 * propertyScale));
    const int propertyGlyphSize =
        std::max(18, std::min(m_toolbarAppearance.toolbarButtonSize - 8, m_toolbarAppearance.fontSize + 8));
    const int propertyButtonIconSize =
        std::max(20, std::min(m_toolbarAppearance.toolbarButtonSize - 8, m_toolbarAppearance.toolbarIconSize));
    auto scaledWidth = [propertyScale](int width) {
        return std::max(width, qRound(width * propertyScale));
    };
    auto *propertyLayout = new QHBoxLayout(m_controls.m_annotationPropertyPanel);
    propertyLayout->setContentsMargins(propertyMarginX, propertyMarginY, propertyMarginX, propertyMarginY);
    propertyLayout->setSpacing(propertySpacing);
    auto addPropertyGlyph = [this, propertyLayout, propertyGlyphSize, panelInk](markshot::ui::PropertyIcon icon,
                                                                               const QString &tooltip) {
        auto *label = new QLabel(m_controls.m_annotationPropertyPanel);
        label->setObjectName(QStringLiteral("propertyGlyph"));
        label->setAlignment(Qt::AlignCenter);
        label->setPixmap(
            markshot::ui::makePropertyIcon(icon, panelInk).pixmap(QSize(propertyGlyphSize, propertyGlyphSize)));
        label->setToolTip(tooltip);
        propertyLayout->addWidget(label);
        return label;
    };
    auto configurePropertyValueLabel = [scaledWidth](QLabel *label, int width, const QString &tooltip) {
        // 数值位于滑块右侧，固定宽度右对齐，数字位数变化时滑块不抖动
        label->setObjectName(QStringLiteral("propertyValue"));
        label->setAlignment(Qt::AlignRight | Qt::AlignVCenter);
        label->setFixedWidth(scaledWidth(width));
        label->setToolTip(tooltip);
    };

    m_controls.m_annotationPropertyTitle = new QLabel(QStringLiteral("Object"), m_controls.m_annotationPropertyPanel);
    m_controls.m_annotationPropertyTitle->setObjectName(QStringLiteral("propertyTitle"));
    m_controls.m_annotationPropertyTitle->setAlignment(Qt::AlignCenter);
    m_controls.m_annotationPropertyTitle->setMinimumWidth(scaledWidth(58));
    m_controls.m_annotationPropertyTitle->setToolTip(MS_TR("Selected object type"));
    // 单个工具不重复显示名称胶囊，多选时由 updateAnnotationPropertyPanel 显示分组数量
    m_controls.m_annotationPropertyTitle->hide();
    propertyLayout->addWidget(m_controls.m_annotationPropertyTitle);
    propertyLayout->addSpacing(propertyMinorSpacing);
    addPropertyGlyph(markshot::ui::PropertyIcon::StrokeWidth, MS_TR("Selected object width or size"));
    m_controls.m_propertyWidthLabel = new QLabel(QStringLiteral("2"), m_controls.m_annotationPropertyPanel);
    configurePropertyValueLabel(m_controls.m_propertyWidthLabel, 34, MS_TR("Selected object width or size"));
    m_controls.m_propertyWidthSlider = new QSlider(Qt::Horizontal, m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyWidthSlider->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyWidthSlider->setFixedWidth(scaledWidth(88));
    m_controls.m_propertyWidthSlider->setToolTip(MS_TR("Selected object width or size"));
    connect(m_controls.m_propertyWidthSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationWidth(value); });
    propertyLayout->addWidget(m_controls.m_propertyWidthSlider);
    propertyLayout->addWidget(m_controls.m_propertyWidthLabel);
    propertyLayout->addSpacing(propertyMinorSpacing);
    addPropertyGlyph(markshot::ui::PropertyIcon::Opacity, MS_TR("Selected object opacity"));
    m_controls.m_propertyOpacityLabel = new QLabel(QStringLiteral("100%"), m_controls.m_annotationPropertyPanel);
    configurePropertyValueLabel(m_controls.m_propertyOpacityLabel, 36, MS_TR("Selected object opacity"));
    m_controls.m_propertyOpacitySlider = new QSlider(Qt::Horizontal, m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyOpacitySlider->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyOpacitySlider->setRange(0, 100);
    m_controls.m_propertyOpacitySlider->setFixedWidth(scaledWidth(76));
    m_controls.m_propertyOpacitySlider->setToolTip(MS_TR("Selected object opacity"));
    connect(m_controls.m_propertyOpacitySlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationOpacity(value); });
    propertyLayout->addWidget(m_controls.m_propertyOpacitySlider);
    propertyLayout->addWidget(m_controls.m_propertyOpacityLabel);
    propertyLayout->addSpacing(propertyMinorSpacing);
    m_controls.m_propertyColorButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyColorButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Color, panelInk));
    m_controls.m_propertyColorButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyColorButton->setToolTip(MS_TR("Change selected object color"));
    m_controls.m_propertyColorButton->setAccessibleName(MS_TR("Change selected object color"));
    connect(m_controls.m_propertyColorButton, &QPushButton::clicked, this, [this] { openSelectedAnnotationColorPalette(); });
    propertyLayout->addWidget(m_controls.m_propertyColorButton);
    m_controls.m_propertyTextBackgroundButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyTextBackgroundButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyTextBackgroundButton->setIcon(
        markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::TextBackground, panelInk));
    m_controls.m_propertyTextBackgroundButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyTextBackgroundButton->setToolTip(MS_TR("Text background color"));
    m_controls.m_propertyTextBackgroundButton->setAccessibleName(MS_TR("Text background color"));
    connect(m_controls.m_propertyTextBackgroundButton, &QPushButton::clicked, this, [this] { openSelectedTextBackgroundColorPalette(); });
    propertyLayout->addWidget(m_controls.m_propertyTextBackgroundButton);
    m_controls.m_propertyFillButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyFillButton->setCheckable(true);
    m_controls.m_propertyFillButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(false, panelInk));
    m_controls.m_propertyFillButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyFillButton->setToolTip(MS_TR("Toggle shape fill"));
    m_controls.m_propertyFillButton->setAccessibleName(MS_TR("Toggle shape fill"));
    connect(m_controls.m_propertyFillButton, &QPushButton::toggled, this, [this, panelInk](bool checked) {
        m_controls.m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(checked, panelInk));
        setSelectedAnnotationFilled(checked);
    });
    propertyLayout->addWidget(m_controls.m_propertyFillButton);
    m_controls.m_propertyRadiusGlyphLabel = addPropertyGlyph(markshot::ui::PropertyIcon::CornerRadius, MS_TR("Rectangle corner radius"));
    m_controls.m_propertyRadiusLabel = new QLabel(QStringLiteral("0"), m_controls.m_annotationPropertyPanel);
    configurePropertyValueLabel(m_controls.m_propertyRadiusLabel, 24, MS_TR("Rectangle corner radius"));
    m_controls.m_propertyRadiusSlider = new QSlider(Qt::Horizontal, m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyRadiusSlider->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyRadiusSlider->setRange(0, 48);
    m_controls.m_propertyRadiusSlider->setFixedWidth(scaledWidth(72));
    m_controls.m_propertyRadiusSlider->setToolTip(MS_TR("Rectangle corner radius"));
    connect(m_controls.m_propertyRadiusSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationCornerRadius(value); });
    propertyLayout->addWidget(m_controls.m_propertyRadiusSlider);
    propertyLayout->addWidget(m_controls.m_propertyRadiusLabel);
    initializePropertyStyleCombos(propertyLayout);
    m_controls.m_propertyResetNumberButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyResetNumberButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyResetNumberButton->setIcon(
        markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::ResetNumber, panelInk));
    m_controls.m_propertyResetNumberButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyResetNumberButton->setToolTip(MS_TR("Reset number sequence"));
    m_controls.m_propertyResetNumberButton->setAccessibleName(MS_TR("Reset number sequence"));
    connect(m_controls.m_propertyResetNumberButton, &QPushButton::clicked, this, [this] { resetNumberSequence(); });
    propertyLayout->addWidget(m_controls.m_propertyResetNumberButton);
    m_controls.m_propertyMagnifierScaleGlyphLabel =
        addPropertyGlyph(markshot::ui::PropertyIcon::MagnifierScale, MS_TR("Magnifier scale"));
    m_controls.m_propertyMagnifierScaleLabel = new QLabel(magnifierScaleText(kDefaultMagnifierScale), m_controls.m_annotationPropertyPanel);
    configurePropertyValueLabel(m_controls.m_propertyMagnifierScaleLabel, 48, MS_TR("Magnifier scale"));
    m_controls.m_propertyMagnifierScaleSlider = new QSlider(Qt::Horizontal, m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyMagnifierScaleSlider->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyMagnifierScaleSlider->setRange(magnifierScaleSliderValue(kMinMagnifierScale),
                                             magnifierScaleSliderValue(kMaxMagnifierScale));
    m_controls.m_propertyMagnifierScaleSlider->setFixedWidth(scaledWidth(84));
    m_controls.m_propertyMagnifierScaleSlider->setToolTip(MS_TR("Magnifier scale"));
    connect(m_controls.m_propertyMagnifierScaleSlider, &QSlider::valueChanged, this, [this](int value) {
        setSelectedMagnifierScale(value);
    });
    propertyLayout->addWidget(m_controls.m_propertyMagnifierScaleSlider);
    propertyLayout->addWidget(m_controls.m_propertyMagnifierScaleLabel);
    m_controls.m_propertyMagnifierShapeButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyMagnifierShapeButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyMagnifierShapeButton->setIcon(
        markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::MagnifierShape, panelInk));
    m_controls.m_propertyMagnifierShapeButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyMagnifierShapeButton->setToolTip(MS_TR("Toggle magnifier shape (circle/rectangle)"));
    m_controls.m_propertyMagnifierShapeButton->setAccessibleName(MS_TR("Toggle magnifier shape"));
    m_controls.m_propertyMagnifierShapeButton->setCheckable(true);
    connect(m_controls.m_propertyMagnifierShapeButton, &QPushButton::clicked, this, [this] { toggleMagnifierShape(); });
    propertyLayout->addWidget(m_controls.m_propertyMagnifierShapeButton);
    m_controls.m_propertyFontButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyFontButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyFontButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Font, panelInk));
    m_controls.m_propertyFontButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyFontButton->setToolTip(MS_TR("Text font"));
    m_controls.m_propertyFontButton->setAccessibleName(MS_TR("Text font"));
    connect(m_controls.m_propertyFontButton, &QPushButton::clicked, this, [this] { toggleSelectedTextFontPanel(); });
    propertyLayout->addWidget(m_controls.m_propertyFontButton);
    m_controls.m_propertyEditTextButton = new QPushButton(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyEditTextButton->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyEditTextButton->setIcon(
        markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::EditText, panelInk));
    m_controls.m_propertyEditTextButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_controls.m_propertyEditTextButton->setToolTip(MS_TR("Edit selected text"));
    m_controls.m_propertyEditTextButton->setAccessibleName(MS_TR("Edit selected text"));
    connect(m_controls.m_propertyEditTextButton, &QPushButton::clicked, this, [this] { beginEditingSelectedTextAnnotation(); });
    propertyLayout->addWidget(m_controls.m_propertyEditTextButton);
    m_controls.m_annotationPropertyPanel->hide();

    m_controls.m_propertyColorDialogPanel = new QWidget(this);
    m_controls.m_propertyColorDialogPanel->setObjectName(QStringLiteral("propertyColorDialogPanel"));
    m_controls.m_propertyColorDialogPanel->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyColorDialogPanel->setStyleSheet(
        markshot::theme::propertyColorDialogPanelStyleSheet(m_toolbarAppearance.palette));
    auto *propertyColorLayout = new QVBoxLayout(m_controls.m_propertyColorDialogPanel);
    propertyColorLayout->setContentsMargins(8, 8, 8, 8);
    propertyColorLayout->setSpacing(0);
    m_controls.m_propertyColorPicker = new markshot::ui::ColorPicker(m_controls.m_propertyColorDialogPanel);
    m_controls.m_propertyColorPicker->setToolbarPalette(m_toolbarAppearance.palette);
    m_controls.m_propertyColorPicker->setColor(m_currentColor);
    connect(m_controls.m_propertyColorPicker, &markshot::ui::ColorPicker::colorChanged, this,
            [this](const QColor &color) { applyPropertyColor(color); });
    propertyColorLayout->addWidget(m_controls.m_propertyColorPicker);
    m_controls.m_propertyColorDialogPanel->hide();

    initializePropertyFontPanel();

    initializeTransientPanels();
    initializeTextEditor();
    initializeLaserTimer();
    initializeWindowDetection(std::move(windowInfos), windowDetectionEnabled);
}

void ShotWindow::setCaptureScreen(QScreen *screen)
{
    m_captureScreen = screen;
}

/// @brief 【截图工具栏】【主面板】创建紧凑组件并按业务职责注册按钮分组
/// @return 无返回值
void ShotWindow::initializeToolbar()
{
    // 1. 创建共享面板组件，业务窗口只管理父子关系与拖拽定位
    auto *panel = new markshot::ui::CompactToolbar(
        QStringLiteral("shotToolbar"), QStringLiteral("toolbarGrip"), Qt::Horizontal,
        {m_toolbarAppearance.toolbarIconSize, 6, 2}, this);
    m_controls.m_toolbarPanel = panel;
    m_controls.m_toolbar = panel;
    panel->setToolbarPalette(m_toolbarAppearance.palette);
    panel->setDynamicIconCallback([this](QPushButton *button, const QColor &ink) -> QIcon {
        // 1. 【截图工具栏】【动态图标】为形状标记按钮根据当前形状与悬停前景色生成图标
        if (!button || !button->property("dynamicIcon").toBool()) {
            return {};
        }
        return markshot::ui::makeMarkerShapeIcon(m_markerShape, ink, m_markerFilled);
    });
    panel->setStyleSheet(markshot::theme::panelStyleSheet(
        m_toolbarAppearance.palette, m_toolbarAppearance.toolbarButtonSize, m_toolbarAppearance.fontSize));
    // 工具栏点击需要清除滚轮预览，握柄事件进入窗口的拖拽与吸附逻辑
    panel->installEventFilter(this);
    panel->grip()->installEventFilter(this);

    // 2. 主工具、历史操作与全屏动作分别建组；组件管理按钮创建和分隔线显隐
    auto addGroup = [this, panel](const auto &actions, bool fullscreenOnly = false) {
        const int group = panel->addGroup();
        for (Action action : actions) {
            const auto tool = toolForToolbarAction(action);
            const QString shortcut = tool ? shortcutText(*tool)
                                          : shortcutText(action, toolbarShortcutFallback(action));
            addToolbarButton(action, shortcut);
        }
        if (fullscreenOnly) {
            m_controls.m_fullscreenActionGroups.append(group);
            panel->setGroupVisible(group, false);
        }
    };
    addGroup(kMainSelectGroup);
    addGroup(kMainDrawGroup);
    addGroup(kMainShapeGroup);
    addGroup(kMainMarkupGroup);
    addGroup(kMainHistoryGroup);
    addGroup(kFullscreenViewGroup, true);
    addGroup(kFullscreenOutputGroup, true);
    addGroup(kSessionSettingsGroup, true);
    addGroup(kSessionCloseGroup, true);
    refreshShapeMarkerToolbarButton();
    panel->hide();
}

void ShotWindow::initializeImageScrollBars()
{
    m_controls.m_horizontalImageScrollBar = new QScrollBar(Qt::Horizontal, this);
    m_controls.m_horizontalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_controls.m_horizontalImageScrollBar->hide();
    m_controls.m_verticalImageScrollBar = new QScrollBar(Qt::Vertical, this);
    m_controls.m_verticalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_controls.m_verticalImageScrollBar->hide();
    markshot::ui::setupCanvasScrollBar(m_controls.m_horizontalImageScrollBar, kImageScrollBarExtent);
    markshot::ui::setupCanvasScrollBar(m_controls.m_verticalImageScrollBar, kImageScrollBarExtent);
    connect(m_controls.m_horizontalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });
    connect(m_controls.m_verticalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });
}

/// @brief 【截图工具栏】【副面板】创建选区输出动作的纵向紧凑组件
/// @return 无返回值
void ShotWindow::initializeActionToolbar()
{
    // 1. 复用主面板样式，并按副工具栏配置收紧按钮尺寸
    auto *panel = new markshot::ui::CompactToolbar(
        QStringLiteral("actionToolbar"), QStringLiteral("actionToolbarGrip"), Qt::Vertical,
        {m_toolbarAppearance.actionToolbarIconSize, 4, 2}, this);
    m_controls.m_actionToolbarPanel = panel;
    m_controls.m_actionToolbar = panel;
    panel->setToolbarPalette(m_toolbarAppearance.palette);
    panel->setStyleSheet(m_controls.m_toolbar->styleSheet()
        + QStringLiteral(
              "QWidget#actionToolbar QPushButton {"
              " border-radius: 6px;"
              " min-width: %1px;"
              " min-height: %1px;"
              " max-width: %1px;"
              " max-height: %1px;"
              "}").arg(m_toolbarAppearance.actionToolbarButtonSize));
    panel->grip()->installEventFilter(this);

    // 2. 视图、扩展、输出、会话分组注册按钮，业务动作仍由窗口映射
    auto addGroup = [this, panel](const auto &actions) {
        panel->addGroup();
        for (Action action : actions) {
            addToolbarButton(action, shortcutText(action, toolbarShortcutFallback(action)), panel);
        }
    };
    addGroup(kActionScopeGroup);
    addGroup(kActionExtendGroup);
    addGroup(kActionOutputGroup);
    addGroup(kSessionSettingsGroup);
    addGroup(kSessionCloseGroup);
    panel->hide();
}

void ShotWindow::initializeTransientPanels()
{
    m_controls.m_openWithPanel = new QWidget(this);
    m_controls.m_openWithPanel->setObjectName(QStringLiteral("openWithPanel"));
    m_controls.m_openWithPanel->setCursor(Qt::ArrowCursor);
    m_controls.m_openWithPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet(m_toolbarAppearance.palette));
    auto *openLayout = new QVBoxLayout(m_controls.m_openWithPanel);
    openLayout->setContentsMargins(8, 8, 8, 8);
    openLayout->setSpacing(4);
    m_controls.m_openWithPanel->hide();

    m_controls.m_extensionPanel = new QWidget(this);
    m_controls.m_extensionPanel->setObjectName(QStringLiteral("extensionPanel"));
    m_controls.m_extensionPanel->setCursor(Qt::ArrowCursor);
    m_controls.m_extensionPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet(m_toolbarAppearance.palette));
    auto *extensionLayout = new QVBoxLayout(m_controls.m_extensionPanel);
    extensionLayout->setContentsMargins(8, 8, 8, 8);
    extensionLayout->setSpacing(4);
    m_controls.m_extensionPanel->hide();

    m_controls.m_colorPalette = new QWidget(this);
    m_controls.m_colorPalette->setObjectName(QStringLiteral("colorPalette"));
    m_controls.m_colorPalette->setCursor(Qt::ArrowCursor);
    m_controls.m_colorPalette->setStyleSheet(markshot::theme::colorPaletteStyleSheet());
    for (const QColor &color : markshot::theme::paletteColors()) {
        auto *button = new QPushButton(m_controls.m_colorPalette);
        button->setFocusPolicy(Qt::NoFocus);
        button->setStyleSheet(QStringLiteral("background: %1;").arg(color.name()));
        connect(button, &QPushButton::clicked, this, [this, color] { setCurrentColor(color); });
    }
    m_controls.m_colorPalettePreview = new QWidget(m_controls.m_colorPalette);
    m_controls.m_colorPalettePreview->setObjectName(QStringLiteral("colorPalettePreview"));
    m_controls.m_colorPalette->installEventFilter(this);

    // 形状标记下拉面板：网格选择三角形/星形/对钩等
    m_controls.m_shapeMarkerPopup = new QWidget(this);
    m_controls.m_shapeMarkerPopup->setObjectName(QStringLiteral("shapeMarkerPopup"));
    m_controls.m_shapeMarkerPopup->setCursor(Qt::ArrowCursor);
    m_controls.m_shapeMarkerPopup->setStyleSheet(m_controls.m_toolbar->styleSheet());
    auto *shapeLayout = new QGridLayout(m_controls.m_shapeMarkerPopup);
    shapeLayout->setContentsMargins(8, 8, 8, 8);
    shapeLayout->setHorizontalSpacing(6);
    shapeLayout->setVerticalSpacing(6);
    const auto shapes = markshot::marker::allShapes();
    for (int i = 0; i < static_cast<int>(shapes.size()); ++i) {
        const ShotWindow::MarkerShape shape = shapes[static_cast<size_t>(i)];
        auto *button = new QPushButton(m_controls.m_shapeMarkerPopup);
        button->setFocusPolicy(Qt::NoFocus);
        button->setCursor(Qt::ArrowCursor);
        button->setIcon(markshot::ui::makeMarkerShapeIcon(shape, m_toolbarAppearance.palette.text));
        button->setIconSize(QSize(m_toolbarAppearance.toolbarIconSize, m_toolbarAppearance.toolbarIconSize));
        button->setToolTip(markshot::i18n::translate(markshot::marker::shapeLabel(shape)));
        button->setProperty("markerShape", static_cast<int>(shape));
        button->setCheckable(true);
        connect(button, &QPushButton::clicked, this, [this, shape] {
            setSelectedMarkerShape(shape);
            setTool(Tool::Marker);
            hideShapeMarkerPopup();
        });
        // 6 列网格，容纳扑克花色与更多形状
        shapeLayout->addWidget(button, i / 6, i % 6);
    }

    // 末行：填充 / 描边切换，作用于新标记与选中的形状标记
    const int fillRow = (static_cast<int>(shapes.size()) + 5) / 6;
    const auto addFillModeButton = [this, shapeLayout, fillRow](bool filled, int column) {
        auto *button = new QPushButton(m_controls.m_shapeMarkerPopup);
        button->setFocusPolicy(Qt::NoFocus);
        button->setCursor(Qt::ArrowCursor);
        button->setIcon(markshot::ui::makeMarkerShapeIcon(ShotWindow::MarkerShape::Circle,
                                                          m_toolbarAppearance.palette.text,
                                                          filled));
        button->setIconSize(QSize(m_toolbarAppearance.toolbarIconSize, m_toolbarAppearance.toolbarIconSize));
        button->setToolTip(markshot::i18n::translate(filled ? QStringLiteral("Filled")
                                                            : QStringLiteral("Outline")));
        button->setProperty("markerFillMode", filled);
        button->setCheckable(true);
        connect(button, &QPushButton::clicked, this, [this, filled] {
            setSelectedMarkerFill(filled);
        });
        shapeLayout->addWidget(button, fillRow, column, 1, 3);
    };
    addFillModeButton(true, 0);
    addFillModeButton(false, 3);
    m_controls.m_shapeMarkerPopup->hide();

    m_controls.m_colorPalette->hide();
    updateColorPalettePreview();
}

void ShotWindow::initializeLaserTimer()
{
    m_laserClock.start();
    m_laserTimer = new QTimer(this);
    m_laserTimer->setInterval(33);
    connect(m_laserTimer, &QTimer::timeout, this, [this] { cleanupLaserStrokes(); });
}

void ShotWindow::initializeWindowDetection(QVector<markshot::WindowInfo> windowInfos, bool enabled)
{
    if (!enabled) {
        return;
    }

    if (windowInfos.isEmpty()) {
#if defined(Q_OS_WIN)
        windowInfos = markshot::windows::enumerateWindowInfos();
#else
        windowInfos = enumerateX11WindowInfos();
#endif
    }
    for (const markshot::WindowInfo &info : std::as_const(windowInfos)) {
        const QRect imageRect = windowGeometryToImageRect(info.rect,
                                                          m_sourceGeometry,
                                                          m_frozenFrame.size());
        if (imageRect.width() > 1 && imageRect.height() > 1) {
            markshot::WindowInfo converted;
            converted.rect = imageRect;
            converted.zOrder = info.zOrder;
            m_windowInfos.append(converted);
        }
    }
}

void ShotWindow::startFullscreenAnnotation()
{
    enterFullscreenAnnotation(true);
}

void ShotWindow::setImageNavigationEnabled(bool enabled)
{
    m_imageNavigationEnabled = enabled;
    // 打开图片的独立窗口不停靠工具栏，切换后按新状态恢复排列方向
    restoreConfiguredToolbarLayout();
    if (!enabled) {
        m_imageZoom = 1.0;
        m_imageCenterInitialized = false;
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateMinimumImageWindowSize();
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::setDefaultTool(Tool tool)
{
    setDefaultTools(tool, tool);
}

void ShotWindow::setDefaultTools(Tool tool, Tool fullscreenTool)
{
    m_defaultTool = tool;
    m_fullscreenDefaultTool = fullscreenTool;
}

void ShotWindow::setDefaultColor(QColor color)
{
    setCurrentColor(color);
}
