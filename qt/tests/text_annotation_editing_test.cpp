#include "shot_window.h"
#include "support/isolated_config.h"
#include "text_annotation_layout.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QTemporaryDir>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextEdit>
#include <QTextLayout>
#include <QWheelEvent>
#include <QtTest>

#include <memory>

namespace {

/// 测试使用的截图选区，左上角留出选区外的空白
const QRect kSelection(100, 100, 400, 300);

/**
 * 在窗口坐标上完成一次左键单击。
 * @param window 截图窗口。
 * @param position 窗口坐标。
 * @return 无返回值。
 */
void clickAt(ShotWindow &window, QPoint position)
{
    QMouseEvent press(QEvent::MouseButtonPress, QPointF(position), QPointF(window.mapToGlobal(position)),
                      Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &press);
    QMouseEvent release(QEvent::MouseButtonRelease, QPointF(position), QPointF(window.mapToGlobal(position)),
                        Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &release);
}

/**
 * 在窗口坐标上发送若干格向上的滚轮事件。
 * @param window 截图窗口。
 * @param position 窗口坐标。
 * @param ticks 滚轮格数。
 * @return 无返回值。
 */
void wheelAt(ShotWindow &window, QPoint position, int ticks)
{
    for (int i = 0; i < ticks; ++i) {
        QWheelEvent event(QPointF(position), QPointF(window.mapToGlobal(position)), QPoint(), QPoint(0, 120),
                          Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
        QCoreApplication::sendEvent(&window, &event);
    }
}

/**
 * 向文字编辑器发送一次快捷键覆盖查询。
 * @param editor 文字编辑器。
 * @param key 按键。
 * @param modifiers 修饰键。
 * @return 事件被认领时返回 true。
 */
bool shortcutOverrideAccepted(QTextEdit *editor, Qt::Key key, Qt::KeyboardModifiers modifiers)
{
    QKeyEvent event(QEvent::ShortcutOverride, key, modifiers);
    event.ignore();
    QCoreApplication::sendEvent(editor, &event);
    return event.isAccepted();
}

/**
 * 判断文档中是否有段落排成多行。
 * @param document 编辑器文档。
 * @return 有段落换行时返回 true。
 */
bool documentWrapped(QTextDocument *document)
{
    document->size();
    for (QTextBlock block = document->begin(); block.isValid(); block = block.next()) {
        if (block.layout() && block.layout()->lineCount() > 1) {
            return true;
        }
    }
    return false;
}

}  // namespace

/// @brief 检查文字工具新建、再编辑、提交与换行的真实窗口行为
class TextAnnotationEditingTest final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建已确定选区并切到文字工具的截图窗口。
     * @param imageScale 冻结帧相对窗口的倍数，2 表示 HiDPI 下屏幕按一半显示。
     * @return 截图窗口，窗口未能显示时返回空指针。
     */
    static std::unique_ptr<ShotWindow> createTextWindow(int imageScale = 1)
    {
        // 1. 窗口固定 800x600，冻结帧按倍数放大，复现屏幕与导出字号不同的场景
        QImage image(800 * imageScale, 600 * imageScale, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(255, 255, 255));
        auto window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 800, 600),
                                                   QVector<markshot::WindowInfo>{}, false);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->resize(800, 600);
        window->show();
        if (!QTest::qWaitForWindowExposed(window.get())) {
            return nullptr;
        }

        // 2. 选区按图像坐标设置，再进入编辑并选用文字工具
        window->m_selection = QRectF(kSelection.topLeft() * imageScale, kSelection.size() * imageScale);
        window->enterSelectionEditing(QPointF(10, 10));
        window->setTool(ShotWindow::Tool::Text);
        return window;
    }

    /** 读取窗口撤销栈数量，参数为截图窗口，返回快照数量。 */
    static size_t undoCountOf(const ShotWindow &window)
    {
        return window.m_annotationHistory.info().undo_count;
    }

