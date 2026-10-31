#pragma once

#include "pinned_window/pinned_resize_controller.h"
#include "shot_window_internal.h"
#include "translation_layout/paragraph_blocks.h"
#include "translation_layout/translated_block.h"

#include <QByteArray>
#include <QImage>
#include <QJsonArray>
#include <QPainter>
#include <QPainterPath>
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
class SelectionTranslateTest;
class PinnedTextSelectionTest;

namespace markshot::translation_layout {
struct TranslationSeed;
}

namespace markshot::providers {
class ProviderTask;
struct TaskResult;
}

namespace markshot::shot {

class PinnedNativeResize;
class PinnedLayerShellDragPreview;

/// @brief 提供置顶图片显示、文本识别、文本选择、翻译和边界缩放的窗口。
class PinnedImageWindow final : public QWidget {
    friend class ::SelectionTranslateTest;
    friend class ::PinnedTextSelectionTest;

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

    /// @brief 使用当前显示方向的图片打开图片编辑视图，贴图窗口保持不变。
    /// @return 新建的图片编辑窗口，图片为空时返回空指针。
    ShotWindow *openImageEditor();

    /// @brief 使用截图窗口翻译模式交出的识别结果与译文，不再重新识别；没有识别结果时不做任何事
    /// @param seed 坐标相对贴图图片的识别结果、译文和当前是否显示译文
    /// @return 无返回值
    void applyTranslationSeed(const markshot::translation_layout::TranslationSeed &seed);

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
    /// @brief 指针重新进入时清理已经结束的原生窗口拖动光标
    /// @param event 指针进入事件
    /// @return 无返回值
    void enterEvent(QEnterEvent *event) override;

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
    /// @brief 绘制完整图片及覆盖层，供原窗口和拖动预览共同使用
    /// @param painter 目标窗口的绘制器
    /// @param viewport 目标窗口内的可见绘制区域
    /// @return 无返回值
    void paintImageContents(QPainter &painter, QRectF viewport);

    /// @brief 为 layer-shell 拖动创建无输入预览，保持原窗口的输入坐标原点
    /// @return 无返回值
    void beginLayerShellDragPreview();

    /// @brief 将预览的最终几何应用回原窗口，并在原图绘制后回收预览
    /// @return 无返回值
    void finishLayerShellDragPreview();

    /// @brief 将固定输入窗口的局部坐标换算为当前图片可见区域内坐标
    /// @param position 鼠标相对原始输入窗口的位置
    /// @return 当前图片可见区域内的逻辑位置
    QPointF pinnedLocalPointForInput(QPointF position) const;

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

    /// @brief 保存并应用文本拖选复制权限。
    /// @param enabled 是否允许通过鼠标拖选复制 OCR 或翻译文本。
    void setTextSelectionCopyEnabled(bool enabled);

    /// @brief 设置并保存翻译目标语言。
    /// @param targetLanguage 实际传给翻译器的目标语言名称。
    void setTranslationTargetLanguage(QString targetLanguage);

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

    /// @brief 返回完整贴图的逻辑显示尺寸。
    /// @return 逻辑显示尺寸。
    QSize logicalPinnedSize() const;

    /// @brief 返回完整图片在当前 QWidget 内的绘制矩形。
    /// @return 图片绘制矩形。
    QRectF displayedImageRect() const;

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

    /**
     * 延迟检查并重建目标屏幕上的 layer-shell surface。
     * @return 无返回值。
     */
    void scheduleLayerShellScreenRebind();

    /**
     * 在当前逻辑几何对应的屏幕上重建 layer-shell surface。
     * @return 无返回值。
     */
    void rebindLayerShellScreen();

    /// @brief 判断鼠标位置命中的缩放边界方向。
    /// @param widgetPoint 鼠标在窗口内的位置。
    /// @return 缩放边界方向。
    PinnedResizeDirection resizeDirectionAt(QPointF widgetPoint) const;

