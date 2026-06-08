#include "shot_window_module.h"

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
    m_autoSelectAfterDrawByTool = annotationAutoSelectAfterDrawTools();

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
    auto *propertyLayout = new QHBoxLayout(m_annotationPropertyPanel);
    propertyLayout->setContentsMargins(8, 6, 8, 6);
    propertyLayout->setSpacing(6);
    auto addPropertyGlyph = [this, propertyLayout](markshot::ui::PropertyIcon icon, const QString &tooltip) {
        auto *label = new QLabel(m_annotationPropertyPanel);
        label->setObjectName(QStringLiteral("propertyGlyph"));
        label->setAlignment(Qt::AlignCenter);
        label->setPixmap(markshot::ui::makePropertyIcon(icon).pixmap(QSize(18, 18)));
        label->setToolTip(tooltip);
        propertyLayout->addWidget(label);
        return label;
    };
    auto configurePropertyValueLabel = [](QLabel *label, int width, const QString &tooltip) {
        label->setObjectName(QStringLiteral("propertyValue"));
        label->setAlignment(Qt::AlignCenter);
        label->setFixedWidth(width);
        label->setToolTip(tooltip);
    };

    m_annotationPropertyTitle = new QLabel(QStringLiteral("Object"), m_annotationPropertyPanel);
    m_annotationPropertyTitle->setObjectName(QStringLiteral("propertyTitle"));
    m_annotationPropertyTitle->setAlignment(Qt::AlignCenter);
    m_annotationPropertyTitle->setMinimumWidth(58);
    m_annotationPropertyTitle->setToolTip(MS_TR("Selected object type"));
    propertyLayout->addWidget(m_annotationPropertyTitle);
    propertyLayout->addSpacing(2);
    addPropertyGlyph(markshot::ui::PropertyIcon::StrokeWidth, MS_TR("Selected object width or size"));
    m_propertyWidthLabel = new QLabel(QStringLiteral("2"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyWidthLabel, 34, MS_TR("Selected object width or size"));
    propertyLayout->addWidget(m_propertyWidthLabel);
    m_propertyWidthSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyWidthSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyWidthSlider->setFixedWidth(88);
    m_propertyWidthSlider->setToolTip(MS_TR("Selected object width or size"));
    connect(m_propertyWidthSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationWidth(value); });
    propertyLayout->addWidget(m_propertyWidthSlider);
    propertyLayout->addSpacing(2);
    addPropertyGlyph(markshot::ui::PropertyIcon::Opacity, MS_TR("Selected object opacity"));
    m_propertyOpacityLabel = new QLabel(QStringLiteral("100%"), m_annotationPropertyPanel);
    configurePropertyValueLabel(m_propertyOpacityLabel, 36, MS_TR("Selected object opacity"));
    propertyLayout->addWidget(m_propertyOpacityLabel);
    m_propertyOpacitySlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyOpacitySlider->setFocusPolicy(Qt::NoFocus);
    m_propertyOpacitySlider->setRange(0, 100);
    m_propertyOpacitySlider->setFixedWidth(76);
    m_propertyOpacitySlider->setToolTip(MS_TR("Selected object opacity"));
    connect(m_propertyOpacitySlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationOpacity(value); });
    propertyLayout->addWidget(m_propertyOpacitySlider);
    propertyLayout->addSpacing(2);
    m_propertyColorButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyColorButton->setFocusPolicy(Qt::NoFocus);
    m_propertyColorButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Color));
    m_propertyColorButton->setIconSize(QSize(18, 18));
    m_propertyColorButton->setToolTip(MS_TR("Change selected object color"));
    m_propertyColorButton->setAccessibleName(MS_TR("Change selected object color"));
    connect(m_propertyColorButton, &QPushButton::clicked, this, [this] { openSelectedAnnotationColorPalette(); });
    propertyLayout->addWidget(m_propertyColorButton);
    m_propertyTextBackgroundButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyTextBackgroundButton->setFocusPolicy(Qt::NoFocus);
    m_propertyTextBackgroundButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::TextBackground));
    m_propertyTextBackgroundButton->setIconSize(QSize(18, 18));
    m_propertyTextBackgroundButton->setToolTip(MS_TR("Text background color"));
    m_propertyTextBackgroundButton->setAccessibleName(MS_TR("Text background color"));
    connect(m_propertyTextBackgroundButton, &QPushButton::clicked, this, [this] { openSelectedTextBackgroundColorPalette(); });
    propertyLayout->addWidget(m_propertyTextBackgroundButton);
    m_propertyFillButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFillButton->setCheckable(true);
    m_propertyFillButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(false));
    m_propertyFillButton->setIconSize(QSize(20, 20));
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
    m_propertyRadiusSlider->setFixedWidth(72);
    m_propertyRadiusSlider->setToolTip(MS_TR("Rectangle corner radius"));
    connect(m_propertyRadiusSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationCornerRadius(value); });
    propertyLayout->addWidget(m_propertyRadiusSlider);
    m_propertyArrowStyleCombo = new QComboBox(m_annotationPropertyPanel);
    m_propertyArrowStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_propertyArrowStyleCombo->addItem(MS_TR("Fletched"), static_cast<int>(ArrowStyle::Fletched));
    m_propertyArrowStyleCombo->addItem(MS_TR("KDE"), static_cast<int>(ArrowStyle::Kde));
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
    m_propertyResetNumberButton->setIconSize(QSize(20, 20));
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
    m_propertyMagnifierScaleSlider->setFixedWidth(84);
    m_propertyMagnifierScaleSlider->setToolTip(MS_TR("Magnifier scale"));
    connect(m_propertyMagnifierScaleSlider, &QSlider::valueChanged, this, [this](int value) {
        setSelectedMagnifierScale(value);
    });
    propertyLayout->addWidget(m_propertyMagnifierScaleSlider);
    m_propertyFontButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFontButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFontButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::Font));
    m_propertyFontButton->setIconSize(QSize(20, 20));
    m_propertyFontButton->setToolTip(MS_TR("Text font"));
    m_propertyFontButton->setAccessibleName(MS_TR("Text font"));
    connect(m_propertyFontButton, &QPushButton::clicked, this, [this] { toggleSelectedTextFontPanel(); });
    propertyLayout->addWidget(m_propertyFontButton);
    m_propertyEditTextButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyEditTextButton->setFocusPolicy(Qt::NoFocus);
    m_propertyEditTextButton->setIcon(markshot::ui::makePropertyIcon(markshot::ui::PropertyIcon::EditText));
    m_propertyEditTextButton->setIconSize(QSize(20, 20));
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
    m_toolbar->setStyleSheet(markshot::theme::panelStyleSheet());
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
                          Action::Cancel}) {
        const QString shortcut = action == Action::OpenWith ? shortcutText(action, QStringLiteral("Open"))
            : action == Action::Extensions           ? shortcutText(action, QStringLiteral("Ext"))
            : action == Action::ScrollCapture        ? shortcutText(action, QStringLiteral("Scroll"))
            : action == Action::Pin                  ? shortcutText(action)
            : action == Action::OcrCopy              ? shortcutText(action, QStringLiteral("OCR"))
            : action == Action::Copy                 ? shortcutText(action)
            : action == Action::Save                 ? shortcutText(action, QStringLiteral("Save As"))
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
    m_actionToolbar->setStyleSheet(m_toolbar->styleSheet()
        + QStringLiteral(
              "QWidget#actionToolbar QPushButton {"
              " border-radius: 6px;"
              " min-width: 28px;"
              " min-height: 28px;"
              " max-width: 28px;"
              " max-height: 28px;"
              "}"));
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
             addToolbarButton(Action::Cancel, shortcutText(Action::Cancel), m_actionToolbar),
         }) {
        button->setIconSize(QSize(20, 20));
        actionLayout->addWidget(button);
    }
    m_actionToolbar->hide();
}

