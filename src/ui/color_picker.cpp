#include "ui/color_picker.h"

#include <QHBoxLayout>
#include <QLineEdit>
#include <QLinearGradient>
#include <QMouseEvent>
#include <QPaintEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QRegularExpression>
#include <QRegularExpressionValidator>
#include <QSignalBlocker>
#include <QStringLiteral>
#include <QVBoxLayout>

#include <algorithm>

namespace markshot::ui {

namespace {

constexpr int kSvWidth = 184;
constexpr int kSvHeight = 112;
constexpr int kTrackHeight = 12;
constexpr int kSwatchSize = 26;
constexpr qreal kHandleRadius = 6.0;

void drawCheckerboard(QPainter &p, const QRectF &rect, int cell = 5)
{
    const QColor light(70, 76, 86);
    const QColor dark(45, 50, 58);
    p.fillRect(rect, light);
    p.setPen(Qt::NoPen);
    p.setBrush(dark);
    int row = 0;
    for (qreal y = rect.top(); y < rect.bottom(); y += cell, ++row) {
        const qreal startX = rect.left() + ((row % 2) ? 0 : cell);
        for (qreal x = startX; x < rect.right(); x += cell * 2) {
            const qreal w = std::min<qreal>(cell, rect.right() - x);
            const qreal h = std::min<qreal>(cell, rect.bottom() - y);
            p.drawRect(QRectF(x, y, w, h));
        }
    }
}

void drawHandle(QPainter &p, QPointF center, qreal radius, const QColor &innerFill)
{
    p.setPen(QPen(QColor(15, 17, 23, 220), 1.5));
    p.setBrush(QColor(255, 255, 255, 230));
    p.drawEllipse(center, radius, radius);
    p.setPen(Qt::NoPen);
    p.setBrush(innerFill);
    p.drawEllipse(center, radius - 2.5, radius - 2.5);
}

}  // namespace

// SVField =====================================================================
SVField::SVField(QWidget *parent) : QWidget(parent)
{
    setFixedSize(kSvWidth, kSvHeight);
    setCursor(Qt::CrossCursor);
}

void SVField::setHue(int hue)
{
    hue = std::clamp(hue, 0, 359);
    if (m_hue == hue) return;
    m_hue = hue;
    update();
}

void SVField::setSv(int sat, int val)
{
    sat = std::clamp(sat, 0, 255);
    val = std::clamp(val, 0, 255);
    if (m_sat == sat && m_val == val) return;
    m_sat = sat;
    m_val = val;
    update();
}

void SVField::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QRectF r = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);

    QPainterPath clip;
    clip.addRoundedRect(r, 4, 4);
    p.save();
    p.setClipPath(clip);

    p.fillRect(r, QColor::fromHsv(m_hue, 255, 255));

    QLinearGradient sat(r.topLeft(), r.topRight());
    sat.setColorAt(0, QColor(255, 255, 255, 255));
    sat.setColorAt(1, QColor(255, 255, 255, 0));
    p.fillRect(r, sat);

    QLinearGradient val(r.topLeft(), r.bottomLeft());
    val.setColorAt(0, QColor(0, 0, 0, 0));
    val.setColorAt(1, QColor(0, 0, 0, 255));
    p.fillRect(r, val);

    p.restore();

    p.setPen(QPen(QColor(255, 255, 255, 28), 1));
    p.setBrush(Qt::NoBrush);
    p.drawPath(clip);

    const qreal hx = r.left() + (m_sat / 255.0) * r.width();
    const qreal hy = r.top() + (1.0 - m_val / 255.0) * r.height();
    drawHandle(p, QPointF(hx, hy), kHandleRadius, QColor::fromHsv(m_hue, m_sat, m_val));
}

void SVField::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        selectAtPos(event->pos());
        event->accept();
    }
}

void SVField::mouseMoveEvent(QMouseEvent *event)
{
    if (event->buttons() & Qt::LeftButton) {
        selectAtPos(event->pos());
        event->accept();
    }
}

void SVField::selectAtPos(QPoint pos)
{
    const int s = std::clamp(qRound(pos.x() * 255.0 / std::max(1, width() - 1)), 0, 255);
    const int v = std::clamp(qRound(255.0 - pos.y() * 255.0 / std::max(1, height() - 1)), 0, 255);
    if (s == m_sat && v == m_val) return;
    m_sat = s;
    m_val = v;
    update();
    emit changed(m_sat, m_val);
}

// HueSlider ===================================================================
HueSlider::HueSlider(QWidget *parent) : QWidget(parent)
{
    setFixedHeight(kTrackHeight);
    setCursor(Qt::PointingHandCursor);
}

void HueSlider::setHue(int hue)
{
    hue = std::clamp(hue, 0, 359);
    if (m_hue == hue) return;
    m_hue = hue;
    update();
}