    /// @brief 判断贴边状态下是否应屏蔽指定缩放方向。
    /// @param direction 缩放方向。
    /// @return 应让位给窗口移动时返回 true。
    bool shouldBlockResizeAtEmbeddedEdge(PinnedResizeDirection direction) const;

    /// @brief 在首次显示前准备 KDE 原生缩放及图片尺寸同步
    /// @return 无返回值
    void initializeNativeResize();

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

    /// @brief 在贴图可见时后台准备识别结果，已有结果或任务时不重复启动
    /// @return 无返回值
    void scheduleBackgroundOcr();

    /// @brief 异步启动 OCR 任务，不阻塞鼠标交互
    /// @param textSelectionOnly 是否仅为拖选准备结果，不自动翻译或弹出后端缺失通知
    /// @return 无返回值
    void startOcr(bool textSelectionOnly = false);

    /// @brief 返回默认 OCR 辅助程序路径。
    /// @return OCR 辅助程序路径。
    QString defaultOcrHelperProgram() const;

    /// @brief 取消正在运行的 OCR 子进程并清理临时文件。
    /// @return 无返回值。
    void cancelOcr();

    /// @brief 处理 OCR 任务结束结果。
    /// @param task OCR 任务。
    /// @param result 任务结果。
    void finishOcr(markshot::providers::ProviderTask *task,
                   const markshot::providers::TaskResult &result);

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

    /// @brief 把识别结果合并成段落后启动翻译子进程。
    /// @param activateWhenFinished 翻译完成后是否直接显示译文。
    /// @param showBusyCursor 是否显示忙碌光标。
    void startTranslation(bool activateWhenFinished = true, bool showBusyCursor = true);

    /// @brief 返回默认翻译辅助程序路径。
    /// @return 翻译辅助程序路径。
    QString defaultTranslationHelperProgram() const;

    /// @brief 取消正在运行的翻译子进程并清理临时文件。
    /// @return 无返回值。
    void cancelTranslation();

    /// @brief 处理翻译任务结束结果。
    /// @param task 翻译任务。
    /// @param output 翻译输出 JSON。
    void finishTranslation(markshot::providers::ProviderTask *task, const QByteArray &output);

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

    /// @brief 复制当前选中文本。
    /// @return 无返回值。
    void copySelectedText();

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

    /// @brief 返回用于鼠标命中和选区绘制的 token 矩形。
    /// @param token OCR 或翻译 token。
    /// @return 经过全角字符宽度补正后的图片坐标矩形。
    QRectF selectionImageRectForToken(const OcrToken &token) const;

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

    /// @brief 将 OCR 或译文行 token 按字符拆分成可选择 token。
    /// @param tokens OCR 或译文行 token。
    /// @return 可选择 token 列表。
    QVector<OcrToken> selectableTranslationTokens(const QVector<OcrToken> &tokens) const;

    /// @brief 使用排好的译文段落作为翻译覆盖层，并生成可拖选的译文 token。
    /// @param blocks 排好的译文段落，图片像素坐标。
    /// @return 无返回值。
    void setTranslationBlocks(QVector<markshot::translation_layout::TranslatedBlock> blocks);

    /// @brief 将单个翻译 token 按字符拆分。
    /// @param token 翻译 token。
    /// @return 拆分后的 token 列表。
    QVector<OcrToken> splitTokenForSelection(const OcrToken &token) const;

    /// @brief 绘制翻译覆盖层。
    /// @param painter 当前窗口画笔。
    void drawTranslationOverlay(QPainter &painter) const;

