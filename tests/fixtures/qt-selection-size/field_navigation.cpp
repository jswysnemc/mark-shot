#include "field_probe.h"

#include <QApplication>
#include <QClipboard>
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocale>
#include <QStyleHints>
#include <QWheelEvent>
#include "ui/i18n.h"

namespace markshot::i18n {
/** 【字段导航对照】【固定语言】按 source 返回英文标签，不读取用户配置 */
QString translate(const QString &source) { return source; }
}

struct KeyCase {
    const char *name;
    int key;
    Qt::KeyboardModifiers modifiers;
};

/** 【字段导航对照】【键盘样本】按 texts 返回各位置与选择方向的真实 Qt 按键结果 */
QJsonArray keyboard(const QStringList &texts)
{
    const Qt::KeyboardModifier c = Qt::ControlModifier, s = Qt::ShiftModifier, a = Qt::AltModifier;
    const KeyCase cases[] = {
        {"left", Qt::Key_Left, {}}, {"right", Qt::Key_Right, {}},
        {"word_left", Qt::Key_Left, c}, {"word_right", Qt::Key_Right, c},
        {"select_word_left", Qt::Key_Left, c | s}, {"select_word_right", Qt::Key_Right, c | s},
        {"delete_word_left", Qt::Key_Backspace, c}, {"delete_word_right", Qt::Key_Delete, c},
        {"alt_left", Qt::Key_Left, a}, {"alt_right", Qt::Key_Right, a},
        {"ctrl_alt_left", Qt::Key_Left, c | a}, {"ctrl_alt_right", Qt::Key_Right, c | a},
        {"ctrl_shift_backspace", Qt::Key_Backspace, c | s}, {"ctrl_shift_delete", Qt::Key_Delete, c | s},
        {"home", Qt::Key_Home, {}}, {"end", Qt::Key_End, {}},
        {"ctrl_home", Qt::Key_Home, c}, {"ctrl_end", Qt::Key_End, c},
        {"select_all", Qt::Key_A, c}, {"ctrl_shift_a", Qt::Key_A, c | s},
        {"ctrl_alt_a", Qt::Key_A, c | a}, {"shift_delete", Qt::Key_Delete, s},
    };
    QJsonArray samples;
    for (const QString &text : texts) {
        for (const QPair<int, int> selection : {qMakePair(0, 0), qMakePair(2, 0),
                                               qMakePair(static_cast<int>(text.size()), 0), qMakePair(1, 3), qMakePair(4, -3)}) {
            for (const KeyCase &input : cases) {
                QWidget host;
                aspect::SelectionSizeField field(QStringLiteral("Width"), &host);
                QLineEdit &editor = prepare(host, field, text);
                editor.setSelection(selection.first, selection.second);
                QApplication::clipboard()->setText(QStringLiteral("sentinel"));
                const QJsonObject before = state(editor);
                key(editor, input.key, input.modifiers);
                QJsonObject sample{{"name", QString::fromLatin1(input.name)}, {"before", before},
                                   {"after", state(editor)}, {"clipboard", QApplication::clipboard()->text()}};
                if (editor.isUndoAvailable()) {
                    key(editor, Qt::Key_Z, Qt::ControlModifier);
                    sample["undone"] = state(editor);
                }
                samples.append(sample);
            }
        }
    }
    return samples;
}