void HueSlider::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QRectF r = QRectF(rect()).adjusted(1, 1.5, -1, -1.5);
    QLinearGradient grad(r.topLeft(), r.topRight());
    for (int i = 0; i <= 6; ++i) {
        grad.setColorAt(i / 6.0, QColor::fromHsv((i * 60) % 360, 255, 255));
    }
    QPainterPath path;
    path.addRoundedRect(r, r.height() / 2.0, r.height() / 2.0);
    p.setPen(Qt::NoPen);
    p.fillPath(path, QBrush(grad));
    p.setPen(QPen(QColor(255, 255, 255, 26), 1));
    p.setBrush(Qt::NoBrush);
    p.drawPath(path);

    const qreal hx = r.left() + (m_hue / 359.0) * r.width();
    drawHandle(p, QPointF(hx, r.center().y()), kHandleRadius,
               QColor::fromHsv(m_hue, 255, 255));
}

void HueSlider::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        selectAtPos(event->pos().x());
        event->accept();
    }
}

void HueSlider::mouseMoveEvent(QMouseEvent *event)
{
    if (event->buttons() & Qt::LeftButton) {
        selectAtPos(event->pos().x());
        event->accept();
    }
}

void HueSlider::selectAtPos(int x)
{
    const int hue = std::clamp(qRound(x * 359.0 / std::max(1, width() - 1)), 0, 359);
    if (hue == m_hue) return;
    m_hue = hue;
    update();
    emit changed(m_hue);
}

// AlphaSlider =================================================================
AlphaSlider::AlphaSlider(QWidget *parent) : QWidget(parent)
{
    setFixedHeight(kTrackHeight);
    setCursor(Qt::PointingHandCursor);
}

void AlphaSlider::setAlpha(int alpha)
{
    alpha = std::clamp(alpha, 0, 255);
    if (m_alpha == alpha) return;
    m_alpha = alpha;
    update();
}

void AlphaSlider::setBaseColor(const QColor &color)
{
    QColor opaque = color;
    opaque.setAlpha(255);
    if (m_baseColor == opaque) return;
    m_baseColor = opaque;
    update();
}

void AlphaSlider::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QRectF r = QRectF(rect()).adjusted(1, 1.5, -1, -1.5);
    QPainterPath path;
    path.addRoundedRect(r, r.height() / 2.0, r.height() / 2.0);

    p.save();
    p.setClipPath(path);
    drawCheckerboard(p, r, 5);
    QLinearGradient grad(r.topLeft(), r.topRight());
    QColor zero = m_baseColor; zero.setAlpha(0);
    QColor full = m_baseColor; full.setAlpha(255);
    grad.setColorAt(0, zero);
    grad.setColorAt(1, full);
    p.fillRect(r, grad);
    p.restore();

    p.setPen(QPen(QColor(255, 255, 255, 26), 1));
    p.setBrush(Qt::NoBrush);
    p.drawPath(path);

    const qreal hx = r.left() + (m_alpha / 255.0) * r.width();
    QColor handleFill = m_baseColor;
    handleFill.setAlpha(m_alpha);
    drawHandle(p, QPointF(hx, r.center().y()), kHandleRadius, handleFill);
}

void AlphaSlider::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        selectAtPos(event->pos().x());
        event->accept();
    }
}

void AlphaSlider::mouseMoveEvent(QMouseEvent *event)
{
    if (event->buttons() & Qt::LeftButton) {
        selectAtPos(event->pos().x());
        event->accept();
    }
}

void AlphaSlider::selectAtPos(int x)
{
    const int alpha = std::clamp(qRound(x * 255.0 / std::max(1, width() - 1)), 0, 255);
    if (alpha == m_alpha) return;
    m_alpha = alpha;
    update();
    emit changed(m_alpha);
}

// ColorSwatch =================================================================
ColorSwatch::ColorSwatch(QWidget *parent) : QWidget(parent)
{
    setFixedSize(kSwatchSize, kSwatchSize);
}

void ColorSwatch::setColor(const QColor &color)
{
    if (m_color == color) return;
    m_color = color;
    update();
}

void ColorSwatch::paintEvent(QPaintEvent *)
{
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing, true);
    const QRectF r = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
    QPainterPath path;
    path.addRoundedRect(r, 6, 6);
    p.save();
    p.setClipPath(path);
    drawCheckerboard(p, r, 5);
    p.fillRect(r, m_color);
    p.restore();
    p.setPen(QPen(QColor(255, 255, 255, 56), 1));
    p.setBrush(Qt::NoBrush);
    p.drawPath(path);
}

