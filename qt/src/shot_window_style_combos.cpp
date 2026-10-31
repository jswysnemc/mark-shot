#include "shot_window_module.h"
#include "shot_window_ruler_tool.h"

#include "ui/compact_combo_box.h"

using namespace markshot::shot;

void ShotWindow::initializePropertyStyleCombos(QHBoxLayout *propertyLayout)
{
    // 1. 矩形风格切换:描边/高亮/反色/聚光灯,仅在 Tool::Rectangle 选中或激活时显示
    m_controls.m_propertyRectangleStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyRectangleStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyRectangleStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyRectangleStyleCombo->addItem(MS_TR("Stroke"), static_cast<int>(RectangleStyle::Stroke));
    m_controls.m_propertyRectangleStyleCombo->addItem(MS_TR("Highlight"), static_cast<int>(RectangleStyle::Highlight));
    m_controls.m_propertyRectangleStyleCombo->addItem(MS_TR("Invert"), static_cast<int>(RectangleStyle::Invert));
    m_controls.m_propertyRectangleStyleCombo->addItem(MS_TR("Spotlight"), static_cast<int>(RectangleStyle::Spotlight));
    m_controls.m_propertyRectangleStyleCombo->setToolTip(MS_TR("Rectangle style"));
    m_controls.m_propertyRectangleStyleCombo->setAccessibleName(MS_TR("Rectangle style"));
    connect(m_controls.m_propertyRectangleStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_controls.m_propertyRectangleStyleCombo) {
            return;
        }
        setSelectedRectangleStyle(
            static_cast<RectangleStyle>(m_controls.m_propertyRectangleStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_controls.m_propertyRectangleStyleCombo);
    // 2. 马赛克滤镜切换：像素化/模糊/灰度/反色/提亮，仅在 Tool::Mosaic 选中或激活时显示
    m_controls.m_propertyMosaicStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyMosaicStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyMosaicStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyMosaicStyleCombo->addItem(MS_TR("Pixelate"), static_cast<int>(MosaicStyle::Pixelate));
    m_controls.m_propertyMosaicStyleCombo->addItem(MS_TR("Blur"), static_cast<int>(MosaicStyle::Blur));
    m_controls.m_propertyMosaicStyleCombo->addItem(MS_TR("Grayscale"), static_cast<int>(MosaicStyle::Grayscale));
    m_controls.m_propertyMosaicStyleCombo->addItem(MS_TR("Invert"), static_cast<int>(MosaicStyle::Invert));
    m_controls.m_propertyMosaicStyleCombo->addItem(MS_TR("Brighten"), static_cast<int>(MosaicStyle::Brighten));
    m_controls.m_propertyMosaicStyleCombo->setToolTip(MS_TR("Filter effect"));
    m_controls.m_propertyMosaicStyleCombo->setAccessibleName(MS_TR("Filter effect"));
    connect(m_controls.m_propertyMosaicStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_controls.m_propertyMosaicStyleCombo) {
            return;
        }
        setSelectedMosaicStyle(static_cast<MosaicStyle>(m_controls.m_propertyMosaicStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_controls.m_propertyMosaicStyleCombo);
    // 3. 箭头样式
    m_controls.m_propertyArrowStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyArrowStyleCombo->setObjectName(QStringLiteral("annotationArrowStyle"));
    m_controls.m_propertyArrowStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyArrowStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyArrowStyleCombo->addItem(MS_TR("Fletched"), static_cast<int>(ArrowStyle::Fletched));
    m_controls.m_propertyArrowStyleCombo->addItem(MS_TR("KDE"), static_cast<int>(ArrowStyle::Kde));
    m_controls.m_propertyArrowStyleCombo->addItem(MS_TR("Double fletched"), static_cast<int>(ArrowStyle::BidirectionalFletched));
    m_controls.m_propertyArrowStyleCombo->addItem(MS_TR("Double KDE"), static_cast<int>(ArrowStyle::BidirectionalKde));
    m_controls.m_propertyArrowStyleCombo->setToolTip(MS_TR("Arrow style"));
    m_controls.m_propertyArrowStyleCombo->setAccessibleName(MS_TR("Arrow style"));
    connect(m_controls.m_propertyArrowStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_controls.m_propertyArrowStyleCombo) {
            return;
        }
        setSelectedAnnotationArrowStyle(static_cast<ArrowStyle>(m_controls.m_propertyArrowStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_controls.m_propertyArrowStyleCombo);
    m_controls.m_propertyArrowLineStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyArrowLineStyleCombo->setObjectName(QStringLiteral("annotationArrowLineStyle"));
    m_controls.m_propertyArrowLineStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyArrowLineStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyArrowLineStyleCombo->addItem(MS_TR("Solid"), static_cast<int>(ArrowLineStyle::Solid));
    m_controls.m_propertyArrowLineStyleCombo->addItem(MS_TR("Dashed"), static_cast<int>(ArrowLineStyle::Dashed));
    m_controls.m_propertyArrowLineStyleCombo->addItem(MS_TR("Dotted"), static_cast<int>(ArrowLineStyle::Dotted));
    m_controls.m_propertyArrowLineStyleCombo->setToolTip(MS_TR("Arrow line style"));
    m_controls.m_propertyArrowLineStyleCombo->setAccessibleName(MS_TR("Arrow line style"));
    connect(m_controls.m_propertyArrowLineStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index >= 0) {
            setSelectedArrowLineStyle(static_cast<ArrowLineStyle>(m_controls.m_propertyArrowLineStyleCombo->itemData(index).toInt()));
        }
    });
    propertyLayout->addWidget(m_controls.m_propertyArrowLineStyleCombo);
    // 4. 荧光笔样式
    m_controls.m_propertyHighlighterStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyHighlighterStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyHighlighterStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyHighlighterStyleCombo->addItem(MS_TR("Pen"), static_cast<int>(HighlighterStyle::Freehand));
    m_controls.m_propertyHighlighterStyleCombo->addItem(MS_TR("Line"), static_cast<int>(HighlighterStyle::StraightLine));
    m_controls.m_propertyHighlighterStyleCombo->setToolTip(MS_TR("Highlighter style"));
    m_controls.m_propertyHighlighterStyleCombo->setAccessibleName(MS_TR("Highlighter style"));
    connect(m_controls.m_propertyHighlighterStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_controls.m_propertyHighlighterStyleCombo) {
            return;
        }
        setSelectedHighlighterStyle(
            static_cast<HighlighterStyle>(m_controls.m_propertyHighlighterStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_controls.m_propertyHighlighterStyleCombo);
    // 5. 序号样式
    m_controls.m_propertyNumberStyleCombo = new markshot::ui::CompactComboBox(m_controls.m_annotationPropertyPanel);
    m_controls.m_propertyNumberStyleCombo->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyNumberStyleCombo->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("1, 2, 3"), static_cast<int>(NumberStyle::Arabic));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("A, B, C"), static_cast<int>(NumberStyle::UpperAlpha));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("a, b, c"), static_cast<int>(NumberStyle::LowerAlpha));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("I, II, III"), static_cast<int>(NumberStyle::UpperRoman));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("i, ii, iii"), static_cast<int>(NumberStyle::LowerRoman));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("甲, 乙, 丙"), static_cast<int>(NumberStyle::HeavenlyStem));
    m_controls.m_propertyNumberStyleCombo->addItem(MS_TR("一, 二, 三"), static_cast<int>(NumberStyle::Chinese));
    m_controls.m_propertyNumberStyleCombo->setToolTip(MS_TR("Number style"));
    m_controls.m_propertyNumberStyleCombo->setAccessibleName(MS_TR("Number style"));
    connect(m_controls.m_propertyNumberStyleCombo, QOverload<int>::of(&QComboBox::activated), this, [this](int index) {
        if (index < 0 || !m_controls.m_propertyNumberStyleCombo) {
            return;
        }
        setSelectedNumberStyle(static_cast<NumberStyle>(m_controls.m_propertyNumberStyleCombo->itemData(index).toInt()));
    });
    propertyLayout->addWidget(m_controls.m_propertyNumberStyleCombo);
    // 6. 尺子测量方式
    ShotWindowRulerTool(*this).initializePanel(propertyLayout);
    // 7. 属性栏下拉框统一跟随工具栏配色
    const auto combos = m_controls.m_annotationPropertyPanel->findChildren<markshot::ui::CompactComboBox *>();
    for (markshot::ui::CompactComboBox *combo : combos) {
        combo->setToolbarPalette(m_toolbarAppearance.palette);
    }
}
