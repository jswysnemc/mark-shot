#pragma once

#include "pinned_window/pinned_resize_controller.h"
#include "shot_window_internal.h"

#include <QByteArray>
#include <QImage>
#include <QJsonArray>
#include <QPainter>
#include <QPixmap>
#include <QPoint>
#include <QPointF>
#include <QProcess>
#include <QRect>
#include <QRectF>
#include <QSize>
#include <QString>
#include <QVector>
#include <QWidget>

#include <optional>
#include <utility>

class QContextMenuEvent;
class QEvent;
class QKeyEvent;
class QMouseEvent;
class QPaintEvent;
class QWheelEvent;

namespace markshot::ocr {
struct Token;
}

namespace markshot::shot {

/// @brief 提供置顶图片显示、文本识别、文本选择、翻译和边界缩放的窗口。
class PinnedImageWindow final : public QWidget {
public:
    /// @brief 表示单个 OCR 文字块的边界和文本。
    struct OcrToken {
        /// @brief OCR 识别出的文本内容。
        QString text;
        /// @brief 文字块在图片坐标系中的矩形。
        QRectF imageRect;
        /// @brief 文字块所在行号。
        int line = 0;
        /// @brief 文字块在行内的序号。
        int index = 0;
        /// @brief OCR 识别置信度。
        qreal confidence = 0.0;
    };

    /// @brief 创建置顶图片窗口。
    /// @param image 要显示和处理的图片。
    /// @param initialTopLeft 初始全局左上角位置，未提供时居中显示。
    explicit PinnedImageWindow(QImage image, std::optional<QPoint> initialTopLeft = std::nullopt);

    /// @brief 释放 OCR 和翻译子进程资源。
    ~PinnedImageWindow() override;

protected:
    /// @brief 处理窗口激活状态并在需要时重新置顶。
    /// @param event Qt 窗口事件。
    /// @return Qt 事件处理结果。
    bool event(QEvent *event) override;

    /// @brief 绘制置顶图片、边框、文本选择和翻译覆盖层。
    /// @param event 绘制事件。
    void paintEvent(QPaintEvent *event) override;

    /// @brief 处理鼠标按下，启动文本选择、窗口拖动或边界缩放。
    /// @param event 鼠标事件。
    void mousePressEvent(QMouseEvent *event) override;

    /// @brief 处理鼠标移动，更新拖动、缩放、文本选择和光标。
    /// @param event 鼠标事件。
    void mouseMoveEvent(QMouseEvent *event) override;

    /// @brief 处理鼠标释放并结束交互状态。
    /// @param event 鼠标事件。
    void mouseReleaseEvent(QMouseEvent *event) override;

    /// @brief 处理滚轮缩放。
    /// @param event 滚轮事件。
    void wheelEvent(QWheelEvent *event) override;

    /// @brief 处理双击关闭窗口或选择单个 OCR 文字块。
    /// @param event 鼠标事件。
    void mouseDoubleClickEvent(QMouseEvent *event) override;

    /// @brief 显示置顶图片上下文菜单。
    /// @param event 菜单事件。
    void contextMenuEvent(QContextMenuEvent *event) override;

    /// @brief 处理复制和关闭快捷键。
    /// @param event 键盘事件。
    void keyPressEvent(QKeyEvent *event) override;

private:
    /// @brief 旋转当前置顶图片。
    /// @param degrees 旋转角度。
    void rotateImage(qreal degrees);

    /// @brief 将当前置顶图片另存为文件。
    /// @return 无返回值。
    void saveImageAs();

    /// @brief 计算当前图片的设备无关基础显示尺寸。
    /// @return 逻辑显示尺寸。
    QSize displayBaseSizeForPixmap() const;

    /// @brief 将置顶窗口提升到当前平台支持的最上层。
    /// @return 无返回值。
    void raisePinnedWindow();

    /// @brief 保存并应用置顶偏好。
    /// @param alwaysOnTop 是否保持置顶。
    void setAlwaysOnTop(bool alwaysOnTop);

    /// @brief 在协议角色需要变化时重新创建窗口。
    /// @return 无返回值。
    void recreateWithCurrentImage();

    /// @brief 恢复重新创建前的窗口几何和缩放。
    /// @param geometry 全局逻辑几何。
    /// @param scale 图片缩放比例。
    void restorePinnedState(QRect geometry, qreal scale);

    /// @brief 延迟多次请求置顶，等待窗口系统注册新窗口。
    /// @return 无返回值。
    void schedulePinnedWindowRaise();

    /// @brief 按缩放比例调整窗口尺寸。
    /// @param scale 新缩放比例。
    /// @param globalAnchor 全局锚点。
    /// @param localAnchor 窗口内锚点。
    void resizeByScale(qreal scale, QPoint globalAnchor, QPointF localAnchor);

