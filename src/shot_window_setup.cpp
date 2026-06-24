#include "shot_window_module.h"

#include "app_config_store.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;


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
    };
}

ShotWindow::ShotWindow(QImage frozenFrame,
                       QString outputName,
                       QRect sourceGeometry,
                       QVector<QRect> windowGeometries,
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
    m_autoSelectAfterDrawByTool = annotationAutoSelectAfterDrawTools();
    bool appConfigOk = false;
    const QJsonObject appConfigRoot = markshot::readAppConfigRoot(&appConfigOk);
    if (appConfigOk) {
        m_toolbarAppearance = markshot::toolbarAppearanceFromConfigRoot(appConfigRoot);
    }
    // 在初始化 UI 之前先加载上次会话的工具默认值,使后续 toolbar/属性面板按
    // 持久化的状态显示;loadAnnotationStateFromDisk 仅修改 m_* 默认值字段,
    // 不会触碰任何尚未构造完成的控件
    loadAnnotationStateFromDisk();

    // 节流定时器:把高频改动(如拖动滑块)合并成一次磁盘写入,避免主线程
    // 在 QSaveFile 写盘期间无法及时刷新滑块和画布
    m_annotationStateTimer = new QTimer(this);
    m_annotationStateTimer->setSingleShot(true);
    m_annotationStateTimer->setInterval(250);
    connect(m_annotationStateTimer, &QTimer::timeout, this, [this] { flushAnnotationStateNow(); });

    // 选中标注的滚轮改粗细需要进入撤销历史,但连续滚轮只应对应一次撤销
    m_annotationWidthWheelHistoryTimer = new QTimer(this);
    m_annotationWidthWheelHistoryTimer->setSingleShot(true);
    m_annotationWidthWheelHistoryTimer->setInterval(350);
    connect(m_annotationWidthWheelHistoryTimer, &QTimer::timeout, this, [this] {
        commitAnnotationWidthWheelHistory();
    });

    setWindowTitle(MS_TR("Mark Shot"));
    setCursor(captureCrossCursor());
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    markshot::windows::setExcludedFromCapture(this);
    if (m_frozenFrame.format() != QImage::Format_ARGB32_Premultiplied) {
        m_frozenFrame = m_frozenFrame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    }
    // Annotation and selection geometry are stored in image pixels. Clear any
    // high-DPI metadata from screen grabs so Qt painting does not apply a second
    // device-pixel-ratio scale on top of our explicit image-to-widget mapping.
    m_frozenFrame.setDevicePixelRatio(1.0);

    initializeToolbar();
    initializeImageScrollBars();
    initializeActionToolbar();
    initializeShortcuts();

    m_annotationPropertyPanel = new QWidget(this);
    m_annotationPropertyPanel->setObjectName(QStringLiteral("annotationPropertyPanel"));
    m_annotationPropertyPanel->setStyleSheet(m_toolbar->styleSheet());
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
    auto *propertyLayout = new QHBoxLayout(m_annotationPropertyPanel);
    propertyLayout->setContentsMargins(propertyMarginX, propertyMarginY, propertyMarginX, propertyMarginY);
    propertyLayout->setSpacing(propertySpacing);
    auto addPropertyGlyph = [this, propertyLayout, propertyGlyphSize](markshot::ui::PropertyIcon icon,
                                                                     const QString &tooltip) {
        auto *label = new QLabel(m_annotationPropertyPanel);
        label->setObjectName(QStringLiteral("propertyGlyph"));
        label->setAlignment(Qt::AlignCenter);
        label->setPixmap(markshot::ui::makePropertyIcon(icon).pixmap(QSize(propertyGlyphSize, propertyGlyphSize)));
        label->setToolTip(tooltip);
        propertyLayout->addWidget(label);
        return label;
    };
    auto configurePropertyValueLabel = [scaledWidth](QLabel *label, int width, const QString &tooltip) {
        label->setObjectName(QStringLiteral("propertyValue"));
        label->setAlignment(Qt::AlignCenter);
        label->setFixedWidth(scaledWidth(width));
        label->setToolTip(tooltip);
    };

    m_annotationPropertyTitle = new QLabel(QStringLiteral("Object"), m_annotationPropertyPanel);
    m_annotationPropertyTitle->setObjectName(QStringLiteral("propertyTitle"));
    m_annotationPropertyTitle->setAlignment(Qt::AlignCenter);
    m_annotationPropertyTitle->setMinimumWidth(scaledWidth(58));
    m_annotationPropertyTitle->setToolTip(MS_TR("Selected object type"));
    propertyLayout->addWidget(m_annotationPropertyTitle);
    propertyLayout->addSpacing(propertyMinorSpacing);
    addPropertyGlyph(markshot::ui::PropertyIcon::StrokeWidth, MS_TR("Selected object width or size"));
    m_propertyWidthLabel = new QLabel(QStringLiteral("2"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyWidthLabel, 34, MS_TR("Selected object width or size"));
    propertyLayout->addWidget(m_propertyWidthLabel);
    m_propertyWidthSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyWidthSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyWidthSlider->setFixedWidth(scaledWidth(88));
    m_propertyWidthSlider->setToolTip(MS_TR("Selected object width or size"));
    connect(m_propertyWidthSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationWidth(value); });
    propertyLayout->addWidget(m_propertyWidthSlider);
    propertyLayout->addSpacing(propertyMinorSpacing);
    addPropertyGlyph(markshot::ui::PropertyIcon::Opacity, MS_TR("Selected object opacity"));
    m_propertyOpacityLabel = new QLabel(QStringLiteral("100%"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyOpacityLabel, 36, MS_TR("Selected object opacity"));
    propertyLayout->addWidget(m_propertyOpacityLabel);
    m_propertyOpacitySlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyOpacitySlider->setFocusPolicy(Qt::NoFocus);
    m_propertyOpacitySlider->setRange(0, 100);
    m_propertyOpacitySlider->setFixedWidth(scaledWidth(76));
    m_propertyOpacitySlider->setToolTip(MS_TR("Selected object opacity"));
    connect(m_propertyOpacitySlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationOpacity(value); });
    propertyLayout->addWidget(m_propertyOpacitySlider);
    propertyLayout->addSpacing(propertyMinorSpacing);
    m_propertyColorButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyColorButton->setFocusPolicy(Qt::NoFocus);
    m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Color));
    m_propertyColorButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyColorButton->setToolTip(MS_TR("Change selected object color"));
    m_propertyColorButton->setAccessibleName(MS_TR("Change selected object color"));
    connect(m_propertyColorButton, &QPushButton::clicked, this, [this] { openSelectedAnnotationColorPalette(); });
    propertyLayout->addWidget(m_propertyColorButton);
    m_propertyTextBackgroundButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyTextBackgroundButton->setFocusPolicy(Qt::NoFocus);
    m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::TextBackground));
    m_propertyTextBackgroundButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyTextBackgroundButton->setToolTip(MS_TR("Text background color"));
    m_propertyTextBackgroundButton->setAccessibleName(MS_TR("Text background color"));
    connect(m_propertyTextBackgroundButton, &QPushButton::clicked, this, [this] { openSelectedTextBackgroundColorPalette(); });
    propertyLayout->addWidget(m_propertyTextBackgroundButton);
    m_propertyFillButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFillButton->setCheckable(true);
    m_propertyFillButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(false));
    m_propertyFillButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyFillButton->setToolTip(MS_TR("Toggle shape fill"));
    m_propertyFillButton->setAccessibleName(MS_TR("Toggle shape fill"));
    connect(m_propertyFillButton, &QPushButton::toggled, this, [this](bool checked) {
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(checked));
        setSelectedAnnotationFilled(checked);
    });
    propertyLayout->addWidget(m_propertyFillButton);
    m_propertyRadiusGlyphLabel = addPropertyGlyph(markshot::ui::PropertyIcon::CornerRadius, MS_TR("Rectangle corner radius"));
    m_propertyRadiusLabel = new QLabel(QStringLiteral("0"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyRadiusLabel, 24, MS_TR("Rectangle corner radius"));
    propertyLayout->addWidget(m_propertyRadiusLabel);
    m_propertyRadiusSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyRadiusSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyRadiusSlider->setRange(0, 48);
    m_propertyRadiusSlider->setFixedWidth(scaledWidth(72));
    m_propertyRadiusSlider->setToolTip(MS_TR("Rectangle corner radius"));
    connect(m_propertyRadiusSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationCornerRadius(value); });
    propertyLayout->addWidget(m_propertyRadiusSlider);
    // 矩形风格切换:描边/高亮/反色,仅在 Tool::Rectangle 选中或激活时显示
    m_propertyRectangleStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyRectangleStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyRectangleStyleCombo->addItem(MS_TR("Stroke"), static_cast<int>(RectangleStyle::Stroke));
    m_propertyRectangleStyleCombo->addItem(MS_TR("Highlight"), static_cast<int>(RectangleStyle::Highlight));
    m_propertyRectangleStyleCombo->addItem(MS_TR("Invert"), static_cast<int>(RectangleStyle::Invert));
    m_propertyRectangleStyleCombo->setToolTip(MS_TR("Rectangle style"));
    m_propertyRectangleStyleCombo->setAccessibleName(MS_TR("Rectangle style"));
    connect(m_propertyRectangleStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyRectangleStyleCombo) {
            return;
        }
        setSelectedRectangleStyle(
            static_cast<RectangleStyle>(m_propertyRectangleStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyRectangleStyleCombo);
    m_propertyArrowStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyArrowStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyArrowStyleCombo->addItem(MS_TR("Fletched"), static_cast<int>(ArrowStyle::Fletched));
    m_propertyArrowStyleCombo->addItem(MS_TR("KDE"), static_cast<int>(ArrowStyle::Kde));
    m_propertyArrowStyleCombo->addItem(MS_TR("Double fletched"), static_cast<int>(ArrowStyle::BidirectionalFletched));
    m_propertyArrowStyleCombo->addItem(MS_TR("Double KDE"), static_cast<int>(ArrowStyle::BidirectionalKde));
    m_propertyArrowStyleCombo->setToolTip(MS_TR("Arrow style"));
    m_propertyArrowStyleCombo->setAccessibleName(MS_TR("Arrow style"));
    connect(m_propertyArrowStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyArrowStyleCombo) {
            return;
        }
        setSelectedAnnotationArrowStyle(static_cast<ArrowStyle>(m_propertyArrowStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyArrowStyleCombo);
    m_propertyHighlighterStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyHighlighterStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyHighlighterStyleCombo->addItem(MS_TR("Pen"), static_cast<int>(HighlighterStyle::Freehand));
    m_propertyHighlighterStyleCombo->addItem(MS_TR("Line"), static_cast<int>(HighlighterStyle::StraightLine));
    m_propertyHighlighterStyleCombo->setToolTip(MS_TR("Highlighter style"));
    m_propertyHighlighterStyleCombo->setAccessibleName(MS_TR("Highlighter style"));
    connect(m_propertyHighlighterStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyHighlighterStyleCombo) {
            return;
        }
        setSelectedHighlighterStyle(
            static_cast<HighlighterStyle>(m_propertyHighlighterStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyHighlighterStyleCombo);
    m_propertyNumberStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyNumberStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyNumberStyleCombo->addItem(MS_TR("1, 2, 3"), static_cast<int>(NumberStyle::Arabic));
    m_propertyNumberStyleCombo->addItem(MS_TR("A, B, C"), static_cast<int>(NumberStyle::UpperAlpha));
    m_propertyNumberStyleCombo->addItem(MS_TR("a, b, c"), static_cast<int>(NumberStyle::LowerAlpha));
    m_propertyNumberStyleCombo->addItem(MS_TR("I, II, III"), static_cast<int>(NumberStyle::UpperRoman));
    m_propertyNumberStyleCombo->addItem(MS_TR("i, ii, iii"), static_cast<int>(NumberStyle::LowerRoman));
    m_propertyNumberStyleCombo->addItem(MS_TR("甲, 乙, 丙"), static_cast<int>(NumberStyle::HeavenlyStem));
    m_propertyNumberStyleCombo->addItem(MS_TR("一, 二, 三"), static_cast<int>(NumberStyle::Chinese));
    m_propertyNumberStyleCombo->setToolTip(MS_TR("Number style"));
    m_propertyNumberStyleCombo->setAccessibleName(MS_TR("Number style"));
    connect(m_propertyNumberStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_propertyNumberStyleCombo) {
            return;
        }
        setSelectedNumberStyle(static_cast<NumberStyle>(m_propertyNumberStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_propertyNumberStyleCombo);
    m_propertyResetNumberButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyResetNumberButton->setFocusPolicy(Qt::NoFocus);
    m_propertyResetNumberButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::ResetNumber));
    m_propertyResetNumberButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyResetNumberButton->setToolTip(MS_TR("Reset number sequence"));
    m_propertyResetNumberButton->setAccessibleName(MS_TR("Reset number sequence"));
    connect(m_propertyResetNumberButton, &QPushButton::clicked, this, [this] { resetNumberSequence(); });
    propertyLayout->addWidget(m_propertyResetNumberButton);
    m_propertyMagnifierScaleGlyphLabel =
        addPropertyGlyph(markshot::ui::PropertyIcon::MagnifierScale, MS_TR("Magnifier scale"));
    m_propertyMagnifierScaleLabel = new QLabel(magnifierScaleText(kDefaultMagnifierScale), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyMagnifierScaleLabel, 48, MS_TR("Magnifier scale"));
    propertyLayout->addWidget(m_propertyMagnifierScaleLabel);
    m_propertyMagnifierScaleSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyMagnifierScaleSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyMagnifierScaleSlider->setRange(magnifierScaleSliderValue(kMinMagnifierScale),
                                             magnifierScaleSliderValue(kMaxMagnifierScale));
    m_propertyMagnifierScaleSlider->setFixedWidth(scaledWidth(84));
    m_propertyMagnifierScaleSlider->setToolTip(MS_TR("Magnifier scale"));
    connect(m_propertyMagnifierScaleSlider, &QSlider::valueChanged, this, [this](int value) {
        setSelectedMagnifierScale(value);
    });
    propertyLayout->addWidget(m_propertyMagnifierScaleSlider);
    m_propertyMagnifierShapeButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyMagnifierShapeButton->setFocusPolicy(Qt::NoFocus);
    m_propertyMagnifierShapeButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::MagnifierShape));
    m_propertyMagnifierShapeButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyMagnifierShapeButton->setToolTip(MS_TR("Toggle magnifier shape (circle/rectangle)"));
    m_propertyMagnifierShapeButton->setAccessibleName(MS_TR("Toggle magnifier shape"));
    m_propertyMagnifierShapeButton->setCheckable(true);
    connect(m_propertyMagnifierShapeButton, &QPushButton::clicked, this, [this] { toggleMagnifierShape(); });
    propertyLayout->addWidget(m_propertyMagnifierShapeButton);
    m_propertyFontButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFontButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFontButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Font));
    m_propertyFontButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyFontButton->setToolTip(MS_TR("Text font"));
    m_propertyFontButton->setAccessibleName(MS_TR("Text font"));
    connect(m_propertyFontButton, &QPushButton::clicked, this, [this] { toggleSelectedTextFontPanel(); });
    propertyLayout->addWidget(m_propertyFontButton);
    m_propertyEditTextButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyEditTextButton->setFocusPolicy(Qt::NoFocus);
    m_propertyEditTextButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::EditText));
    m_propertyEditTextButton->setIconSize(QSize(propertyButtonIconSize, propertyButtonIconSize));
    m_propertyEditTextButton->setToolTip(MS_TR("Edit selected text"));
    m_propertyEditTextButton->setAccessibleName(MS_TR("Edit selected text"));
    connect(m_propertyEditTextButton, &QPushButton::clicked, this, [this] { beginEditingSelectedTextAnnotation(); });
    propertyLayout->addWidget(m_propertyEditTextButton);
    m_annotationPropertyPanel->hide();

    m_propertyColorDialogPanel = new QWidget(this);
    m_propertyColorDialogPanel->setObjectName(QStringLiteral("propertyColorDialogPanel"));
    m_propertyColorDialogPanel->setStyleSheet(markshot::theme::propertyColorDialogPanelStyleSheet());
    auto *propertyColorLayout = new QVBoxLayout(m_propertyColorDialogPanel);
    propertyColorLayout->setContentsMargins(8, 8, 8, 8);
    propertyColorLayout->setSpacing(0);
    m_propertyColorPicker = new markshot::ui::ColorPicker(m_propertyColorDialogPanel);
    m_propertyColorPicker->setColor(m_currentColor);
    connect(m_propertyColorPicker, &markshot::ui::ColorPicker::colorChanged, this,
            [this](const QColor &color) { applyPropertyColor(color); });
    propertyColorLayout->addWidget(m_propertyColorPicker);
    m_propertyColorDialogPanel->hide();

    m_propertyFontPanel = new QWidget(this);
    m_propertyFontPanel->setObjectName(QStringLiteral("propertyFontPanel"));
    m_propertyFontPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *fontPanelLayout = new QVBoxLayout(m_propertyFontPanel);
    fontPanelLayout->setContentsMargins(6, 6, 6, 6);
    fontPanelLayout->setSpacing(0);
    m_propertyFontList = new QListWidget(m_propertyFontPanel);
    m_propertyFontList->setFocusPolicy(Qt::NoFocus);
    m_propertyFontList->setUniformItemSizes(true);
    m_propertyFontList->setMinimumHeight(84);
    m_propertyFontList->setMaximumHeight(260);
    m_propertyFontList->setVerticalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    m_propertyFontList->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const QString &family : QFontDatabase::families()) {
        auto *item = new QListWidgetItem(family, m_propertyFontList);
        item->setData(Qt::UserRole, family);
        item->setFont(QFont(family, 12));
    }
    connect(m_propertyFontList, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        setSelectedTextFontFamily(item->data(Qt::UserRole).toString());
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
    });
    fontPanelLayout->addWidget(m_propertyFontList);
    m_propertyFontPanel->hide();

    initializeTransientPanels();
    initializeTextEditor();
    initializeLaserTimer();
    initializeWindowDetection(std::move(windowGeometries), windowDetectionEnabled);
}

