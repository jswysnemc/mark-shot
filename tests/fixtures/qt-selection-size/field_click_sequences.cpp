#include "field_probe.h"

#include <QApplication>
#include <QEventLoop>
#include <QMouseEvent>
#include <QTimer>
#include <QWindow>
#include <qpa/qwindowsysteminterface.h>

namespace {

class Events final : public QObject {
public:
    QJsonArray values;

    /** 【字段点击对照】【事件记录】按 target 与 event 保存字段实际收到的类型、坐标和平台时间，返回继续分派 */
    bool eventFilter(QObject *target, QEvent *event) override
    {
        Q_UNUSED(target);
        QString type;
        switch (event->type()) {
        case QEvent::MouseButtonPress: type = QStringLiteral("press"); break;
        case QEvent::MouseButtonDblClick: type = QStringLiteral("double"); break;
        case QEvent::MouseButtonRelease: type = QStringLiteral("release"); break;
        case QEvent::MouseMove: type = QStringLiteral("move"); break;
        default: return false;
        }
        const auto *mouse = static_cast<QMouseEvent *>(event);
        values.append(QJsonObject{{"type", type}, {"button", static_cast<int>(mouse->button())},
                                  {"buttons", static_cast<int>(mouse->buttons())},
                                  {"point", QJsonArray{mouse->position().x(), mouse->position().y()}},
                                  {"timestamp", static_cast<qint64>(mouse->timestamp())}});
        return false;
    }
};

struct Input {
    QString action;
    QPointF point;
    int timestamp;
    int wait = 0;
};

/** 【字段点击对照】【平台事件】按 editor、point、timestamp、button、pressed 与 buttons 投递真实平台事件，无返回值 */
void platform(QLineEdit &editor, QPointF point, ulong timestamp, Qt::MouseButton button,
              bool pressed, Qt::MouseButtons buttons)
{
    QWidget *window = editor.window();
    const QPoint offset = editor.mapTo(window, QPoint());
    const QPointF local = QPointF(offset) + point;
    const QPointF global = QPointF(window->mapToGlobal(QPoint())) + local;
    QWindowSystemInterface::handleMouseEvent<QWindowSystemInterface::SynchronousDelivery>(
        window->windowHandle(), timestamp, local, global, buttons, button,
        button == Qt::NoButton ? QEvent::MouseMove : pressed ? QEvent::MouseButtonPress : QEvent::MouseButtonRelease);
    QApplication::processEvents();
}

/** 【字段点击对照】【完整序列】按 text、name、inputs 和 base 返回真实平台事件及每步状态，等待只用于原版字段计时器 */
QJsonObject sequence(const QString &text, const QString &name, const QList<Input> &inputs, ulong base)
{
    QWidget host;
    aspect::SelectionSizeField field(QStringLiteral("Width"), &host);
    QLineEdit &editor = prepare(host, field, text);
    Events events;
    editor.installEventFilter(&events);
    QWindowSystemInterface::handleEnterEvent(host.windowHandle(), QPointF(25, 10), host.mapToGlobal(QPoint(25, 10)));
    QWindowSystemInterface::handleFocusWindowChanged(host.windowHandle());
    QApplication::processEvents();
    QJsonArray steps;
    Qt::MouseButtons buttons;
    for (const Input &input : inputs) {
        // 1. 【字段点击对照】【墙钟等待】平台时间戳与字段计时器各自使用原版路径，不用伪造时间替代计时器过期
        if (input.wait > 0) {
            QEventLoop loop;
            QTimer::singleShot(input.wait, &loop, &QEventLoop::quit);
            loop.exec();
        }
        const auto before = state(editor);
        const int offset = events.values.size();
        const ulong timestamp = base + static_cast<ulong>(input.timestamp);
        if (input.action == QStringLiteral("left") || input.action == QStringLiteral("right")) {
            const auto button = input.action == QStringLiteral("left") ? Qt::LeftButton : Qt::RightButton;
            buttons |= button;
            platform(editor, input.point, timestamp, button, true, buttons);
            buttons &= ~button;
            platform(editor, input.point, timestamp + 1, button, false, buttons);
        } else if (input.action == QStringLiteral("move")) {
            platform(editor, input.point, timestamp, Qt::NoButton, false, buttons);
        } else if (input.action == QStringLiteral("up")) {
            key(editor, Qt::Key_Up, Qt::NoModifier);
        } else if (input.action == QStringLiteral("focus_out")) {
            host.setFocus();
            QApplication::processEvents();
        } else if (input.action == QStringLiteral("focus_in")) {
            field.beginEditing();
            QApplication::processEvents();
        }
        QJsonArray delivered;
        for (int index = offset; index < events.values.size(); ++index) {
            delivered.append(events.values.at(index));
        }
        steps.append(QJsonObject{{"action", input.action}, {"point", QJsonArray{input.point.x(), input.point.y()}},
                                 {"time_ms", input.timestamp}, {"wait_ms", input.wait},
                                 {"before", before}, {"after", state(editor)}, {"events", delivered},
                                 {"editing", editor.isVisible()}});
    }
    return {{"name", name}, {"text", text}, {"steps", steps}};
}

} // namespace

