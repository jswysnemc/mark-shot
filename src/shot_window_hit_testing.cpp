#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::updateCursor()
{
    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        setCursor(Qt::BlankCursor);
        return;
    }

    if (m_imagePanning) {
        setCursor(Qt::ClosedHandCursor);
        return;
    }

    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected) {
        setCursor(m_imagePanning ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        switch (m_selectionDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
        case SelectionDrag::LineStart:
        case SelectionDrag::LineEnd:
        case SelectionDrag::NumberTip:
        case SelectionDrag::NumberBubble:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(captureCrossCursor());
            return;
        }
    }

    if (m_tool == Tool::Select) {
        switch (m_annotationDrag) {
        case SelectionDrag::MagnifierSource:
        case SelectionDrag::MagnifierLens:
        case SelectionDrag::Rotate:
        case SelectionDrag::LineControl:
        case SelectionDrag::LineStart:
        case SelectionDrag::LineEnd:
        case SelectionDrag::NumberTip:
        case SelectionDrag::NumberBubble:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(Qt::ArrowCursor);
            return;
        }
    }

    setCursor(m_tool == Tool::Text ? Qt::IBeamCursor : captureCrossCursor());
}

void ShotWindow::clearWheelPreview()
{
    if (!m_showWheelPreview) {
        return;
    }

    m_showWheelPreview = false;
    m_wheelPreviewTimer.invalidate();
    updateCursor();
    update();
}

bool ShotWindow::hasUsableSelection() const
{
    const QRectF selection = normalizedSelection();
    return selection.width() >= kMinSelectionSize && selection.height() >= kMinSelectionSize;
}

bool ShotWindow::imageNavigationAvailable() const
{
    return m_imageNavigationEnabled || m_mode == Mode::Editing;
}

bool ShotWindow::wheelZoomsImage() const
{
    return m_imageNavigationEnabled || (m_mode == Mode::Editing && m_tool == Tool::Select);
}

qreal ShotWindow::annotationSizeScale(bool widgetCoordinates) const
{
    if (!widgetCoordinates || m_frozenFrame.isNull()) {
        return 1.0;
    }

    return !m_frozenImageRect.isEmpty()
        ? m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width())
        : 1.0;
}

