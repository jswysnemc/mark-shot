#include "shot_window_module.h"

#include "annotation/text_size_limits.h"

#include "text_annotation_layout.h"
#include "ui/recent_font_store.h"
#include "ui/toolbar_panel_style.h"

#include <QColor>
#include <QFontDatabase>
#include <QIcon>
#include <QLabel>
#include <QPainter>
#include <QPixmap>

using namespace markshot::shot::text_size;

namespace {

/**
 * 修改文字样式，并让修改前的单行文字保持单行。
 * @param annotation 文字标注。
 * @param applyStyle 修改字体样式的函数。
 * @return 无返回值。
 */
template <typename ApplyStyle>
void applyTextStyleKeepingLines(markshot::shot::types::Annotation &annotation, ApplyStyle applyStyle)
{
    const bool wrapped = markshot::shot::textAnnotationWraps(annotation);
    applyStyle(annotation);
    if (!wrapped) {
        markshot::shot::fitTextAnnotationRect(annotation);
    }
}

}  // namespace

/**
 * 设置选中文字标注或文字工具默认使用的字体系列。
 * @param fontFamily 字体系列名称。
 * @return 无返回值。
 */
void ShotWindow::setSelectedTextFontFamily(const QString &fontFamily)
{
    if (fontFamily.isEmpty()) {
        return;
    }
    rememberRecentTextFontIfEnabled(fontFamily);

    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Text || annotation->fontFamily == fontFamily) {
            return;
        }
        pushHistorySnapshot();
        applyTextStyleKeepingLines(*annotation, [&fontFamily](Annotation &item) { item.fontFamily = fontFamily; });
    } else {
        if (m_tool != Tool::Text || m_textFontFamily == fontFamily) {
            return;
        }
        m_textFontFamily = fontFamily;
        if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
            QFont font = m_textEditor->font();
            font.setFamily(m_textFontFamily);
            m_textEditor->setFont(font);
        }
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
    applyEditorDisplayFontIfEnabled();
}

/**
 * 设置选中文字标注或文字工具默认使用的精确字号。
 * @param pointSize 最终渲染字号，范围为 8 至 300 点。
 * @return 无返回值。
 */
void ShotWindow::setSelectedTextFontSize(int pointSize)
{
    pointSize = std::clamp(pointSize, kMinTextPointSize, kMaxTextPointSize);
    if (m_controls.m_propertyFontSizeSpin) {
        const QSignalBlocker blocker(m_controls.m_propertyFontSizeSpin);
        m_controls.m_propertyFontSizeSpin->setValue(pointSize);
    }

    // 1. 标注宽度字段保存相对于 19 点的字号偏移
    const int targetWidth = pointSize - kTextPointSizeOffset;
    setSelectedAnnotationWidth(targetWidth);

    // 2. 内联编辑器立即使用相同字号
    if (m_textEditor && m_textEditor->isVisible()) {
        QFont font = m_textEditor->font();
        font.setPointSize(pointSize);
        m_textEditor->setFont(font);
        applySnowTextEditorGeometryIfEnabled();
    }
    applyEditorDisplayFontIfEnabled();
    update();
}

/**
 * 切换选中文字标注或文字工具默认样式的粗体状态。
 * @param bold 是否使用半粗体字重。
 * @return 无返回值。
 */
void ShotWindow::setSelectedTextBold(bool bold)
{
    const QFont::Weight targetWeight = bold ? QFont::DemiBold : QFont::Normal;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Text && annotation->fontWeight != targetWeight) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Text) {
                applyTextStyleKeepingLines(*annotation, [targetWeight](Annotation &item) {
                    item.fontWeight = targetWeight;
                });
            }
        }
    } else {
        if (m_textWeight == targetWeight) {
            return;
        }
        m_textWeight = targetWeight;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QFont font = m_textEditor->font();
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                font.setWeight(annotation->fontWeight);
                font.setItalic(annotation->textItalic);
            }
        } else {
            font.setWeight(m_textWeight);
            font.setItalic(m_textItalic);
        }
        m_textEditor->setFont(font);
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
    applyEditorDisplayFontIfEnabled();
}