void ShotWindow::initializeToolbar()
{
    m_toolbar = new QWidget(this);
    m_toolbar->setObjectName(QStringLiteral("shotToolbar"));
    m_toolbar->setStyleSheet(
        markshot::theme::panelStyleSheet(m_toolbarAppearance.toolbarButtonSize, m_toolbarAppearance.fontSize));
    m_toolbar->installEventFilter(this);

    m_toolbarLayout = new QHBoxLayout(m_toolbar);
    m_toolbarLayout->setContentsMargins(6, 6, 6, 6);
    m_toolbarLayout->setSpacing(3);

    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMove, shortcutText(Tool::Move)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolSelect, shortcutText(Tool::Select)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolPen, shortcutText(Tool::Pen)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLine, shortcutText(Tool::Line)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolHighlighter, shortcutText(Tool::Highlighter)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolRectangle, shortcutText(Tool::Rectangle)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolEllipse, shortcutText(Tool::Ellipse)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolArrow, shortcutText(Tool::Arrow)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolText, shortcutText(Tool::Text)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolNumber, shortcutText(Tool::Number)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMosaic, shortcutText(Tool::Mosaic)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMagnifier, shortcutText(Tool::Magnifier)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLaser, shortcutText(Tool::Laser)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Clear, shortcutText(Action::Clear, QStringLiteral("Clear"))));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Undo, shortcutText(Action::Undo)));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Redo, shortcutText(Action::Redo, QStringLiteral("Ctrl+Shift+Z"))));
    for (Action action : {Action::ToggleCaptureScope,
                          Action::ToggleToolbarLayout,
                          Action::OpenWith,
                          Action::Extensions,
                          Action::ScrollCapture,
                          Action::Pin,
                          Action::OcrCopy,
                          Action::Copy,
                          Action::Save,
                          Action::Upload,
                          Action::Settings,
                          Action::Cancel}) {
        const QString shortcut = action == Action::OpenWith ? shortcutText(action, QStringLiteral("Open"))
            : action == Action::Extensions           ? shortcutText(action, QStringLiteral("Ext"))
            : action == Action::ScrollCapture        ? shortcutText(action, QStringLiteral("Scroll"))
            : action == Action::Pin                  ? shortcutText(action)
            : action == Action::OcrCopy              ? shortcutText(action, QStringLiteral("OCR"))
            : action == Action::Copy                 ? shortcutText(action)
            : action == Action::Save                 ? shortcutText(action, QStringLiteral("Save As"))
            : action == Action::Upload               ? shortcutText(action, QStringLiteral("Upload"))
            : action == Action::Settings             ? shortcutText(action, QStringLiteral("Settings"))
            : action == Action::ToggleToolbarLayout  ? shortcutText(action, QStringLiteral("Layout"))
            : action == Action::ToggleCaptureScope   ? shortcutText(action)
                                                     : shortcutText(action);
        QPushButton *button = addToolbarButton(action, shortcut);
        button->hide();
        m_fullscreenActionButtons.append(button);
        m_toolbarLayout->addWidget(button);
    }
    m_toolbar->hide();
}