void ShotWindow::initializeShortcuts()
{
    auto shortcutBlockedByTextInput = [this] {
        if (m_textEditor && m_textEditor->isVisible()) {
            return true;
        }
        QWidget *focusWidget = QApplication::focusWidget();
        return qobject_cast<QLineEdit *>(focusWidget) != nullptr
            || qobject_cast<QTextEdit *>(focusWidget) != nullptr;
    };
    auto addPlainShortcut = [this, shortcutBlockedByTextInput](const QKeySequence &sequence, auto callback) {
        if (sequence.isEmpty()) {
            return;
        }
        auto *shortcut = new QShortcut(sequence, this);
        shortcut->setContext(Qt::WindowShortcut);
        shortcut->setAutoRepeat(false);
        connect(shortcut, &QShortcut::activated, this, [this, shortcutBlockedByTextInput, callback] {
            if (shortcutBlockedByTextInput()) {
                return;
            }
            callback();
        });
    };
    auto addToolShortcut = [this, addPlainShortcut](Tool tool) {
        const QKeySequence sequence = this->shortcutForTool(tool);
        addPlainShortcut(sequence, [this, tool, sequence] {
            if (m_mode == Mode::Selecting && sequence == m_startupColorPickerShortcut) {
                setStartupTool(StartupTool::ColorPicker);
                return;
            }
            if (m_mode == Mode::Selecting && sequence == m_startupRulerShortcut) {
                setStartupTool(StartupTool::Ruler);
                return;
            }
            setTool(tool);
        });
    };
    for (Tool tool : {Tool::Move,
                      Tool::Select,
                      Tool::Pen,
                      Tool::Line,
                      Tool::Highlighter,
                      Tool::Rectangle,
                      Tool::Ellipse,
                      Tool::Arrow,
                      Tool::Text,
                      Tool::Number,
                      Tool::Mosaic,
                      Tool::Magnifier,
                      Tool::Laser}) {
        addToolShortcut(tool);
    }
    auto sequenceUsedByTool = [this](const QKeySequence &sequence) {
        for (const QKeySequence &toolSequence : m_toolShortcuts) {
            if (!sequence.isEmpty() && sequence == toolSequence) {
                return true;
            }
        }
        return false;
    };
    if (!sequenceUsedByTool(m_startupColorPickerShortcut)) {
        addPlainShortcut(m_startupColorPickerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::ColorPicker);
            }
        });
    }
    if (!sequenceUsedByTool(m_startupRulerShortcut)) {
        addPlainShortcut(m_startupRulerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::Ruler);
            }
        });
    }
    auto addActionShortcut = [this, addPlainShortcut](Action action, auto callback) {
        addPlainShortcut(this->shortcutForAction(action), callback);
    };
    addActionShortcut(Action::ToggleCaptureScope, [this] { toggleCaptureScope(); });
    addActionShortcut(Action::Pin, [this] { pinSelection(); });
    addActionShortcut(Action::Copy, [this] {
        commitTextEditor();
        copySelection();
    });
    addActionShortcut(Action::Save, [this] {
        commitTextEditor();
        saveSelection();
    });
    addActionShortcut(Action::Undo, [this] { undoAnnotationEdit(); });
    addActionShortcut(Action::Redo, [this] { redoAnnotation(); });
    addActionShortcut(Action::OpenWith, [this] { toggleOpenWithPanel(); });
    addActionShortcut(Action::Extensions, [this] { toggleExtensionPanel(); });
    addActionShortcut(Action::ScrollCapture, [this] { startScrollCapture(); });
    addActionShortcut(Action::OcrCopy, [this] { ocrCopySelection(); });
    addActionShortcut(Action::Clear, [this] { clearAnnotations(); });
    addActionShortcut(Action::ToggleToolbarLayout, [this] { toggleToolbarLayout(); });
    addActionShortcut(Action::Cancel, [this] {
        if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
            leaveStartupTool();
            return;
        }
        emit sessionCancelRequested();
        close();
    });
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
        {QStringLiteral("mark-shot"),
         markshot::layershell::KeyboardInteractivity::Exclusive,
         true,
         true});
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