    /// @brief 返回记录的全局左上角位置。
    /// @return 全局逻辑左上角。
    QPoint pinnedTopLeft() const;

    /// @brief 将本地锚点转换成适合当前平台协议的全局坐标。
    /// @param localAnchor 窗口内锚点。
    /// @param fallbackGlobal 普通窗口的全局坐标。
    /// @return 全局逻辑坐标。
    QPoint logicalGlobalPointForLocalAnchor(QPointF localAnchor, QPoint fallbackGlobal);

    /// @brief 更新置顶窗口的逻辑几何和平台几何。
    /// @param geometry 全局逻辑几何。
    /// @param moveWidget 是否同步更新 QWidget 几何。
    void setPinnedGeometry(QRect geometry, bool moveWidget);

    /// @brief 判断鼠标位置命中的缩放边界方向。
    /// @param widgetPoint 鼠标在窗口内的位置。
    /// @return 缩放边界方向。
    PinnedResizeDirection resizeDirectionAt(QPointF widgetPoint) const;

    /// @brief 开始边界拖拽缩放。
    /// @param event 鼠标按下事件。
    /// @return 成功进入缩放状态时返回 true。
    bool startResizeDrag(QMouseEvent *event);

    /// @brief 继续处理边界拖拽缩放。
    /// @param event 鼠标移动事件。
    /// @return 当前正在缩放时返回 true。
    bool continueResizeDrag(QMouseEvent *event);

    /// @brief 结束边界拖拽缩放。
    /// @param widgetPoint 鼠标释放时的窗口内位置。
    void finishResizeDrag(QPointF widgetPoint);

    /// @brief 应用边界拖拽计算出的窗口几何。
    /// @param geometry 新窗口几何。
    void applyResizeGeometry(QRect geometry);

    /// @brief 启动 OCR 子进程。
    /// @return 无返回值。
    void startOcr();

    /// @brief 返回默认 OCR 辅助程序路径。
    /// @return OCR 辅助程序路径。
    QString defaultOcrHelperProgram() const;

    /// @brief 取消正在运行的 OCR 子进程并清理临时文件。
    /// @return 无返回值。
    void cancelOcr();

    /// @brief 处理 OCR 子进程结束结果。
    /// @param process OCR 子进程。
    /// @param output 标准输出。
    /// @param errorOutput 标准错误。
    /// @param exitCode 退出码。
    /// @param exitStatus 退出状态。
    /// @param processError 进程错误。
    void finishOcr(QProcess *process,
                   const QByteArray &output,
                   const QByteArray &errorOutput,
                   int exitCode,
                   QProcess::ExitStatus exitStatus,
                   QProcess::ProcessError processError);

    /// @brief 解析并应用 OCR 输出。
    /// @param output 标准输出。
    /// @param errorOutput 标准错误。
    void applyOcrOutput(const QByteArray &output, const QByteArray &errorOutput);

    /// @brief 通知用户默认 OCR 后端缺失。
    /// @return 无返回值。
    void notifyMissingOcrBackend();

    /// @brief 将 OCR JSON 输出转换为窗口内部 token。
    /// @param output OCR JSON 输出。
    /// @return 解析得到的 token 列表。
    QVector<OcrToken> tokensFromJsonOutput(const QByteArray &output) const;

    /// @brief 启动翻译子进程。
    /// @param activateWhenFinished 翻译完成后是否直接显示译文。
    /// @param showBusyCursor 是否显示忙碌光标。
    void startTranslation(bool activateWhenFinished = true, bool showBusyCursor = true);

    /// @brief 构建翻译子进程输入 JSON。
    /// @return 翻译输入 JSON。
    QByteArray translationInputJson() const;

    /// @brief 将矩形转换为 OCR JSON 数组。
    /// @param rect 图片坐标系矩形。
    /// @return JSON 数组。
    QJsonArray rectToJson(QRectF rect) const;

    /// @brief 返回默认翻译辅助程序路径。
    /// @return 翻译辅助程序路径。
    QString defaultTranslationHelperProgram() const;

    /// @brief 取消正在运行的翻译子进程并清理临时文件。
    /// @return 无返回值。
    void cancelTranslation();

    /// @brief 处理翻译子进程结束结果。
    /// @param process 翻译子进程。
    /// @param output 标准输出。
    void finishTranslation(QProcess *process, const QByteArray &output);

    /// @brief 将窗口坐标转换为图片坐标。
    /// @param point 窗口内坐标。
    /// @return 图片坐标。
    QPointF widgetToImage(QPointF point) const;