/** 【字段导航对照】【鼠标样本】按 texts 返回单击命中、双击、三击、双击拖选和右键状态 */
QJsonArray pointer(const QStringList &texts)
{
    QJsonArray samples;
    for (const QString &text : texts) {
        for (int x : {-4, 0, 3, 6, 10, 15, 20, 25, 30, 35, 40, 46, 50}) {
            for (const QString &action : {QStringLiteral("click"), QStringLiteral("double"),
                                         QStringLiteral("triple"), QStringLiteral("double_drag"), QStringLiteral("right"),
                                         QStringLiteral("up_drag"), QStringLiteral("wheel_drag"), QStringLiteral("double_up_click")}) {
                QWidget host;
                aspect::SelectionSizeField field(QStringLiteral("Width"), &host);
                QLineEdit &editor = prepare(host, field, text);
                const QPointF point(x, editor.height() / 2.0);
                const Qt::MouseButton button = action == QStringLiteral("right") ? Qt::RightButton : Qt::LeftButton;
                const QJsonObject initial = metrics(editor, x);
                mouse(editor, QEvent::MouseButtonPress, point, button, button);
                const QJsonObject hit = state(editor);
                const QJsonObject afterHit = metrics(editor, x);
                QJsonObject program;
                QJsonObject programMetrics;
                if (action == QStringLiteral("up_drag") || action == QStringLiteral("wheel_drag")) {
                    if (action == QStringLiteral("up_drag")) {
                        key(editor, Qt::Key_Up, Qt::NoModifier);
                    } else {
                        QWheelEvent event(point, editor.mapToGlobal(point.toPoint()), QPoint(0, 1), QPoint(),
                                          Qt::LeftButton, Qt::NoModifier, Qt::NoScrollPhase, false);
                        QApplication::sendEvent(&editor, &event);
                        QApplication::processEvents();
                    }
                    program = state(editor);
                    programMetrics = metrics(editor, x);
                    mouse(editor, QEvent::MouseMove, point - QPointF(20, 0), Qt::NoButton, Qt::LeftButton);
                    mouse(editor, QEvent::MouseButtonRelease, point - QPointF(20, 0), Qt::LeftButton, Qt::NoButton);
                } else {
                    mouse(editor, QEvent::MouseButtonRelease, point, button, Qt::NoButton);
                    if (action != QStringLiteral("click") && action != QStringLiteral("right")) {
                        mouse(editor, QEvent::MouseButtonDblClick, point, Qt::LeftButton, Qt::LeftButton);
                        if (action == QStringLiteral("double_drag")) {
                            mouse(editor, QEvent::MouseMove, point + QPointF(20, 0), Qt::NoButton, Qt::LeftButton);
                        }
                        mouse(editor, QEvent::MouseButtonRelease, point, Qt::LeftButton, Qt::NoButton);
                        if (action == QStringLiteral("triple")) {
                            mouse(editor, QEvent::MouseButtonPress, point, Qt::LeftButton, Qt::LeftButton);
                            mouse(editor, QEvent::MouseButtonRelease, point, Qt::LeftButton, Qt::NoButton);
                        }
                        if (action == QStringLiteral("double_up_click")) {
                            key(editor, Qt::Key_Up, Qt::NoModifier);
                            program = state(editor);
                            programMetrics = metrics(editor, x);
                            mouse(editor, QEvent::MouseButtonPress, point, Qt::LeftButton, Qt::LeftButton);
                            mouse(editor, QEvent::MouseButtonRelease, point, Qt::LeftButton, Qt::NoButton);
                        }
                    }
                }
                samples.append(QJsonObject{{"text", text}, {"x", x}, {"action", action},
                                           {"hit", hit}, {"after", state(editor)}, {"initial_metrics", initial}, {"hit_metrics", afterHit},
                                           {"program", program}, {"program_metrics", programMetrics}, {"after_metrics", metrics(editor, x)}});
            }
        }
    }
    return samples;
}

/** 【字段导航对照】【光标几何】按 texts 返回实际字段各字符边界的输入法矩形，不推算内部文字边距 */
QJsonArray carets(const QStringList &texts)
{
    QJsonArray result;
    for (const QString &text : texts) {
        QWidget host;
        aspect::SelectionSizeField field(QStringLiteral("Width"), &host);
        QLineEdit &editor = prepare(host, field, text);
        QJsonArray positions;
        for (int index = 0; index <= text.size(); ++index) {
            editor.setCursorPosition(index);
            QApplication::processEvents();
            const QRectF rect = editor.inputMethodQuery(Qt::ImCursorRectangle).toRectF();
            positions.append(QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()});
        }
        result.append(QJsonObject{{"text", text}, {"rects", positions}});
    }
    return result;
}

/** 【字段导航对照】【采集入口】argv 指定结果文件，返回写入状态；全部生产控件只读链接原版 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    app.setStyle("Fusion");
    QLocale::setDefault(QLocale::c());
    const QStringList texts = {QStringLiteral("12345"), QStringLiteral("+12345"), QStringLiteral("1,234"),
                               QStringLiteral("+1,234"), QStringLiteral("00012")};
    aspect::SelectionSizeField field(QStringLiteral("Width"));
    const QFontInfo font(field.font());
    QJsonObject output{{"qt", QString::fromLatin1(qVersion())}, {"font", font.family()},
                       {"dpi", field.logicalDpiX()}, {"click_interval", app.styleHints()->mouseDoubleClickInterval()},
                       {"click_distance", app.styleHints()->mouseDoubleClickDistance()},
                       {"drag_distance", app.styleHints()->startDragDistance()}};
    if (argc > 2 && QString::fromLocal8Bit(argv[2]) == QStringLiteral("--click-sequences")) {
        output["sequences"] = click_sequences(texts);
    } else {
        output["keys"] = keyboard(texts);
        output["mouse"] = pointer(texts);
        output["carets"] = carets(texts);
    }
    QFile file(QString::fromLocal8Bit(argv[1]));
    if (!file.open(QIODevice::WriteOnly)) {
        return 1;
    }
    file.write(QJsonDocument(output).toJson(QJsonDocument::Compact));
    file.write("\n");
    return 0;
}