void ShotWindow::initializeImageScrollBars()
{
    m_horizontalImageScrollBar = new QScrollBar(Qt::Horizontal, this);
    m_horizontalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_horizontalImageScrollBar->hide();
    m_verticalImageScrollBar = new QScrollBar(Qt::Vertical, this);
    m_verticalImageScrollBar->setFocusPolicy(Qt::NoFocus);
    m_verticalImageScrollBar->hide();
    const QString imageScrollBarStyle = QStringLiteral(
        "QScrollBar { background: rgba(8,13,19,190); border: 0; }"
        "QScrollBar:horizontal { height: 14px; }"
        "QScrollBar:vertical { width: 14px; }"
        "QScrollBar::handle { background: rgba(45,212,191,180); border-radius: 6px; min-width: 28px; min-height: 28px; }"
        "QScrollBar::handle:hover { background: rgba(94,234,212,220); }"
        "QScrollBar::add-line, QScrollBar::sub-line { width: 0; height: 0; }"
        "QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }");
    m_horizontalImageScrollBar->setStyleSheet(imageScrollBarStyle);
    m_verticalImageScrollBar->setStyleSheet(imageScrollBarStyle);
    connect(m_horizontalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });
    connect(m_verticalImageScrollBar, &QScrollBar::valueChanged, this, [this] {
        setImageCenterFromScrollBars();
    });
}

