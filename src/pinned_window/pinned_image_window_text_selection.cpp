#include "pinned_window/pinned_image_window.h"

#include "clipboard_image.h"
#include "ocr_result.h"

#include <QCursor>

#include <algorithm>
#include <limits>

namespace markshot::shot {

QPointF PinnedImageWindow::widgetToImage(QPointF point) const
{
    const QRectF imageRect = displayedImageRect();
    if (imageRect.width() <= 0.0 || imageRect.height() <= 0.0 || m_imageSize.isEmpty()) {
        return {};
    }
    return QPointF((point.x() - imageRect.left()) * static_cast<qreal>(m_imageSize.width()) / imageRect.width(),
                   (point.y() - imageRect.top()) * static_cast<qreal>(m_imageSize.height()) / imageRect.height());
}

QRectF PinnedImageWindow::imageToWidget(QRectF imageRect) const
{
    if (m_imageSize.isEmpty()) {
        return {};
    }
    const QRectF displayRect = displayedImageRect();
    const qreal sx = displayRect.width() / static_cast<qreal>(m_imageSize.width());
    const qreal sy = displayRect.height() / static_cast<qreal>(m_imageSize.height());
    return QRectF(displayRect.left() + imageRect.left() * sx,
                  displayRect.top() + imageRect.top() * sy,
                  imageRect.width() * sx,
                  imageRect.height() * sy);
}

std::optional<int> PinnedImageWindow::tokenAt(QPointF imagePoint) const
{
    const QVector<OcrToken> &tokens = activeTokens();
    for (int i = 0; i < tokens.size(); ++i) {
        const QRectF hitRect = tokens.at(i).imageRect.adjusted(-2.0, -2.0, 2.0, 2.0);
        if (hitRect.contains(imagePoint)) {
            return i;
        }
    }
    return std::nullopt;
}

std::optional<int> PinnedImageWindow::closestToken(QPointF imagePoint) const
{
    const QVector<OcrToken> &tokens = activeTokens();
    if (tokens.isEmpty()) {
        return std::nullopt;
    }

    int bestIndex = 0;
    qreal bestDistance = std::numeric_limits<qreal>::max();
    for (int i = 0; i < tokens.size(); ++i) {
        const QRectF rect = tokens.at(i).imageRect;
        const qreal dx = imagePoint.x() < rect.left()
            ? rect.left() - imagePoint.x()
            : imagePoint.x() > rect.right() ? imagePoint.x() - rect.right() : 0.0;
        const qreal dy = imagePoint.y() < rect.top()
            ? rect.top() - imagePoint.y()
            : imagePoint.y() > rect.bottom() ? imagePoint.y() - rect.bottom() : 0.0;
        const qreal distance = dx * dx + dy * dy;
        if (distance < bestDistance) {
            bestDistance = distance;
            bestIndex = i;
        }
    }
    return bestIndex;
}

void PinnedImageWindow::updateCursorForPosition(QPointF widgetPoint)
{
    const PinnedResizeDirection direction = resizeDirectionAt(widgetPoint);
    if (isPinnedResizeDirection(direction)) {
        setCursor(cursorForPinnedResizeDirection(direction));
        return;
    }

    if (m_config.textSelectionCopyEnabled && tokenAt(widgetToImage(widgetPoint))) {
        setCursor(Qt::IBeamCursor);
    } else {
        setCursor(Qt::OpenHandCursor);
    }
}

bool PinnedImageWindow::hasTextSelection() const
{
    const QVector<OcrToken> &tokens = activeTokens();
    return m_selectionAnchor >= 0
        && m_selectionFocus >= 0
        && m_selectionAnchor < tokens.size()
        && m_selectionFocus < tokens.size();
}

std::pair<int, int> PinnedImageWindow::selectionRange() const
{
    const int first = std::min(m_selectionAnchor, m_selectionFocus);
    const int last = std::max(m_selectionAnchor, m_selectionFocus);
    return {first, last};
}

void PinnedImageWindow::clearTextSelection()
{
    if (m_selectionAnchor < 0 && m_selectionFocus < 0) {
        return;
    }
    m_selectionAnchor = -1;
    m_selectionFocus = -1;
    update();
}

QString PinnedImageWindow::selectedText() const
{
    if (!hasTextSelection()) {
        return {};
    }
    const auto [first, last] = selectionRange();
    return tokenRangeText(first, last);
}

void PinnedImageWindow::copySelectedText()
{
    if (!hasTextSelection()) {
        return;
    }
    markshot::copyTextToClipboard(selectedText());
}

QString PinnedImageWindow::allText() const
{
    const QVector<OcrToken> &tokens = activeTokens();
    if (tokens.isEmpty()) {
        return {};
    }
    return tokenRangeText(0, tokens.size() - 1);
}

void PinnedImageWindow::copyImageText()
{
    if (!m_config.ocrEnabled) {
        return;
    }
    if (!activeTokens().isEmpty()) {
        markshot::copyTextToClipboard(allText());
        return;
    }
    m_copyTextAfterOcr = true;
    if (!m_ocrTask) {
        startOcr();
    }
}

QString PinnedImageWindow::tokenRangeText(int first, int last) const
{
    const QVector<OcrToken> &tokens = activeTokens();
    return markshot::ocr::tokenRangeText(sharedOcrTokens(tokens), first, last);
}

QVector<markshot::ocr::Token> PinnedImageWindow::sharedOcrTokens(const QVector<OcrToken> &tokens) const
{
    QVector<markshot::ocr::Token> sharedTokens;
    sharedTokens.reserve(tokens.size());
    for (const OcrToken &token : tokens) {
        sharedTokens.append({token.text,
                             token.imageRect,
                             token.line,
                             token.index,
                             token.confidence});
    }
    return sharedTokens;
}

const QVector<PinnedImageWindow::OcrToken> &PinnedImageWindow::activeTokens() const
{
    return m_translationActive ? m_translatedTokens : m_ocrTokens;
}

}  // namespace markshot::shot
