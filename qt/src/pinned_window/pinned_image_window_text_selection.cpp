#include "pinned_window/pinned_image_window.h"

#include "clipboard_image.h"
#include "ocr_result.h"


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
    std::optional<int> best;
    qreal bestDistance = std::numeric_limits<qreal>::max();
    for (int i = 0; i < tokens.size(); ++i) {
        const QRectF rect = selectionImageRectForToken(tokens.at(i));
        if (rect.isEmpty()) {
            continue;
        }
        // 1. 重叠的检测框取离中心更近的字，避免前一个字的外扩挡住后一个字
        if (!rect.adjusted(-1.0, -1.0, 1.0, 1.0).contains(imagePoint)) {
            continue;
        }
        const qreal dx = imagePoint.x() - rect.center().x();
        const qreal dy = imagePoint.y() - rect.center().y();
        const qreal distance = dx * dx + dy * dy;
        if (distance < bestDistance) {
            bestDistance = distance;
            best = i;
        }
    }
    return best;
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
        const QRectF rect = selectionImageRectForToken(tokens.at(i));
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
    // 1. 【置顶图片】【光标状态】活动操作保持自己的光标，不受后台翻译完成事件覆盖
    if (isPinnedResizeDirection(m_resizeDrag.direction)) {
        setCursor(cursorForPinnedResizeDirection(m_resizeDrag.direction));
        return;
    }
    if (m_selectingText || m_moving) {
        setCursor(m_selectingText ? Qt::IBeamCursor : Qt::ClosedHandCursor);
        return;
    }
    // 2. 【置顶图片】【悬停反馈】缩放和可选文字优先，等待翻译仅影响当前窗口空白处
    const PinnedResizeDirection direction = resizeDirectionAt(widgetPoint);
    if (isPinnedResizeDirection(direction)) {
        setCursor(cursorForPinnedResizeDirection(direction));
        return;
    }

    if (m_config.textSelectionCopyEnabled && tokenAt(widgetToImage(widgetPoint))) {
        setCursor(Qt::IBeamCursor);
    } else {
        setCursor(m_translationBusyCursor ? Qt::BusyCursor : Qt::OpenHandCursor);
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

QRectF PinnedImageWindow::selectionImageRectForToken(const OcrToken &token) const
{
    // 1. 拖选高亮使用字符切分后的检测框，不再向外补宽
    return token.imageRect;
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
    if (m_translationActive) {
        return m_translatedTokens;
    }
    return m_ocrSelectionTokens.isEmpty() ? m_ocrTokens : m_ocrSelectionTokens;
}

void PinnedImageWindow::selectTokenLine(int tokenIndex)
{
    const QVector<OcrToken> &tokens = activeTokens();
    if (tokenIndex < 0 || tokenIndex >= tokens.size()) {
        return;
    }
    const int line = tokens.at(tokenIndex).line;
    int first = tokenIndex;
    int last = tokenIndex;
    for (int index = 0; index < tokens.size(); ++index) {
        if (tokens.at(index).line != line) {
            continue;
        }
        first = std::min(first, index);
        last = std::max(last, index);
    }
    m_selectionAnchor = first;
    m_selectionFocus = last;
    update();
}

}  // namespace markshot::shot