private slots:
    /** 验证选区外点击不会打开输入框，光标也提示不可输入，无参数和返回值。 */
    void ignoresClickOutsideSelection()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(40, 40));
        QVERIFY(!window->m_textEditor->isVisible());
        QVERIFY(!window->textToolAcceptsPoint(QPointF(40, 40)));
        QVERIFY(window->textToolAcceptsPoint(QPointF(200, 200)));
    }

    /** 验证点击编辑器外只提交当前文字，不会紧接着打开新的空输入框，无参数和返回值。 */
    void outsideClickOnlyCommits()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        QVERIFY(window->m_textEditor->isVisible());
        window->m_textEditor->insertPlainText(QStringLiteral("hello"));

        clickAt(*window, QPoint(150, 300));
        QVERIFY(!window->m_textEditor->isVisible());
        QCOMPARE(window->m_annotations.size(), 1);
        QCOMPARE(window->m_annotations.first().text, QStringLiteral("hello"));

        // 再点一次空白处才开始新的输入
        clickAt(*window, QPoint(150, 300));
        QVERIFY(window->m_textEditor->isVisible());
        QVERIFY(!window->m_editingTextAnnotationId.has_value());
    }

    /** 验证文字工具点在已有文字上进入该文字的编辑，而不是叠加新输入框，无参数和返回值。 */
    void clickOnTextEditsIt()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("hello"));
        window->commitTextEditor();
        QCOMPARE(window->m_annotations.size(), 1);
        const int id = window->m_annotations.first().id;

        const QPoint inside = window->textContentRect(window->m_annotations.first(), true).center().toPoint();
        clickAt(*window, inside);
        QVERIFY(window->m_textEditor->isVisible());
        QCOMPARE(window->m_editingTextAnnotationId, std::optional<int>(id));
        QCOMPARE(window->m_textEditor->toPlainText(), QStringLiteral("hello"));
        QCOMPARE(window->m_tool, ShotWindow::Tool::Text);

        // 提交后回到文字工具的新建状态，不残留选中
        window->commitTextEditor();
        QCOMPARE(window->m_annotations.size(), 1);
        QVERIFY(window->selectedAnnotationIds().isEmpty());
    }

    /** 验证清空已有文字后提交会删除该标注，且可撤销，无参数和返回值。 */
    void clearingTextRemovesAnnotation()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("hello"));
        window->commitTextEditor();
        const size_t before = undoCountOf(*window);

        window->setSelectedAnnotations({window->m_annotations.first().id});
        window->beginEditingSelectedTextAnnotation();
        window->m_textEditor->clear();
        window->m_textEditor->insertPlainText(QStringLiteral("   "));
        window->commitTextEditor();
        QVERIFY(window->m_annotations.isEmpty());
        QVERIFY(window->selectedAnnotationIds().isEmpty());
        QCOMPARE(undoCountOf(*window), before + 1);
    }

    /** 验证输入中滚动滚轮时编辑器立即换用新字号，提交结果与编辑时一致，无参数和返回值。 */
    void wheelResizesEditorWhileTyping()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("hello"));
        const qreal sizeBefore = window->m_textSize;
        const int pointBefore = window->m_textEditor->font().pointSize();
        const int widthBefore = window->m_textEditor->width();

        // 1. 文字每格 1.5，四格累计 +6
        wheelAt(*window, window->m_textEditor->geometry().center(), 4);
        QCOMPARE(window->m_textSize, sizeBefore + 6);
        QCOMPARE(window->m_textEditor->font().pointSize(), pointBefore + 6);
        QVERIFY(window->m_textEditor->width() > widthBefore);

        window->commitTextEditor();
        QCOMPARE(window->m_annotations.size(), 1);
        QCOMPARE(window->m_annotations.first().width, sizeBefore + 6);
    }

    /** 验证文字工具编辑已有文字时滚轮按该文字自身字号调整，且可一步撤销，无参数和返回值。 */
    void wheelResizesEditedTextFromItsOwnSize()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("hello"));
        window->commitTextEditor();
        const qreal ownSize = window->m_annotations.first().width;
        // 1. 默认字号与已有文字不同时，滚轮不能从默认字号起算
        window->m_textSize = ownSize + 20;

        const QPoint inside = window->textContentRect(window->m_annotations.first(), true).center().toPoint();
        clickAt(*window, inside);
        QVERIFY(window->m_editingTextAnnotationId.has_value());
        const int pointBefore = window->m_textEditor->font().pointSize();
        const size_t undoBefore = undoCountOf(*window);
        wheelAt(*window, inside, 2);
        QCOMPARE(window->m_annotations.first().width, ownSize + 3);
        QCOMPARE(window->m_textEditor->font().pointSize(), pointBefore + 3);

        // 2. 提交后字号保留，滚轮改动合并为一步撤销
        window->commitTextEditor();
        window->commitAnnotationWidthWheelHistory();
        QCOMPARE(window->m_annotations.first().width, ownSize + 3);
        QCOMPARE(undoCountOf(*window), undoBefore + 1);
        window->undoAnnotationEdit();
        QCOMPARE(window->m_annotations.first().width, ownSize);
    }

    /** 验证进入编辑后原样提交不会新增撤销记录，无参数和返回值。 */
    void unchangedEditKeepsHistory()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("  indented"));
        window->commitTextEditor();
        QCOMPARE(window->m_annotations.first().text, QStringLiteral("  indented"));
        const size_t before = undoCountOf(*window);

        window->setSelectedAnnotations({window->m_annotations.first().id});
        window->beginEditingSelectedTextAnnotation();
        window->commitTextEditor();
        QCOMPARE(undoCountOf(*window), before);
    }

    /** 提供屏幕与图像同尺寸、HiDPI 半尺寸两种缩放，无参数和返回值。 */
    void longTextStaysInsideSelection_data()
    {
        QTest::addColumn<int>("imageScale");
        QTest::newRow("1x") << 1;
        QTest::newRow("hidpi") << 2;
    }

    /** 验证长文字在选区右边界换行，提交和再编辑都保持同样换行且不超出选区，无参数和返回值。 */
    void longTextStaysInsideSelection()
    {
        QFETCH(int, imageScale);
        auto window = createTextWindow(imageScale);
        QVERIFY(window);
        const int selectionRight = kSelection.x() + kSelection.width();
        clickAt(*window, QPoint(300, 150));
        QVERIFY(window->m_textEditor->isVisible());
        window->m_textEditor->insertPlainText(
            QStringLiteral("The quick brown fox jumps over the lazy dog near the riverbank"));

        // 1. 编辑器在选区右边界内换行，不再延伸到窗口边缘
        QVERIFY(documentWrapped(window->m_textEditor->document()));
        QVERIFY(window->m_textEditor->geometry().x() + window->m_textEditor->width() <= selectionRight);

        // 2. 提交后保持换行，屏幕和导出坐标下的背景框都留在选区内
        window->commitTextEditor();
        QCOMPARE(window->m_annotations.size(), 1);
        const markshot::shot::types::Annotation committed = window->m_annotations.first();
        QVERIFY(markshot::shot::textAnnotationWraps(committed));
        QVERIFY2(committed.rect.right() <= window->normalizedSelection().right(),
                 qPrintable(QStringLiteral("rect right %1, selection right %2")
                                .arg(committed.rect.right())
                                .arg(window->normalizedSelection().right())));
        QVERIFY(window->textContentRect(committed, true).right() <= selectionRight + 1.0);

        // 3. 原样再编辑一次，换行宽度与撤销记录都不变
        const size_t before = undoCountOf(*window);
        window->setSelectedAnnotations({committed.id});
        window->beginEditingSelectedTextAnnotation();
        QVERIFY(documentWrapped(window->m_textEditor->document()));
        window->commitTextEditor();
        QCOMPARE(window->m_annotations.first().rect, committed.rect);
        QCOMPARE(undoCountOf(*window), before);
    }

    /** 验证单行文字切换粗体后仍保持单行，无参数和返回值。 */
    void boldKeepsSingleLine()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        window->setSelectedTextBold(false);
        clickAt(*window, QPoint(150, 150));
        window->m_textEditor->insertPlainText(QStringLiteral("Single line label"));
        window->commitTextEditor();
        const qreal heightBefore = window->m_annotations.first().rect.height();

        window->setSelectedAnnotations({window->m_annotations.first().id});
        window->setSelectedTextBold(true);
        const markshot::shot::types::Annotation bolded = window->m_annotations.first();
        QCOMPARE(bolded.fontWeight, QFont::DemiBold);
        QVERIFY(!markshot::shot::textAnnotationWraps(bolded));
        QCOMPARE(bolded.rect.height(), heightBefore);
    }

    /** 验证编辑文字时导出快捷键由截图窗口认领，有选中文字时 Ctrl+C 仍复制文字，无参数和返回值。 */
    void editorClaimsExportShortcuts()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        clickAt(*window, QPoint(150, 150));
        QTextEdit *editor = window->m_textEditor;
        editor->insertPlainText(QStringLiteral("hello"));

        QKeyEvent copy(QEvent::ShortcutOverride, Qt::Key_C, Qt::ControlModifier);
        QVERIFY(window->handleTextEditorActionShortcut(editor, &copy));
        QVERIFY(shortcutOverrideAccepted(editor, Qt::Key_S, Qt::ControlModifier));

        // 1. 普通字符和非导出组合键仍交给编辑器
        QKeyEvent plain(QEvent::ShortcutOverride, Qt::Key_C, Qt::NoModifier);
        QVERIFY(!window->handleTextEditorActionShortcut(editor, &plain));
        QKeyEvent selectAll(QEvent::ShortcutOverride, Qt::Key_A, Qt::ControlModifier);
        QVERIFY(!window->handleTextEditorActionShortcut(editor, &selectAll));

        // 2. 选中文字后 Ctrl+C 复制所选文字，不触发截图复制
        editor->selectAll();
        QKeyEvent copySelected(QEvent::ShortcutOverride, Qt::Key_C, Qt::ControlModifier);
        QVERIFY(!window->handleTextEditorActionShortcut(editor, &copySelected));
        QVERIFY(editor->isVisible());
    }

    /** 验证文字工具下双击不触发配置的选区双击动作，无参数和返回值。 */
    void textToolSkipsDoubleClickAction()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        window->m_doubleClickAction = markshot::CaptureDoubleClickAction::Copy;
        QVERIFY(!window->canRunDoubleClickAction(QPointF(200, 200)));
        window->setTool(ShotWindow::Tool::Pen);
        QVERIFY(window->canRunDoubleClickAction(QPointF(200, 200)));
    }

    /** 验证文字工具在选区外显示箭头、选区内显示插入光标，无参数和返回值。 */
    void cursorFollowsSelection()
    {
        auto window = createTextWindow();
        QVERIFY(window);
        window->updatePointerCursor(QPointF(40, 40));
        QCOMPARE(window->cursor().shape(), Qt::ArrowCursor);
        window->updatePointerCursor(QPointF(200, 200));
        QCOMPARE(window->cursor().shape(), Qt::IBeamCursor);
    }
};

/**
 * 使用隔离配置运行真实窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    TextAnnotationEditingTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "text_annotation_editing_test.moc"