void ShotWindow::initializeActionToolbar()
{
    m_actionToolbar = new QWidget(this);
    m_actionToolbar->setObjectName(QStringLiteral("actionToolbar"));
    const int actionButtonSize = m_toolbarAppearance.actionToolbarButtonSize;
    m_actionToolbar->setStyleSheet(m_toolbar->styleSheet()
        + QStringLiteral(
              "QWidget#actionToolbar QPushButton {"
              " border-radius: 6px;"
              " min-width: %1px;"
              " min-height: %1px;"
              " max-width: %1px;"
              " max-height: %1px;"
              "}").arg(actionButtonSize));
    auto *actionLayout = new QVBoxLayout(m_actionToolbar);
    actionLayout->setContentsMargins(4, 4, 4, 4);
    actionLayout->setSpacing(2);
    for (QPushButton *button : {
             addToolbarButton(Action::ToggleCaptureScope, shortcutText(Action::ToggleCaptureScope), m_actionToolbar),
             addToolbarButton(Action::OpenWith, shortcutText(Action::OpenWith, QStringLiteral("Open")), m_actionToolbar),
             addToolbarButton(Action::Extensions, shortcutText(Action::Extensions, QStringLiteral("Ext")), m_actionToolbar),
             addToolbarButton(Action::ScrollCapture, shortcutText(Action::ScrollCapture, QStringLiteral("Scroll")), m_actionToolbar),
             addToolbarButton(Action::Pin, shortcutText(Action::Pin), m_actionToolbar),
             addToolbarButton(Action::OcrCopy, shortcutText(Action::OcrCopy, QStringLiteral("OCR")), m_actionToolbar),
             addToolbarButton(Action::Copy, shortcutText(Action::Copy), m_actionToolbar),
             addToolbarButton(Action::Save, shortcutText(Action::Save, QStringLiteral("Save As")), m_actionToolbar),
             addToolbarButton(Action::Upload, shortcutText(Action::Upload, QStringLiteral("Upload")), m_actionToolbar),
             addToolbarButton(Action::Settings, shortcutText(Action::Settings, QStringLiteral("Settings")), m_actionToolbar),
             addToolbarButton(Action::Cancel, shortcutText(Action::Cancel), m_actionToolbar),
         }) {
        const int iconSize = m_toolbarAppearance.actionToolbarIconSize;
        button->setIconSize(QSize(iconSize, iconSize));
        actionLayout->addWidget(button);
    }
    m_actionToolbar->hide();
}

