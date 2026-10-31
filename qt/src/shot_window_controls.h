#pragma once
#include "shot_window_qt_fwd.h"
#include "capture_toolbar_position.h"
#include "display_capture/display_capture_target.h"
#include "selection_translate/selection_translate_session.h"
#include <QPoint>
#include <QRect>
#include <QRectF>
#include <QVector>
#include <optional>
class QWidget;
namespace markshot::ui { class ColorPicker; class CompactToolbar; }
namespace markshot::display_capture { class DisplayCapturePicker; }
namespace markshot::selection_translate { class TranslateLanguagePopup; class TranslateResultBar; }
/// @brief 【Qt界面】【控件组合】管理截图窗口控件；所有权仍归 Qt 父子对象树
struct ShotWindowControls {
    // Persistent widgets owned by the annotation window. Their geometry is
    // recomputed after selection changes, zoom/pan changes, and panel toggles.
    // m_toolbar / m_actionToolbar 是几何与显隐句柄；按钮、分组、方向与选中态通过对应 Panel 组件访问
    QWidget *m_toolbar = nullptr;
    markshot::ui::CompactToolbar *m_toolbarPanel = nullptr;
    QScrollBar *m_horizontalImageScrollBar = nullptr;
    QScrollBar *m_verticalImageScrollBar = nullptr;
    QWidget *m_actionToolbar = nullptr;
    markshot::ui::CompactToolbar *m_actionToolbarPanel = nullptr;
    QWidget *m_annotationPropertyPanel = nullptr;
    QLabel *m_annotationPropertyTitle = nullptr;
    QLabel *m_propertyWidthLabel = nullptr;
    QSlider *m_propertyWidthSlider = nullptr;
    QLabel *m_propertyOpacityLabel = nullptr;
    QSlider *m_propertyOpacitySlider = nullptr;
    QPushButton *m_propertyColorButton = nullptr;
    QPushButton *m_propertyTextBackgroundButton = nullptr;
    QPushButton *m_propertyFillButton = nullptr;
    QLabel *m_propertyRadiusGlyphLabel = nullptr;
    QLabel *m_propertyRadiusLabel = nullptr;
    QSlider *m_propertyRadiusSlider = nullptr;
    QLabel *m_propertyArrowStyleLabel = nullptr;
    QComboBox *m_propertyArrowStyleCombo = nullptr;
    QComboBox *m_propertyArrowLineStyleCombo = nullptr;
    QComboBox *m_propertyHighlighterStyleCombo = nullptr;
    QComboBox *m_propertyNumberStyleCombo = nullptr;
    QComboBox *m_propertyRectangleStyleCombo = nullptr;
    QComboBox *m_propertyMosaicStyleCombo = nullptr;
    QComboBox *m_propertyRulerModeCombo = nullptr;
    QPushButton *m_propertyResetNumberButton = nullptr;
    QLabel *m_propertyMagnifierScaleGlyphLabel = nullptr;
    QLabel *m_propertyMagnifierScaleLabel = nullptr;
    QSlider *m_propertyMagnifierScaleSlider = nullptr;
    QPushButton *m_propertyMagnifierShapeButton = nullptr;
    QPushButton *m_propertyFontButton = nullptr;
    QWidget *m_propertyFontPanel = nullptr;
    QLineEdit *m_propertyFontSearch = nullptr;
    QListWidget *m_propertyFontList = nullptr;
    QSpinBox *m_propertyFontSizeSpin = nullptr;
    QToolButton *m_propertyFontBoldButton = nullptr;
    QToolButton *m_propertyFontItalicButton = nullptr;
    QPushButton *m_propertyEditTextButton = nullptr;
    QWidget *m_propertyColorDialogPanel = nullptr;
    markshot::ui::ColorPicker *m_propertyColorPicker = nullptr;
    bool m_propertyColorEditHistoryCaptured = false;
    bool m_propertyColorEditingTextBackground = false;
    QWidget *m_openWithPanel = nullptr;
    QWidget *m_extensionPanel = nullptr;
    // 框选翻译的目标语言弹层、译文浮条和翻译模式状态，首次翻译时创建
    markshot::selection_translate::TranslateLanguagePopup *m_translateLanguagePopup = nullptr;
    markshot::selection_translate::TranslateResultBar *m_translateResultBar = nullptr;
    std::optional<markshot::selection_translate::SelectionTranslateSession> m_selectionTranslateSession;
    QWidget *m_startupColorPanel = nullptr;
    markshot::display_capture::DisplayCapturePicker *m_displayCapturePicker = nullptr;
    QVector<markshot::display_capture::Target> m_displayCaptureTargets;
    QWidget *m_colorPalette = nullptr;
    QWidget *m_colorPalettePreview = nullptr;
    QPoint m_colorPaletteAnchor;
    QWidget *m_shapeMarkerPopup = nullptr;
    QPushButton *m_shapeMarkerToolbarButton = nullptr;
    QPoint m_toolbarDragStart;
    QRect m_toolbarBeforeDrag;
    std::optional<QRectF> m_selectionBeforeFullscreenAnnotation;
    // 主工具栏中仅在全屏标注时显示的分组索引
    QVector<int> m_fullscreenActionGroups;
    bool m_toolbarVerticalLayout = false;
    // 两条工具栏的摆放方式，会话启动时从应用配置读入一次（issue #146）
    markshot::CaptureToolbarPosition m_toolbarPosition = markshot::CaptureToolbarPosition::FollowSelection;

};
