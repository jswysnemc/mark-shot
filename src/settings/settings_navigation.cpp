#include "settings/settings_navigation.h"

#include "settings/settings_design_tokens.h"
#include "ui/i18n.h"

#include <QIcon>
#include <QLabel>
#include <QListWidgetItem>
#include <QPainter>
#include <QPen>
#include <QPixmap>
#include <QPolygonF>
#include <QStringLiteral>
#include <QVBoxLayout>

namespace markshot::settings {

namespace {

/// 导航图标种类，与分类顺序一一对应。
enum class NavIcon {
    General,
    Capture,
    Shortcuts,
    Annotation,
    Pinned,
    Integrations,
    Scroll,
    Storage,
    Advanced,
};

/// @brief 构造圆角连接的标准画笔。
/// @param ink 描边颜色。
/// @param width 描边宽度。
QPen navPen(const QColor &ink, qreal width = 1.8)
{
    return QPen(ink, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
}

/// @brief 绘制齿轮图标（General）。
void drawGear(QPainter &p, const QColor &ink)
{
    const QPointF center(9, 9);
    // 1. 先绘制短矩形齿，避免外轮廓在小尺寸下像太阳光芒
    p.save();
    p.translate(center);
    p.setPen(Qt::NoPen);
    p.setBrush(ink);
    for (int i = 0; i < 8; ++i) {
        p.drawRoundedRect(QRectF(-1.05, -7.8, 2.1, 2.55), 0.55, 0.55);
        p.rotate(45.0);
    }
    p.restore();

    // 2. 外环和中心孔提供机械齿轮的主体轮廓
    p.setPen(navPen(ink, 1.45));
    p.setBrush(Qt::NoBrush);
    p.drawEllipse(center, 5.25, 5.25);
    p.drawEllipse(center, 2.15, 2.15);
}

/// @brief 绘制相机图标（Capture）。
void drawCamera(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink));
    p.setBrush(Qt::NoBrush);
    p.drawRoundedRect(QRectF(2, 5, 14, 10), 2.5, 2.5);
    p.drawEllipse(QPointF(9, 10), 3.0, 3.0);
}

/// @brief 绘制键盘图标（Shortcuts）。
void drawKeyboard(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink, 1.6));
    p.setBrush(Qt::NoBrush);
    p.drawRoundedRect(QRectF(2, 4, 14, 10), 2, 2);
    // 2 行 3 列键点
    p.setBrush(ink);
    p.setPen(Qt::NoPen);
    for (int row = 0; row < 2; ++row) {
        for (int col = 0; col < 3; ++col) {
            p.drawEllipse(QPointF(5 + col * 4, 6.5 + row * 3), 0.9, 0.9);
        }
    }
}

/// @brief 绘制画笔图标（Annotation）。
void drawBrush(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink, 2.2));
    p.setBrush(Qt::NoBrush);
    p.drawLine(QPointF(5, 14), QPointF(11, 8));
    // 笔尖三角
    p.setBrush(ink);
    p.setPen(Qt::NoPen);
    QPolygonF tip;
    tip << QPointF(11, 8) << QPointF(13.6, 5.4) << QPointF(10.6, 5.0);
    p.drawPolygon(tip);
}

/// @brief 绘制图钉图标（Pinned Image）。
void drawPin(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink));
    p.setBrush(ink);
    p.drawEllipse(QPointF(9, 6), 3.0, 3.0);
    // 针身与底横
    p.setBrush(Qt::NoBrush);
    p.drawLine(QPointF(9, 9), QPointF(9, 15));
    p.drawLine(QPointF(6.5, 15), QPointF(11.5, 15));
}

/// @brief 绘制咬合方块图标（Integrations）。
void drawPuzzle(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink));
    p.setBrush(Qt::NoBrush);
    p.drawRoundedRect(QRectF(2, 5, 7.5, 7.5), 1.8, 1.8);
    p.drawRoundedRect(QRectF(8.5, 6, 7.5, 7.5), 1.8, 1.8);
}

/// @brief 绘制向下双箭头图标（Scroll Capture）。
void drawScroll(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink, 1.9));
    p.setBrush(Qt::NoBrush);
    QPolygonF a1;
    a1 << QPointF(4, 5) << QPointF(9, 10) << QPointF(14, 5);
    p.drawPolyline(a1);
    QPolygonF a2;
    a2 << QPointF(4, 10) << QPointF(9, 15) << QPointF(14, 10);
    p.drawPolyline(a2);
}

/// @brief 绘制数据库圆柱图标（Storage）。
void drawStorage(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink, 1.6));
    p.setBrush(Qt::NoBrush);
    // 1. 顶面椭圆
    p.drawEllipse(QPointF(9, 4.5), 6.0, 2.0);
    // 2. 圆柱身两侧
    p.drawLine(QPointF(3, 4.5), QPointF(3, 13.5));
    p.drawLine(QPointF(15, 4.5), QPointF(15, 13.5));
    // 3. 底面半椭圆弧
    p.drawArc(QRectF(3, 11.5, 12, 4), 0, 180 * 16);
}

/// @brief 绘制终端图标（Advanced）。
void drawTerminal(QPainter &p, const QColor &ink)
{
    p.setPen(navPen(ink, 1.6));
    p.setBrush(Qt::NoBrush);
    p.drawRoundedRect(QRectF(2, 3, 14, 12), 2, 2);
    // 提示符 > 与光标下划线
    QPolygonF arrow;
    arrow << QPointF(5, 7) << QPointF(8, 9.5) << QPointF(5, 12);
    p.drawPolyline(arrow);
    p.drawLine(QPointF(10, 12), QPointF(14, 12));
}