void ShotWindow::initializeTransientPanels()
{
    m_openWithPanel = new QWidget(this);
    m_openWithPanel->setObjectName(QStringLiteral("openWithPanel"));
    m_openWithPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *openLayout = new QVBoxLayout(m_openWithPanel);
    openLayout->setContentsMargins(8, 8, 8, 8);
    openLayout->setSpacing(4);
    m_openWithPanel->hide();

    m_extensionPanel = new QWidget(this);
    m_extensionPanel->setObjectName(QStringLiteral("extensionPanel"));
    m_extensionPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *extensionLayout = new QVBoxLayout(m_extensionPanel);
    extensionLayout->setContentsMargins(8, 8, 8, 8);
    extensionLayout->setSpacing(4);
    m_extensionPanel->hide();

    m_colorPalette = new QWidget(this);
    m_colorPalette->setObjectName(QStringLiteral("colorPalette"));
    m_colorPalette->setStyleSheet(markshot::theme::colorPaletteStyleSheet());
    for (const QColor &color : markshot::theme::paletteColors()) {
        auto *button = new QPushButton(m_colorPalette);
        button->setFocusPolicy(Qt::NoFocus);
        button->setStyleSheet(QStringLiteral("background: %1;").arg(color.name()));
        connect(button, &QPushButton::clicked, this, [this, color] { setCurrentColor(color); });
    }
    m_colorPalettePreview = new QWidget(m_colorPalette);
    m_colorPalettePreview->setObjectName(QStringLiteral("colorPalettePreview"));
    m_colorPalette->installEventFilter(this);
    m_colorPalette->hide();
    updateColorPalettePreview();
}

