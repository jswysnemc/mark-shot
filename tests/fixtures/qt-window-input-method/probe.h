#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QString>
#include <QVector>
#include <memory>

class QLineEdit;
class QTextEdit;
class QWidget;

struct Operation {
    QString kind;
    QString text;
    QString commit;
    int start = 0;
    int length = 0;
    int cursor = -2;
};

struct Scenario {
    QString name;
    QString mode;
    QString initial;
    int cursor = -1;
    int anchor = -1;
    QVector<Operation> operations;
};

/// 【窗口输入法对照】【场景集】返回纯文本和单行编辑器的真实输入法事件，不修改原版源码
QVector<Scenario> scenarios();
/// 【窗口输入法对照】【表格事件】返回真实表格的编辑激活、输入法和提交状态，无参数
QJsonArray tableScenarios();

class Probe {
public:
    /// 【窗口输入法对照】【控件构造】按 scenario 创建原版使用的 Qt 控件与初始选区，无返回值
    explicit Probe(const Scenario &scenario);
    ~Probe();
    /// 【窗口输入法对照】【状态采集】返回实际文本、UTF-8 字节光标、选区和撤销状态，无参数
    QJsonObject state() const;
    /// 【窗口输入法对照】【事件投递】投递 operation，返回用于 Rust 回放的事件参数
    QJsonObject apply(const Operation &operation);

private:
    std::unique_ptr<QWidget> widget;
    QTextEdit *multiline = nullptr;
    QLineEdit *singleline = nullptr;
};
