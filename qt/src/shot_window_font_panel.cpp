#include "shot_window_module.h"
#include "annotation/text_size_limits.h"
#include "ui/toolbar_panel_style.h"

#include <QFontDatabase>
#include <QLineEdit>
#include <QStyledItemDelegate>

using namespace markshot::shot::text_size;

namespace {
constexpr int kFontPanelControlHeight = 26;

/** 【字体弹层】【列表布局】统一行高，只在绘制可见行时解析预览字体，避免尺寸计算加载所有字体。 */
class FontPreviewDelegate final : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

    /** 【字体弹层】【行尺寸】option/index 为列表布局请求，返回随逻辑 DPI 缩放的统一行尺寸，不测量字体预览。 */
    QSize sizeHint(const QStyleOptionViewItem &option, const QModelIndex &) const override
    {
        const qreal dpi = option.widget ? option.widget->logicalDpiY() : 96.0;
        return QSize(1, std::max(28, qRound(28.0 * dpi / 96.0)));
    }
};
}  // namespace

/**
 * 【界面内存】【字体弹层】首次使用时创建字体、字号和样式控件；后续调用复用面板。
 * @return 无返回值。
 */
void ShotWindow::initializePropertyFontPanel()
{
    if (m_controls.m_propertyFontPanel) return;
    m_controls.m_propertyFontPanel = new QWidget(this);
    m_controls.m_propertyFontPanel->setObjectName(QStringLiteral("propertyFontPanel"));
    m_controls.m_propertyFontPanel->setCursor(Qt::ArrowCursor);
    m_controls.m_propertyFontPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet(m_toolbarAppearance.palette));
    auto *fontPanelLayout = new QVBoxLayout(m_controls.m_propertyFontPanel);
    fontPanelLayout->setContentsMargins(6, 6, 6, 6);
    // 列表与底部控件行之间保留固定间距，高度由 updatePropertyFontPanelGeometry 统一分配
    fontPanelLayout->setSpacing(6);

    // 1. 【字体弹层】【搜索】搜索框按名称过滤，避免在整份字体列表里滚动查找
    m_controls.m_propertyFontSearch = new QLineEdit(m_controls.m_propertyFontPanel);
    m_controls.m_propertyFontSearch->setFixedHeight(kFontPanelControlHeight);
    m_controls.m_propertyFontSearch->setClearButtonEnabled(true);
    m_controls.m_propertyFontSearch->setPlaceholderText(MS_TR("Search fonts"));
    m_controls.m_propertyFontSearch->setAccessibleName(MS_TR("Search fonts"));
    fontPanelLayout->addWidget(m_controls.m_propertyFontSearch);

    // 2. 【字体弹层】【列表】仅在用户请求字体选择时枚举系统字体
    m_controls.m_propertyFontList = new QListWidget(m_controls.m_propertyFontPanel);
    m_controls.m_propertyFontList->setItemDelegate(new FontPreviewDelegate(m_controls.m_propertyFontList));
    m_controls.m_propertyFontList->setFocusPolicy(Qt::NoFocus);
    m_controls.m_propertyFontList->setUniformItemSizes(true);
    m_controls.m_propertyFontList->setVerticalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    m_controls.m_propertyFontList->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_controls.m_propertyFontList->setTextElideMode(Qt::ElideRight);
    m_controls.m_propertyFontList->setIconSize(QSize(14, 14));
    for (const QString &family : QFontDatabase::families()) {
        auto *item = new QListWidgetItem(family, m_controls.m_propertyFontList);
        item->setData(Qt::UserRole, family);
        item->setToolTip(family);
        item->setFont(QFont(family, 13));
    }
    connect(m_controls.m_propertyFontSearch, &QLineEdit::textChanged, this, [this](const QString &query) {
        const QString needle = query.trimmed();
        for (int index = 0; index < m_controls.m_propertyFontList->count(); ++index) {
            QListWidgetItem *item = m_controls.m_propertyFontList->item(index);
            const QString family = item->data(Qt::UserRole).toString();
            item->setHidden(!needle.isEmpty() && !family.contains(needle, Qt::CaseInsensitive));
        }
    });
    connect(m_controls.m_propertyFontList, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item || ignoreRecentFontHeaderClickIfEnabled(item)) {
            return;
        }
        setSelectedTextFontFamily(item->data(Qt::UserRole).toString());
        m_controls.m_propertyFontPanel->hide();
    });
    fontPanelLayout->addWidget(m_controls.m_propertyFontList, 1);

    // 3. 【字体弹层】【字号】底部控件行：字号输入框与 B/I 按钮同排，固定高度，不参与列表伸缩
    auto *fontStyleLayout = new QHBoxLayout;
    fontStyleLayout->setContentsMargins(0, 0, 0, 0);
    fontStyleLayout->setSpacing(4);
    m_controls.m_propertyFontSizeSpin = new QSpinBox(m_controls.m_propertyFontPanel);
    m_controls.m_propertyFontSizeSpin->setFixedHeight(kFontPanelControlHeight);
    m_controls.m_propertyFontSizeSpin->setRange(kMinTextPointSize, kMaxTextPointSize);
    m_controls.m_propertyFontSizeSpin->setSingleStep(1);
    m_controls.m_propertyFontSizeSpin->setSuffix(QStringLiteral(" pt"));
    m_controls.m_propertyFontSizeSpin->setFocusPolicy(Qt::StrongFocus);
    m_controls.m_propertyFontSizeSpin->setToolTip(MS_TR("Text font size in points"));
    connect(m_controls.m_propertyFontSizeSpin, &QSpinBox::valueChanged, this, [this](int value) {
        setSelectedTextFontSize(value);
    });
    fontStyleLayout->addWidget(m_controls.m_propertyFontSizeSpin, 1);

    // 4. 【字体弹层】【样式】创建粗体和斜体切换控件
    m_controls.m_propertyFontBoldButton = new QToolButton(m_controls.m_propertyFontPanel);
    m_controls.m_propertyFontBoldButton->setFixedSize(kFontPanelControlHeight, kFontPanelControlHeight);
    m_controls.m_propertyFontBoldButton->setCheckable(true);
    m_controls.m_propertyFontBoldButton->setText(QStringLiteral("B"));
    m_controls.m_propertyFontBoldButton->setToolTip(MS_TR("Bold"));
    m_controls.m_propertyFontBoldButton->setFocusPolicy(Qt::NoFocus);
    QFont boldButtonFont = m_controls.m_propertyFontBoldButton->font();
    boldButtonFont.setBold(true);
    m_controls.m_propertyFontBoldButton->setFont(boldButtonFont);
    connect(m_controls.m_propertyFontBoldButton, &QToolButton::toggled, this, [this](bool checked) {
        setSelectedTextBold(checked);
    });
    fontStyleLayout->addWidget(m_controls.m_propertyFontBoldButton);

    m_controls.m_propertyFontItalicButton = new QToolButton(m_controls.m_propertyFontPanel);
    m_controls.m_propertyFontItalicButton->setFixedSize(kFontPanelControlHeight, kFontPanelControlHeight);
    m_controls.m_propertyFontItalicButton->setCheckable(true);
    m_controls.m_propertyFontItalicButton->setText(QStringLiteral("I"));
    m_controls.m_propertyFontItalicButton->setToolTip(MS_TR("Italic"));
    m_controls.m_propertyFontItalicButton->setFocusPolicy(Qt::NoFocus);
    QFont italicButtonFont = m_controls.m_propertyFontItalicButton->font();
    italicButtonFont.setItalic(true);
    m_controls.m_propertyFontItalicButton->setFont(italicButtonFont);
    connect(m_controls.m_propertyFontItalicButton, &QToolButton::toggled, this, [this](bool checked) {
        setSelectedTextItalic(checked);
    });
    fontStyleLayout->addWidget(m_controls.m_propertyFontItalicButton);
    fontPanelLayout->addLayout(fontStyleLayout, 0);
    m_controls.m_propertyFontPanel->hide();
}