// ColorPicker =================================================================
ColorPicker::ColorPicker(QWidget *parent) : QWidget(parent)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(6);

    m_svField = new SVField(this);
    layout->addWidget(m_svField);

    m_hueSlider = new HueSlider(this);
    layout->addWidget(m_hueSlider);

    m_alphaSlider = new AlphaSlider(this);
    layout->addWidget(m_alphaSlider);

    auto *bottom = new QHBoxLayout;
    bottom->setContentsMargins(0, 0, 0, 0);
    bottom->setSpacing(6);
    m_swatch = new ColorSwatch(this);
    bottom->addWidget(m_swatch);
    m_hex = new QLineEdit(this);
    m_hex->setPlaceholderText(QStringLiteral("#RRGGBB or #RRGGBBAA"));
    m_hex->setMaxLength(9);
    QRegularExpression re(QStringLiteral("^#?[0-9a-fA-F]{0,8}$"));
    m_hex->setValidator(new QRegularExpressionValidator(re, this));
    m_hex->setStyleSheet(QStringLiteral(
        "QLineEdit {"
        " color: #E5E7EB;"
        " background: rgba(255,255,255,16);"
        " border: 1px solid rgba(255,255,255,28);"
        " border-radius: 6px;"
        " padding: 4px 8px;"
        " font-family: 'Sans Serif';"
        " font-size: 12px;"
        " font-weight: 600;"
        " selection-background-color: #2DD4BF;"
        " selection-color: #042F2E;"
        "}"
        "QLineEdit:focus { border-color: rgba(45,212,191,180); }"));
    bottom->addWidget(m_hex, 1);
    layout->addLayout(bottom);

    setFixedWidth(kSvWidth);

    connect(m_svField, &SVField::changed, this, [this](int s, int v) {
        if (m_settingColor) return;
        m_sat = s;
        m_val = v;
        emitColor();
    });
    connect(m_hueSlider, &HueSlider::changed, this, [this](int h) {
        if (m_settingColor) return;
        m_hue = h;
        m_svField->setHue(h);
        emitColor();
    });
    connect(m_alphaSlider, &AlphaSlider::changed, this, [this](int a) {
        if (m_settingColor) return;
        m_alpha = a;
        emitColor();
    });
    connect(m_hex, &QLineEdit::editingFinished, this, [this] { onHexEdited(); });

    setColor(QColor(255, 0, 0, 255));
}

void ColorPicker::setColor(const QColor &color)
{
    QColor c = color.isValid() ? color : QColor(255, 255, 255);
    m_settingColor = true;
    m_color = c;
    int h = -1;
    int s = 0;
    int v = 0;
    c.getHsv(&h, &s, &v);
    if (h < 0) {
        // Achromatic input: keep last hue so the SV field doesn't snap to red.
        h = m_hue;
    }
    m_hue = h;
    m_sat = s;
    m_val = v;
    m_alpha = c.alpha();
    m_svField->setHue(m_hue);
    m_svField->setSv(m_sat, m_val);
    m_hueSlider->setHue(m_hue);
    QColor base = QColor::fromHsv(m_hue, m_sat, m_val);
    m_alphaSlider->setBaseColor(base);
    m_alphaSlider->setAlpha(m_alpha);
    m_swatch->setColor(m_color);
    rebuildHex();
    m_settingColor = false;
}

void ColorPicker::emitColor()
{
    QColor c = QColor::fromHsv(m_hue, m_sat, m_val);
    c.setAlpha(m_alpha);
    m_color = c;
    QColor base = c;
    base.setAlpha(255);
    m_alphaSlider->setBaseColor(base);
    m_swatch->setColor(c);
    rebuildHex();
    emit colorChanged(c);
}

void ColorPicker::rebuildHex()
{
    const QString text = m_alpha == 255
        ? QString::asprintf("#%02X%02X%02X", m_color.red(), m_color.green(), m_color.blue())
        : QString::asprintf("#%02X%02X%02X%02X", m_color.red(), m_color.green(), m_color.blue(), m_alpha);
    if (m_hex->text().compare(text, Qt::CaseInsensitive) == 0) return;
    const QSignalBlocker blocker(m_hex);
    m_hex->setText(text);
}

void ColorPicker::onHexEdited()
{
    QString text = m_hex->text().trimmed();
    if (text.isEmpty()) {
        rebuildHex();
        return;
    }
    if (!text.startsWith(QLatin1Char('#'))) {
        text.prepend(QLatin1Char('#'));
    }
    QColor parsed;
    int alpha = 255;
    if (text.length() == 9) {
        // Qt parses #RRGGBBAA only when prefixed. Manually split alpha.
        bool ok = false;
        alpha = text.mid(7, 2).toInt(&ok, 16);
        if (!ok) {
            rebuildHex();
            return;
        }
        parsed = QColor(text.left(7));
    } else if (text.length() == 7 || text.length() == 4) {
        parsed = QColor(text);
    } else {
        rebuildHex();
        return;
    }
    if (!parsed.isValid()) {
        rebuildHex();
        return;
    }
    parsed.setAlpha(alpha);
    setColor(parsed);
    emit colorChanged(parsed);
}

}  // namespace markshot::ui