ShotWindow::SelectionDrag ShotWindow::selectionDragAt(QPointF imagePoint) const
{
    const QRectF selection = normalizedSelection();
    if (selection.isEmpty() || m_frozenImageRect.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (!selection.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)) {
        return SelectionDrag::None;
    }

    const bool nearLeft = std::abs(imagePoint.x() - selection.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - selection.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - selection.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - selection.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return selection.contains(imagePoint) ? SelectionDrag::Move : SelectionDrag::None;
}

ShotWindow::Annotation *ShotWindow::annotationById(int id)
{
    for (Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

const ShotWindow::Annotation *ShotWindow::annotationById(int id) const
{
    for (const Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

bool ShotWindow::annotationSupportsRotation(const Annotation &annotation) const
{
    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return false;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Highlighter:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
    case Tool::Text:
    case Tool::Number:
    case Tool::Magnifier:
        return true;
    }
    return false;
}

bool ShotWindow::annotationSupportsLineControl(const Annotation &annotation) const
{
    return annotation.tool == Tool::Line
        || annotation.tool == Tool::Arrow
        || (annotation.tool == Tool::Highlighter
            && annotation.highlighterStyle == HighlighterStyle::StraightLine);
}

QPointF ShotWindow::annotationLineControlPoint(const Annotation &annotation) const
{
    if (!annotationSupportsLineControl(annotation) || annotation.points.size() < 2) {
        return {};
    }
    if (annotation.points.size() >= 3) {
        return annotation.points.at(2);
    }
    return (annotation.points.first() + annotation.points.at(1)) / 2.0;
}

QString ShotWindow::numberLabelText(int number, NumberStyle style) const
{
    if (number <= 0) {
        return QString::number(number);
    }

    auto alphaLabel = [](int value, bool uppercase) {
        QString label;
        while (value > 0) {
            --value;
            const ushort base = uppercase ? QLatin1Char('A').unicode() : QLatin1Char('a').unicode();
            label.prepend(QChar(base + value % 26));
            value /= 26;
        }
        return label;
    };

    auto romanLabel = [](int value, bool uppercase) {
        struct RomanPart {
            int value;
            const char *text;
        };
        static constexpr std::array<RomanPart, 13> parts = {{
            {1000, "M"}, {900, "CM"}, {500, "D"}, {400, "CD"},
            {100, "C"}, {90, "XC"}, {50, "L"}, {40, "XL"},
            {10, "X"}, {9, "IX"}, {5, "V"}, {4, "IV"}, {1, "I"},
        }};

        QString label;
        for (const RomanPart &part : parts) {
            while (value >= part.value) {
                label += QString::fromLatin1(part.text);
                value -= part.value;
            }
        }
        return uppercase ? label : label.toLower();
    };

    auto chineseLabel = [](int value) {
        if (value <= 0 || value > 9999) {
            return QString::number(value);
        }

        const QString digits = QStringLiteral("零一二三四五六七八九");
        const QStringList units = {QStringLiteral("千"), QStringLiteral("百"), QStringLiteral("十"), QString()};
        QString label;
        bool pendingZero = false;
        int divisor = 1000;
        for (int unitIndex = 0; divisor >= 1; divisor /= 10, ++unitIndex) {
            const int digit = (value / divisor) % 10;
            if (digit == 0) {
                pendingZero = !label.isEmpty();
                continue;
            }
            if (pendingZero) {
                label += digits.mid(0, 1);
                pendingZero = false;
            }
            if (!(digit == 1 && divisor == 10 && label.isEmpty())) {
                label += digits.mid(digit, 1);
            }
            label += units.at(unitIndex);
        }
        return label;
    };

    switch (style) {
    case NumberStyle::Arabic:
        return QString::number(number);
    case NumberStyle::UpperAlpha:
        return alphaLabel(number, true);
    case NumberStyle::LowerAlpha:
        return alphaLabel(number, false);
    case NumberStyle::UpperRoman:
        return romanLabel(number, true);
    case NumberStyle::LowerRoman:
        return romanLabel(number, false);
    case NumberStyle::HeavenlyStem: {
        const QString stems = QStringLiteral("甲乙丙丁戊己庚辛壬癸");
        return stems.mid((number - 1) % stems.size(), 1);
    }
    case NumberStyle::Chinese:
        return chineseLabel(number);
    }

    return QString::number(number);
}

QPointF ShotWindow::rotatedPoint(QPointF point, QPointF center, qreal degrees) const
{
    const qreal radians = degrees * M_PI / 180.0;
    const qreal c = std::cos(radians);
    const qreal s = std::sin(radians);
    const QPointF delta = point - center;
    return center + QPointF(delta.x() * c - delta.y() * s,
                            delta.x() * s + delta.y() * c);
}

QRectF ShotWindow::rotatedRectBounds(QRectF rect, qreal degrees) const
{
    rect = rect.normalized();
    if (rect.isEmpty() || qFuzzyIsNull(degrees)) {
        return rect;
    }

    const QPointF center = rect.center();
    const QVector<QPointF> points = {
        rotatedPoint(rect.topLeft(), center, degrees),
        rotatedPoint(rect.topRight(), center, degrees),
        rotatedPoint(rect.bottomLeft(), center, degrees),
        rotatedPoint(rect.bottomRight(), center, degrees),
    };

    qreal left = points.first().x();
    qreal right = left;
    qreal top = points.first().y();
    qreal bottom = top;
    for (const QPointF &point : points) {
        left = std::min(left, point.x());
        right = std::max(right, point.x());
        top = std::min(top, point.y());
        bottom = std::max(bottom, point.y());
    }
    return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
}

QRectF ShotWindow::annotationUnrotatedBounds(const Annotation &annotation) const
{
    auto pointsBounds = [&annotation] {
        if (annotation.points.isEmpty()) {
            return QRectF();
        }
        qreal left = annotation.points.first().x();
        qreal right = left;
        qreal top = annotation.points.first().y();
        qreal bottom = top;
        for (const QPointF &point : annotation.points) {
            left = std::min(left, point.x());
            right = std::max(right, point.x());
            top = std::min(top, point.y());
            bottom = std::max(bottom, point.y());
        }
        return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
    };

    QRectF bounds;
    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return {};
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
        bounds = pointsBounds();
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Highlighter:
        if (annotation.highlighterStyle == HighlighterStyle::StraightLine
            && annotation.points.size() >= 2) {
            bounds = annotation.points.size() >= 3
                ? pointsBounds()
                : QRectF(annotation.points.first(), annotation.points.at(1)).normalized();
        } else {
            bounds = pointsBounds();
        }
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        bounds = annotation.rect.normalized();
        break;
    case Tool::Magnifier:
        bounds = annotation.rect.normalized().united(magnifierSourceRect(annotation));
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Text:
        bounds = textContentRect(annotation, false);
        break;
    case Tool::Number: {
        if (annotation.points.isEmpty()) {
            return {};
        }
        const qreal radius = std::max<qreal>(13.0, 13.0 + annotation.width * 1.35);
        const QPointF tip = annotation.points.first();
        const QPointF center = annotation.points.size() >= 2 ? annotation.points.last() : tip;
        bounds = QRectF(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);
        bounds = bounds.united(QRectF(tip, QSizeF(0.0, 0.0)));
        break;
    }
    }

    return bounds.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QPointF ShotWindow::annotationRotationCenter(const Annotation &annotation, bool widgetCoordinates) const
{
    const QRectF bounds = annotationUnrotatedBounds(annotation);
    const QPointF center = bounds.center();
    return widgetCoordinates ? imageToWidget(center) : center;
}

QPointF ShotWindow::annotationRotationHandlePoint(const Annotation &annotation, bool widgetCoordinates) const
{
    QRectF bounds = annotationUnrotatedBounds(annotation);
    if (bounds.isEmpty()) {
        return {};
    }

    if (widgetCoordinates) {
        bounds = imageRectToWidget(bounds);
    }
    const QPointF center = bounds.center();
    const qreal angle = annotation.rotationDegrees;
    const QPointF topCenter = rotatedPoint(QPointF(bounds.center().x(), bounds.top()), center, angle);
    QPointF direction = topCenter - center;
    const qreal length = QLineF(center, topCenter).length();
    if (length <= 0.1) {
        direction = QPointF(0.0, -1.0);
    } else {
        direction /= length;
    }
    const qreal handleGap = widgetCoordinates
        ? 26.0
        : 26.0 / std::max<qreal>(0.001, annotationSizeScale(true));
    return topCenter + direction * handleGap;
}

QPointF ShotWindow::selectionRotationHandlePoint(QRectF imageBounds, bool widgetCoordinates) const
{
    imageBounds = imageBounds.normalized();
    if (imageBounds.isEmpty()) {
        return {};
    }

    QRectF bounds = widgetCoordinates ? imageRectToWidget(imageBounds) : imageBounds;
    const QPointF center = bounds.center();
    const QPointF topCenter(bounds.center().x(), bounds.top());
    QPointF direction = topCenter - center;
    const qreal length = QLineF(center, topCenter).length();
    if (length <= 0.1) {
        direction = QPointF(0.0, -1.0);
    } else {
        direction /= length;
    }

    const qreal handleGap = widgetCoordinates
        ? 26.0
        : 26.0 / std::max<qreal>(0.001, annotationSizeScale(true));
    return topCenter + direction * handleGap;
}

QRectF ShotWindow::annotationBounds(const Annotation &annotation) const
{
    const QRectF bounds = annotationUnrotatedBounds(annotation);
    if (!annotationSupportsRotation(annotation)) {
        return bounds;
    }
    return rotatedRectBounds(bounds, annotation.rotationDegrees)
        .intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QVector<int> ShotWindow::selectedAnnotationIds() const
{
    QVector<int> ids;
    for (int id : m_selectedAnnotationIds) {
        if (annotationById(id) && !ids.contains(id)) {
            ids.append(id);
        }
    }
    if (m_selectedAnnotationId.has_value() && annotationById(*m_selectedAnnotationId) && !ids.contains(*m_selectedAnnotationId)) {
        ids.append(*m_selectedAnnotationId);
    }
    return ids;
}

void ShotWindow::setSelectedAnnotations(QVector<int> annotationIds)
{
    QVector<int> validIds;
    for (int id : annotationIds) {
        if (annotationById(id) && !validIds.contains(id)) {
            validIds.append(id);
        }
    }
    if (validIds != selectedAnnotationIds()) {
        m_lineSkeletonDragPointIndex = -1;
    }
    m_selectedAnnotationIds = validIds;
    m_selectedAnnotationId = validIds.size() == 1
        ? std::optional<int>(validIds.first())
        : std::nullopt;
    if (!validIds.isEmpty()) {
        m_imageSelected = false;
        m_imagePanning = false;
    }
}

QRectF ShotWindow::selectedAnnotationsBounds() const
{
    QRectF bounds;
    for (int id : selectedAnnotationIds()) {
        const Annotation *annotation = annotationById(id);
        if (!annotation) {
            continue;
        }
        const QRectF annotationRect = annotationBounds(*annotation);
        if (annotationRect.isEmpty()) {
            continue;
        }
        bounds = bounds.isEmpty() ? annotationRect : bounds.united(annotationRect);
    }
    return bounds.normalized();
}

QVector<int> ShotWindow::annotationsInRect(QRectF imageRect) const
{
    imageRect = imageRect.normalized();
    QVector<int> ids;
    if (imageRect.width() < 2.0 || imageRect.height() < 2.0) {
        return ids;
    }
    for (const Annotation &annotation : m_annotations) {
        const QRectF bounds = annotationBounds(annotation);
        if (!bounds.isEmpty() && imageRect.intersects(bounds)) {
            ids.append(annotation.id);
        }
    }
    return ids;
}

ShotWindow::SelectionDrag ShotWindow::annotationBoundsDragAt(QPointF imagePoint, QRectF bounds) const
{
    bounds = bounds.normalized();
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const bool nearLeft = std::abs(imagePoint.x() - bounds.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - bounds.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - bounds.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - bounds.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return bounds.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)
        ? SelectionDrag::Move
        : SelectionDrag::None;
}

ShotWindow::SelectionDrag ShotWindow::annotationDragAt(QPointF imagePoint, int annotationId) const
{
    const Annotation *annotation = annotationById(annotationId);
    if (!annotation) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (annotationSupportsRotation(*annotation)) {
        const QPointF rotationHandle = annotationRotationHandlePoint(*annotation, false);
        if (!rotationHandle.isNull() && QLineF(imagePoint, rotationHandle).length() <= imageTolerance * 1.4) {
            return SelectionDrag::Rotate;
        }
    }

    QRectF localBounds;
    QPointF localPoint = imagePoint;
    if (annotationSupportsRotation(*annotation)) {
        localBounds = annotationUnrotatedBounds(*annotation);
        if (!localBounds.isEmpty()) {
            localPoint = rotatedPoint(imagePoint, localBounds.center(), -annotation->rotationDegrees);
        }
    }

    if (annotation->tool == Tool::Magnifier) {
        const SelectionDrag magnifierDrag = magnifierDragAt(*annotation, localPoint);
        if (magnifierDrag != SelectionDrag::None) {
            return magnifierDrag;
        }
    }

    if (annotation->tool == Tool::Number) {
        const SelectionDrag numberDrag = numberDragAt(*annotation, localPoint);
        if (numberDrag != SelectionDrag::None) {
            return numberDrag;
        }
    }

    if (annotationSupportsLineAnchors(*annotation) && annotation->points.size() >= 2) {
        const SelectionDrag lineDrag = lineAnchorDragAt(*annotation, localPoint);
        if (lineDrag != SelectionDrag::None) {
            return lineDrag;
        }
    }

    if (annotationSupportsRotation(*annotation) && !localBounds.isEmpty()) {
        return annotationBoundsDragAt(localPoint, localBounds);
    }

    const QRectF bounds = annotationBounds(*annotation);
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    return annotationBoundsDragAt(imagePoint, bounds);
}

std::optional<int> ShotWindow::annotationAt(QPointF imagePoint) const
{
    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        const Annotation &annotation = m_annotations.at(i);
        QPointF localPoint = imagePoint;
        if (annotationSupportsRotation(annotation)) {
            const QRectF localBounds = annotationUnrotatedBounds(annotation);
            if (!localBounds.isEmpty()) {
                localPoint = rotatedPoint(imagePoint, localBounds.center(), -annotation.rotationDegrees);
            }
        }
        if (annotation.tool == Tool::Magnifier) {
            if (magnifierDragAt(annotation, localPoint) != SelectionDrag::None) {
                return annotation.id;
            }
        }
        if (annotation.tool == Tool::Number) {
            if (numberDragAt(annotation, localPoint) != SelectionDrag::None) {
                return annotation.id;
            }
        }
        if (annotationSupportsLineControl(annotation) && annotation.points.size() >= 2) {
            const qreal pathTolerance = std::max(imageTolerance, annotation.width * 0.5 + imageTolerance);
            if (lineSkeletonContainsPoint(annotation.points, localPoint, pathTolerance)) {
                return annotation.id;
            }
            continue;
        }
        const QRectF bounds = annotationBounds(annotation).adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance);
        if (bounds.contains(imagePoint)) {
            return annotation.id;
        }
    }
    return std::nullopt;
}