/**
 * 切换选中文字标注或文字工具默认样式的斜体状态。
 * @param italic 是否使用斜体。
 * @return 无返回值。
 */
void ShotWindow::setSelectedTextItalic(bool italic)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Text && annotation->textItalic != italic) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Text) {
                applyTextStyleKeepingLines(*annotation, [italic](Annotation &item) { item.textItalic = italic; });
            }
        }
    } else {
        if (m_textItalic == italic) {
            return;
        }
        m_textItalic = italic;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QFont font = m_textEditor->font();
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                font.setWeight(annotation->fontWeight);
                font.setItalic(annotation->textItalic);
            }
        } else {
            font.setWeight(m_textWeight);
            font.setItalic(m_textItalic);
        }
        m_textEditor->setFont(font);
    }
    updateAnnotationPropertyPanel();
    update();
    persistAnnotationState();
    applyEditorDisplayFontIfEnabled();
}

namespace {

constexpr int kRecentFontRowRole = Qt::UserRole + 1;

enum class RecentFontRowKind : int {
    AllHeader = 1,
    RecentFont = 2,
};

/**
 * 生成字体列表左侧的空白占位，让字体名称对齐。
 * @return 透明图标。
 */
QIcon fontListBlankIcon()
{
    QPixmap pixmap(14, 14);
    pixmap.fill(Qt::transparent);
    return QIcon(pixmap);
}

/**
 * 生成当前字体的勾选标记。
 * @param ink 勾选标记颜色，与字体面板前景一致。
 * @return 勾选图标。
 */
QIcon fontListCheckIcon(const QColor &ink)
{
    QPixmap pixmap(14, 14);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(ink, 1.8, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawLine(QPointF(2.2, 7.2), QPointF(5.4, 10.6));
    painter.drawLine(QPointF(5.4, 10.6), QPointF(11.6, 3.2));
    return QIcon(pixmap);
}

}  // namespace

/**
 * 记录用户选中的字体系列。开关关闭时直接返回。
 * @param fontFamily 字体系列名称。
 * @return 无返回值。
 */
void ShotWindow::rememberRecentTextFontIfEnabled(const QString &fontFamily)
{
    if (!markshot::ui::FEATURE_RECENT_TEXT_FONTS_ENABLED) {
        return;
    }
    markshot::ui::rememberRecentFont(fontFamily);
}

/**
 * 【字体弹层】【最近字体】在列表顶部刷新最近使用分区。开关关闭时直接返回。
 * @return 无返回值。
 */
void ShotWindow::refreshRecentTextFontsIfEnabled()
{
    if (!markshot::ui::FEATURE_RECENT_TEXT_FONTS_ENABLED) {
        return;
    }
    if (!m_controls.m_propertyFontList) {
        return;
    }
    // 1. 【界面内存】【字体图标】整份列表共用两个图标，不为每个字体重复分配像素和图标对象
    if (m_controls.m_propertyFontBlankIcon.isNull()) m_controls.m_propertyFontBlankIcon = fontListBlankIcon();
    if (m_controls.m_propertyFontCheckIcon.isNull()
        || m_controls.m_propertyFontCheckInk != m_toolbarAppearance.palette.text) {
        m_controls.m_propertyFontCheckInk = m_toolbarAppearance.palette.text;
        m_controls.m_propertyFontCheckIcon = fontListCheckIcon(m_controls.m_propertyFontCheckInk);
    }
    const QIcon &blankIcon = m_controls.m_propertyFontBlankIcon;
    const QIcon &checkIcon = m_controls.m_propertyFontCheckIcon;

    // 2. 【字体弹层】【最近字体】移除上一次插入的分区，完整字体列表保持原顺序
    for (int index = m_controls.m_propertyFontList->count() - 1; index >= 0; --index) {
        QListWidgetItem *item = m_controls.m_propertyFontList->item(index);
        if (item->data(kRecentFontRowRole).toInt() == 0) {
            continue;
        }
        delete m_controls.m_propertyFontList->takeItem(index);
    }

    // 3. 【字体弹层】【最近字体】最近使用放在顶部，每项用该字体绘制名称
    QString currentFamily = m_textFontFamily;
    if (m_selectedAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_selectedAnnotationId);
            annotation && annotation->tool == Tool::Text && !annotation->fontFamily.isEmpty()) {
            currentFamily = annotation->fontFamily;
        }
    }
    if (currentFamily.isEmpty()) {
        currentFamily = markshot::theme::textFontFamily();
    }
    const QStringList recent = markshot::ui::visibleRecentFonts(
        markshot::ui::readRecentFonts(), QFontDatabase::families());
    bool currentInRecent = false;
    for (int index = recent.size() - 1; index >= 0; --index) {
        const QString family = recent.at(index);
        auto *item = new QListWidgetItem(family);
        item->setData(Qt::UserRole, family);
        item->setData(kRecentFontRowRole, static_cast<int>(RecentFontRowKind::RecentFont));
        item->setToolTip(family);
        item->setFont(QFont(family, 13));
        item->setIcon(family == currentFamily ? checkIcon : blankIcon);
        currentInRecent = currentInRecent || family == currentFamily;
        m_controls.m_propertyFontList->insertItem(0, item);
    }

    // 4. 【字体弹层】【分区标题】有最近使用时，用「所有系统字体」把完整列表隔开
    if (!recent.isEmpty()) {
        const int rowHeight = std::max(28, m_controls.m_propertyFontList->sizeHintForRow(0));
        auto *header = new QListWidgetItem;
        header->setData(kRecentFontRowRole, static_cast<int>(RecentFontRowKind::AllHeader));
        header->setFlags(Qt::ItemIsEnabled);
        header->setSizeHint(QSize(1, rowHeight));
        m_controls.m_propertyFontList->insertItem(recent.size(), header);
        auto *label = new QLabel(MS_TR("All system fonts"), m_controls.m_propertyFontList);
        label->setStyleSheet(QStringLiteral(
            "QLabel { color: %1; background: transparent; font-size: 11px; font-weight: 600; padding: 8px 8px 2px 8px; }")
                .arg(markshot::ui::cssColor(m_toolbarAppearance.palette.mutedText)));
        label->setAttribute(Qt::WA_TransparentForMouseEvents, true);
        m_controls.m_propertyFontList->setIndexWidget(
            m_controls.m_propertyFontList->model()->index(recent.size(), 0), label);
    }

    // 5. 【字体弹层】【当前字体】系统字体与最近使用左对齐，当前字体只在最近使用里打勾
    for (int index = recent.size(); index < m_controls.m_propertyFontList->count(); ++index) {
        QListWidgetItem *item = m_controls.m_propertyFontList->item(index);
        if (item->data(kRecentFontRowRole).toInt() != 0) {
            continue;
        }
        const QString family = item->data(Qt::UserRole).toString();
        item->setFont(QFont(family, 13));
        const bool checked = !currentInRecent && family == currentFamily;
        item->setIcon(checked ? checkIcon : blankIcon);
        if (checked) {
            currentInRecent = true;
        }
    }
}

/**
 * 点击最近使用标题时不关闭字体面板。
 * @param item 被点击的列表项。
 * @return 该行是分区标题时返回 true。
 */
bool ShotWindow::ignoreRecentFontHeaderClickIfEnabled(QListWidgetItem *item) const
{
    if (!markshot::ui::FEATURE_RECENT_TEXT_FONTS_ENABLED || !item) {
        return false;
    }
    return item->data(kRecentFontRowRole).toInt() == static_cast<int>(RecentFontRowKind::AllHeader);
}