void ShotWindow::initializeTextEditor()
{
    m_textEditor = new QTextEdit(this);
    m_textEditor->setObjectName(QStringLiteral("textEditor"));
    m_textEditor->setPlaceholderText(MS_TR("Type text"));
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(QColor(94, 234, 212), QColor(0, 0, 0, 0), 24));
    m_textEditor->setAcceptRichText(false);
    m_textEditor->setTabChangesFocus(false);
    m_textEditor->setFrameShape(QFrame::NoFrame);
    m_textEditor->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setAttribute(Qt::WA_InputMethodEnabled, true);
    m_textEditor->viewport()->setAutoFillBackground(false);
    m_textEditor->setToolTip(MS_TR("Enter inserts newline, click outside commits, Esc cancels"));
    m_textEditor->hide();
    m_textEditor->installEventFilter(this);
}

void ShotWindow::initializeLaserTimer()
{
    m_laserClock.start();
    m_laserTimer = new QTimer(this);
    m_laserTimer->setInterval(33);
    connect(m_laserTimer, &QTimer::timeout, this, [this] { cleanupLaserStrokes(); });
}

void ShotWindow::initializeWindowDetection(QVector<QRect> windowGeometries, bool enabled)
{
    if (!enabled) {
        return;
    }

    if (windowGeometries.isEmpty()) {
#if defined(Q_OS_WIN)
        windowGeometries = markshot::windows::enumerateWindowGeometries();
#else
        windowGeometries = enumerateX11WindowGeometries();
#endif
    }
    for (const QRect &windowGeometry : std::as_const(windowGeometries)) {
        const QRect imageRect = windowGeometryToImageRect(windowGeometry,
                                                          m_sourceGeometry,
                                                          m_frozenFrame.size());
        if (imageRect.width() > 1 && imageRect.height() > 1) {
            m_windowRects.append(imageRect);
        }
    }
}