QKeySequence ShotWindow::shortcutForAction(Action action) const
{
    return m_actionShortcuts.at(shortcuts::actionIndex(action));
}

QKeySequence ShotWindow::shortcutForTool(Tool tool) const
{
    return m_toolShortcuts.at(shortcuts::toolIndex(tool));
}

QString ShotWindow::shortcutText(Action action, const QString &fallback) const
{
    const QKeySequence sequence = shortcutForAction(action);
    if (sequence.isEmpty()) {
        return fallback;
    }
    return sequence.toString(QKeySequence::NativeText);
}

QString ShotWindow::shortcutText(Tool tool) const
{
    const QKeySequence sequence = shortcutForTool(tool);
    return sequence.isEmpty() ? QString() : sequence.toString(QKeySequence::NativeText);
}

/// @brief Checks if a given key sequence matches a key event.
/// @param sequence The key sequence to match against.
/// @param event The keyboard event to check.
/// @return True if the key event matches the sequence, false otherwise.
bool shortcutMatchesEvent(const QKeySequence &sequence, const QKeyEvent *event)
{
    if (!event || sequence.isEmpty() || event->key() == Qt::Key_unknown) {
        return false;
    }
    return QKeySequence(event->keyCombination()).matches(sequence) == QKeySequence::ExactMatch;
}

bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Action action) const
{
    return shortcutMatchesEvent(shortcutForAction(action), event);
}

bool ShotWindow::eventMatchesShortcut(const QKeyEvent *event, Tool tool) const
{
    return shortcutMatchesEvent(shortcutForTool(tool), event);
}

bool ShotWindow::eventMatchesStartupShortcut(const QKeyEvent *event, StartupTool tool) const
{
    if (tool == StartupTool::ColorPicker) {
        return shortcutMatchesEvent(m_startupColorPickerShortcut, event);
    }
    if (tool == StartupTool::Ruler) {
        return shortcutMatchesEvent(m_startupRulerShortcut, event);
    }
    return false;
}

bool ShotWindow::handleConfiguredActionShortcut(QKeyEvent *event)
{
    if (eventMatchesShortcut(event, Action::Cancel)) {
        emit sessionCancelRequested();
        close();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Copy)) {
        commitTextEditor();
        copySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Save)) {
        commitTextEditor();
        saveSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Pin)) {
        pinSelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Undo)) {
        undoAnnotationEdit();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Redo) || event->matches(QKeySequence::Redo)) {
        redoAnnotation();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleCaptureScope)) {
        toggleCaptureScope();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ToggleToolbarLayout)) {
        toggleToolbarLayout();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OpenWith)) {
        toggleOpenWithPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Extensions)) {
        toggleExtensionPanel();
        return true;
    }
    if (eventMatchesShortcut(event, Action::ScrollCapture)) {
        startScrollCapture();
        return true;
    }
    if (eventMatchesShortcut(event, Action::OcrCopy)) {
        ocrCopySelection();
        return true;
    }
    if (eventMatchesShortcut(event, Action::Clear)) {
        clearAnnotations();
        return true;
    }
    return false;
}

bool ShotWindow::handleConfiguredToolShortcut(QKeyEvent *event)
{
    const std::array<Tool, static_cast<int>(Tool::Laser) + 1> tools = {
        Tool::Move,
        Tool::Select,
        Tool::Pen,
        Tool::Line,
        Tool::Highlighter,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Arrow,
        Tool::Text,
        Tool::Number,
        Tool::Mosaic,
        Tool::Magnifier,
        Tool::Laser,
    };
    for (Tool tool : tools) {
        if (eventMatchesShortcut(event, tool)) {
            setTool(tool);
            return true;
        }
    }
    return false;
}