/** 【字段点击对照】【平台序列】按 texts 生成双击间距、三击位移、右键插入、墙钟过期和分数命中样本 */
QJsonArray click_sequences(const QStringList &texts)
{
    QJsonArray output;
    ulong base = 10000;
    const QPointF point(25, 10);
    const auto append = [&](const QString &text, const QString &name, const QList<Input> &inputs) {
        output.append(sequence(text, name, inputs, base));
        base += 2000;
    };
    for (const QString &text : texts) {
        // 1. 【字段点击对照】【平台双击】读取 Qt 自行生成的双击，包含整数边界、对角位移和分数位置
        for (const QPointF &delta : {QPointF(0, 0), QPointF(4, 0), QPointF(5, 0), QPointF(5.01, 0),
                                     QPointF(4, 4), QPointF(5, 5), QPointF(0, 5.01), QPointF(-5, 0)}) {
            append(text, QStringLiteral("double_distance"), {{"left", point, 0}, {"left", point + delta, 100}});
        }
        // 2. 【字段点击对照】【独立三击】第三次普通平台按下使用字段拖动距离，不复用双击的五像素门限
        for (const QPointF &delta : {QPointF(0, 0), QPointF(6, 0), QPointF(9, 0), QPointF(10, 0),
                                     QPointF(4, 4), QPointF(5, 5), QPointF(9.49, 0), QPointF(9.5, 0),
                                     QPointF(-9.49, 0), QPointF(-9.5, 0)}) {
            append(text, QStringLiteral("triple_distance"), {{"left", point, 0}, {"left", point, 100},
                                                              {"left", point + delta, 200}});
        }
        append(text, QStringLiteral("double_then_right"), {{"left", point, 0}, {"left", point, 100},
                                                            {"right", point, 150}, {"left", point, 200}});
        append(text, QStringLiteral("right_between_first_two"), {{"left", point, 0}, {"right", point, 50},
                                                                  {"left", point, 100}});
        append(text, QStringLiteral("move_between_first_two"), {{"left", point, 0}, {"move", point + QPointF(15, 0), 50},
                                                                 {"move", point, 75}, {"left", point, 100}});
        append(text, QStringLiteral("up_after_double"), {{"left", point, 0}, {"left", point, 100},
                                                         {"up", point, 150}, {"left", point, 200}});
        append(text, QStringLiteral("focus_after_double"), {{"left", point, 0}, {"left", point, 100},
                                                            {"focus_out", point, 150}, {"focus_in", point, 175},
                                                            {"left", point, 200}});
        for (int gap : {399, 400, 401}) {
            append(text, QStringLiteral("double_interval"), {{"left", point, 0}, {"left", point, gap}});
        }
        append(text, QStringLiteral("triple_native_timestamp_late"), {{"left", point, 0}, {"left", point, 100},
                                                                      {"left", point + QPointF(6, 0), 600}});
        append(text, QStringLiteral("triple_wall_timer_expired"), {{"left", point, 0}, {"left", point, 100},
                                                                   {"left", point + QPointF(6, 0), 200, 450}});
        append(text, QStringLiteral("triple_wall_timer_active"), {{"left", point, 0}, {"left", point, 100},
                                                                  {"left", point + QPointF(6, 0), 200, 100}});
        for (qreal x : {10.49, 10.5, 18.49, 18.5, 18.6, 25.49, 25.5, 33.49, 33.5, 40.49, 40.5}) {
            append(text, QStringLiteral("fractional_hit"), {{"left", QPointF(x, 10), 0}});
        }
    }
    return output;
}