    /// @brief 将图片矩形转换为窗口矩形。
    /// @param imageRect 图片坐标系矩形。
    /// @return 窗口坐标系矩形。
    QRectF imageToWidget(QRectF imageRect) const;

    /// @brief 返回图片坐标命中的 OCR token 索引。
    /// @param imagePoint 图片坐标。
    /// @return 命中的 token 索引。
    std::optional<int> tokenAt(QPointF imagePoint) const;

    /// @brief 返回距离图片坐标最近的 OCR token 索引。
    /// @param imagePoint 图片坐标。
    /// @return 最近的 token 索引。
    std::optional<int> closestToken(QPointF imagePoint) const;

    /// @brief 根据鼠标位置更新窗口光标。
    /// @param widgetPoint 鼠标在窗口内的位置。
    void updateCursorForPosition(QPointF widgetPoint);

    /// @brief 判断当前是否存在文本选择。
    /// @return 存在文本选择时返回 true。
    bool hasTextSelection() const;

    /// @brief 返回当前文本选择范围。
    /// @return 起止 token 索引。
    std::pair<int, int> selectionRange() const;

    /// @brief 清空当前文本选择。
    /// @return 无返回值。
    void clearTextSelection();

    /// @brief 返回当前选中文本。
    /// @return 选中文本。
    QString selectedText() const;

    /// @brief 返回当前全部 OCR 或翻译文本。
    /// @return 全部文本。
    QString allText() const;

    /// @brief 复制图片识别文本，必要时先启动 OCR。
    /// @return 无返回值。
    void copyImageText();

    /// @brief 返回指定 token 范围内的文本。
    /// @param first 起始 token 索引。
    /// @param last 结束 token 索引。
    /// @return 拼接后的文本。
    QString tokenRangeText(int first, int last) const;

    /// @brief 将窗口内部 token 转换为共享 OCR token。
    /// @param tokens 窗口内部 token 列表。
    /// @return 共享 OCR token 列表。
    QVector<markshot::ocr::Token> sharedOcrTokens(const QVector<OcrToken> &tokens) const;

    /// @brief 返回当前可选择的 OCR 或翻译 token。
    /// @return token 列表引用。
    const QVector<OcrToken> &activeTokens() const;

    /// @brief 判断当前是否可以请求翻译。
    /// @return 可以请求翻译时返回 true。
    bool canRequestTranslation() const;

    /// @brief 请求翻译当前 OCR 文本。
    /// @return 无返回值。
    void requestTranslation();

    /// @brief 设置翻译覆盖层是否激活。
    /// @param active 是否显示译文。
    void setTranslationActive(bool active);

    /// @brief 设置翻译子进程运行期间的忙碌光标。
    /// @param active 是否显示忙碌光标。
    void setTranslationBusyCursor(bool active);

    /// @brief 将翻译 token 拆分成可选择 token。
    /// @param tokens 翻译覆盖层 token。
    /// @return 可选择 token 列表。
    QVector<OcrToken> selectableTranslationTokens(const QVector<OcrToken> &tokens) const;

    /// @brief 将单个翻译 token 按字符拆分。
    /// @param token 翻译 token。
    /// @return 拆分后的 token 列表。
    QVector<OcrToken> splitTokenForSelection(const OcrToken &token) const;

    /// @brief 返回字符在选择拆分中的宽度权重。
    /// @param ch 字符。
    /// @return 宽度权重。
    qreal selectionCharacterWeight(QChar ch) const;

    /// @brief 绘制翻译覆盖层。
    /// @param painter 当前窗口画笔。
    void drawTranslationOverlay(QPainter &painter) const;

    QPixmap m_pixmap;
    QSize m_imageSize;
    QSize m_displayBaseSize;
    qreal m_scale = 1.0;
    QRect m_logicalGeometry;
    QPoint m_dragOffset;
    PinnedResizeDragState m_resizeDrag;
    PinnedWindowConfig m_config;
    QVector<OcrToken> m_ocrTokens;
    QVector<OcrToken> m_translatedTokens;
    QVector<OcrToken> m_translationOverlayTokens;
    QProcess *m_ocrProcess = nullptr;
    QProcess *m_translationProcess = nullptr;
    QString m_ocrTempPath;
    QString m_translationInputPath;
    int m_selectionAnchor = -1;
    int m_selectionFocus = -1;
    bool m_selectingText = false;
    bool m_translationActive = false;
    bool m_translateAfterOcr = false;
    bool m_copyTextAfterOcr = false;
    bool m_translationBusyCursor = false;
    bool m_activateTranslationWhenFinished = true;
    bool m_ocrBackendWarningShown = false;
    bool m_recreating = false;
};

}  // namespace markshot::shot
