// 【滚轮协议】【实际转换】只记录 Qt 平台后端收到的事件，不修改原版对象和事件分派

#pragma once
#include <QApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTimer>
#include <QWheelEvent>
#include <QWidget>

class AxisReferenceWindow final : public QWidget {
public:
    /** 【滚轮协议】【窗口构造】使用 prefix 保存真实事件，parent 为窗口父对象，无返回值。 */
    explicit AxisReferenceWindow(QString prefix, QWidget *parent = nullptr) : QWidget(parent), m_prefix(std::move(prefix))
    {
        setWindowTitle(QStringLiteral("Mark Shot axis reference"));
        resize(600, 400);
    }

protected:
    /** 【滚轮协议】【事件记录】将 event 原始角度、像素、方向和阶段追加到 JSON，无返回值，不合成或重新发送事件。 */
    void wheelEvent(QWheelEvent *event) override
    {
        m_events.append(QJsonObject{{"angle", QJsonArray{event->angleDelta().x(), event->angleDelta().y()}},
            {"pixels", QJsonArray{event->pixelDelta().x(), event->pixelDelta().y()}},
            {"inverted",event->inverted()},{"phase",event->phase()},{"source",event->source()}});
        QFile file(m_prefix + "-axis.json");
        if (file.open(QIODevice::WriteOnly)) file.write(QJsonDocument(m_events).toJson());
        event->accept();
    }

private:
    QString m_prefix;
    QJsonArray m_events;
};