bool ShotWindow::configureLayerShell(QScreen *screen)
{
    const QSize desiredSize = m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()
        ? m_sourceGeometry.size()
        : m_frozenFrame.size();
    if (!desiredSize.isEmpty()) {
        resize(desiredSize);
    }

    if (screen) {
        setScreen(screen);
    }

    return markshot::layershell::configureOverlay(
        this,
        screen,
        {QStringLiteral("dock"),
         markshot::layershell::KeyboardInteractivity::Exclusive,
         true,
         true});
}

void ShotWindow::updateLayerShellForIme()
{
    const bool imeActive = m_textEditor && m_textEditor->isVisible();
    markshot::layershell::setOverlayLayer(
        this, imeActive ? markshot::layershell::Layer::Top : markshot::layershell::Layer::Overlay);
    if (imeActive) {
        QTimer::singleShot(0, this, [this]() {
            if (m_textEditor && m_textEditor->isVisible()) {
                if (QInputMethod *im = QGuiApplication::inputMethod()) {
                    im->update(Qt::ImCursorRectangle);
                }
            }
        });
    }
}

void ShotWindow::startFullscreenAnnotation()
{
    enterFullscreenAnnotation(true);
}

void ShotWindow::setImageNavigationEnabled(bool enabled)
{
    m_imageNavigationEnabled = enabled;
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