/// @brief 按图标种类分派绘制。
void drawNavGlyph(QPainter &p, NavIcon icon, const QColor &ink)
{
    switch (icon) {
    case NavIcon::General:
        drawGear(p, ink);
        break;
    case NavIcon::Capture:
        drawCamera(p, ink);
        break;
    case NavIcon::Shortcuts:
        drawKeyboard(p, ink);
        break;
    case NavIcon::Annotation:
        drawBrush(p, ink);
        break;
    case NavIcon::Pinned:
        drawPin(p, ink);
        break;
    case NavIcon::Integrations:
        drawPuzzle(p, ink);
        break;
    case NavIcon::Scroll:
        drawScroll(p, ink);
        break;
    case NavIcon::Storage:
        drawStorage(p, ink);
        break;
    case NavIcon::Advanced:
        drawTerminal(p, ink);
        break;
    }
}

/// @brief 渲染单个导航图标位图。
/// @param icon 图标种类。
/// @param ink 描边颜色。
QPixmap renderNavPixmap(NavIcon icon, const QColor &ink)
{
    QPixmap pm(tokens::kNavIconSize, tokens::kNavIconSize);
    pm.fill(Qt::transparent);
    QPainter p(&pm);
    p.setRenderHint(QPainter::Antialiasing, true);
    drawNavGlyph(p, icon, ink);
    return pm;
}

/// @brief 构造导航图标，含默认态与选中态两套位图。
/// @param icon 图标种类。
QIcon makeNavIcon(NavIcon icon)
{
    QIcon result;
    result.addPixmap(renderNavPixmap(icon, tokens::kTextSecondary), QIcon::Normal, QIcon::Off);
    result.addPixmap(renderNavPixmap(icon, tokens::kAccent), QIcon::Selected, QIcon::Off);
    result.addPixmap(renderNavPixmap(icon, tokens::kAccent), QIcon::Active, QIcon::Off);
    return result;
}

}  // namespace

SettingsNavigation::SettingsNavigation(QWidget *parent)
    : QWidget(parent)
{
    setObjectName(QStringLiteral("settingsSidebar"));
    setFixedWidth(tokens::kSidebarWidth);
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(18, 20, 18, 18);
    layout->setSpacing(14);

    buildHeader();
    buildList();
}

void SettingsNavigation::buildHeader()
{
    auto *title = new QLabel(QStringLiteral("Mark Shot"), this);
    title->setObjectName(QStringLiteral("settingsHeroTitle"));
    layout()->addWidget(title);
    auto *subtitle = new QLabel(MS_TR("Settings Center"), this);
    subtitle->setObjectName(QStringLiteral("settingsHeroText"));
    subtitle->setWordWrap(true);
    layout()->addWidget(subtitle);
}

void SettingsNavigation::buildList()
{
    m_list = new QListWidget(this);
    m_list->setObjectName(QStringLiteral("settingsNavigation"));
    m_list->setFrameShape(QFrame::NoFrame);
    m_list->setFocusPolicy(Qt::NoFocus);
    m_list->setIconSize(QSize(tokens::kNavIconSize, tokens::kNavIconSize));

    // 组1：常用
    addCategory(MS_TR("General"), makeNavIcon(NavIcon::General));
    addCategory(MS_TR("Capture"), makeNavIcon(NavIcon::Capture));
    addCategory(MS_TR("Shortcuts"), makeNavIcon(NavIcon::Shortcuts));
    addCategory(MS_TR("Annotation"), makeNavIcon(NavIcon::Annotation));
    addSeparator();
    // 组2：进阶
    addCategory(MS_TR("Pinned Image"), makeNavIcon(NavIcon::Pinned));
    addCategory(MS_TR("Integrations"), makeNavIcon(NavIcon::Integrations));
    addCategory(MS_TR("Scroll Capture"), makeNavIcon(NavIcon::Scroll));
    addCategory(MS_TR("Storage"), makeNavIcon(NavIcon::Storage));
    addSeparator();
    // 组3：其他
    addCategory(MS_TR("Advanced"), makeNavIcon(NavIcon::Advanced));

    static_cast<QVBoxLayout *>(layout())->addWidget(m_list, 1);

    // 实际行号转逻辑索引后转发，分隔项不占用逻辑索引
    connect(m_list, &QListWidget::currentRowChanged, this, [this](int row) {
        const int logical = m_logicalRows.indexOf(row);
        if (logical >= 0) {
            emit navigationChanged(logical);
        }
    });
}

void SettingsNavigation::addCategory(const QString &text, const QIcon &icon)
{
    auto *item = new QListWidgetItem(m_list);
    item->setText(text);
    item->setIcon(icon);
    item->setSizeHint(QSize(0, 38));
    m_logicalRows.append(m_list->row(item));
}

void SettingsNavigation::addSeparator()
{
    auto *item = new QListWidgetItem(m_list);
    item->setText(QString());
    item->setFlags(Qt::NoItemFlags);
    item->setSizeHint(QSize(0, 10));
}

void SettingsNavigation::setCurrentLogicalRow(int index)
{
    if (index < 0 || index >= m_logicalRows.size()) {
        return;
    }
    m_list->setCurrentRow(m_logicalRows[index]);
}

}  // namespace markshot::settings