    QPixmap m_pixmap;
    QSize m_imageSize;
    QSize m_displayBaseSize;
    qreal m_scale = 1.0;
    QRect m_logicalGeometry;
    QRect m_layerShellVisibleGeometry;
    QRect m_layerShellDragInputGeometry;
    PinnedLayerShellDragPreview *m_layerShellDragPreview = nullptr;
    bool m_layerShellDragPreviewActive = false;
    bool m_layerShellScreenRebindInProgress = false;
    QPoint m_layerShellContentOffset;
    QPoint m_dragOffset;
    PinnedResizeDragState m_resizeDrag;
    PinnedNativeResize *m_nativeResize = nullptr;
    PinnedWindowConfig m_config;
    QVector<OcrToken> m_ocrTokens;
    QVector<OcrToken> m_translatedTokens;
    // 翻译覆盖层：排好的译文段落，以及本次送去翻译的原文段落
    QVector<markshot::translation_layout::TranslatedBlock> m_translationBlocks;
    QVector<markshot::translation_layout::SourceBlock> m_translationSourceBlocks;
    markshot::providers::ProviderTask *m_ocrTask = nullptr;
    markshot::providers::ProviderTask *m_translationTask = nullptr;
    QString m_ocrTempPath;
    QString m_translationInputPath;
    int m_selectionAnchor = -1;
    int m_selectionFocus = -1;
    bool m_selectingText = false;
    bool m_textSelectionOcrAttempted = false;
    bool m_ocrForTextSelection = false;
    bool m_moving = false;
    bool m_translationActive = false;
    bool m_translateAfterOcr = false;
    bool m_copyTextAfterOcr = false;
    bool m_translationBusyCursor = false;
    bool m_activateTranslationWhenFinished = true;
    bool m_ocrBackendWarningShown = false;
    bool m_recreating = false;
    bool m_layerShellScreenRebindPending = false;

    // --- 新增：非窗口贴图的悬停边缘 ---
    bool m_pinChromeHovered = false;
    /// @brief 把贴图改成不抢焦点的工具层。
    /// @return 无返回值。
    void applyPinnedNonWindowChromeIfEnabled();
    /// @brief 根据进入或离开更新边缘强调状态。
    /// @param event 窗口事件。
    /// @return 无返回值。
    void updatePinnedNonWindowHoverIfEnabled(QEvent *event);
    /// @brief 绘制悬停和未悬停时的边缘与阴影。
    /// @param painter 当前画笔。
    /// @param imageRect 图片显示矩形。
    /// @return 已接管边框绘制时返回 true。
    bool paintPinnedNonWindowChromeIfEnabled(QPainter &painter, const QRectF &imageRect);

    // OCR 原文仅保存文字坐标，用于逐字拖选
    QVector<OcrToken> m_ocrSelectionTokens;
    /// @brief 选中同一行的全部可拖选文字。
    /// @param tokenIndex 命中的文字下标。
    /// @return 无返回值。
    void selectTokenLine(int tokenIndex);
    /// @brief 返回当前拖选在窗口坐标中的区域。
    /// @param viewport 需要裁剪到的窗口范围。
    /// @return 选区路径，没有选区时为空。
    QPainterPath textSelectionWidgetPath(QRectF viewport) const;

    // --- 新增：贴图阴影 ---
    int m_shadowMargin = 0;
    /// @brief 按配置为贴图留出阴影边距并改为透明背景。
    /// @return 无返回值。
    void applyPinnedShadowLayoutIfEnabled();
    /// @brief 把图片矩形收进阴影边距内。
    /// @param rect 窗口中的图片矩形。
    /// @return 无返回值。
    void insetPinnedImageForShadowIfEnabled(QRectF *rect) const;
    /// @brief 按阴影边距扩展窗口几何，使图片位置保持不变。
    /// @param topLeft 图片左上角。
    /// @param size 图片尺寸。
    /// @return 无返回值。
    void expandPinnedGeometryForShadowIfEnabled(QPoint *topLeft, QSize *size) const;
    /// @brief 在图片后方绘制柔和阴影。
    /// @param painter 当前画笔。
    /// @param imageRect 图片显示矩形。
    /// @return 无返回值。
    void paintPinnedShadowIfEnabled(QPainter &painter, const QRectF &imageRect) const;
};

}  // namespace markshot::shot
