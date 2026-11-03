#include "probe.h"

/// 【窗口输入法对照】【输入事件】构造 preedit、commit、start 和 length 指定的事件，cursor 控制候选光标，返回操作
static Operation input(QString preedit = {}, QString commit = {}, int start = 0,
                       int length = 0, int cursor = -2)
{
    return {"input", preedit, commit, start, length, cursor};
}

QVector<Scenario> scenarios()
{
    QVector<Scenario> result;
    for (const QString &mode : {QString("multiline"), QString("singleline")}) {
        result.append({"consecutive-commits", mode, "base", -1, -1,
                       {input({}, QString::fromUtf8("你好")), input({}, QString::fromUtf8("您")),
                        {"undo"}, {"undo"}, {"redo"}, {"redo"}}});
        result.append({"preedit-cancel", mode, "base", -1, -1,
                       {input("ni"), input("nin"), input(), {"undo"}}});
        result.append({"selected-preedit", mode, "abcd", 3, 1,
                       {input("ni"), input(), {"undo"}, {"redo"}}});
        result.append({"backward-selection", mode, "abcd", 1, 3,
                       {input("ni"), input("nin", "X"), {"undo"}, {"undo"}, {"redo"}}});
        result.append({"commit-and-next", mode, "base", -1, -1,
                       {input("ni"), input("hao", QString::fromUtf8("你")),
                        input({}, QString::fromUtf8("好")), {"undo"}, {"undo"}, {"redo"}}});
        result.append({"empty-selected-event", mode, "abcd", 3, 1,
                       {input(), input({}, "X"), {"undo"}, {"redo"}}});
        result.append({"unicode-replacement", mode, QString::fromUtf8("aé𠮷b"), -1, -1,
                       {input({}, "X", -4, 3), {"undo"}, {"redo"}}});
        result.append({"commit-surrounding-replacement", mode, "abcde", 3, 3,
                       {input({}, "XY", -1, 2), {"undo"}, {"redo"}}});
        result.append({"selected-relative-replacement", mode, "abcdef", 4, 2,
                       {input({}, "X", -1, 2), {"undo"}, {"undo"}, {"redo"}}});
        result.append({"selected-typing", mode, "abcd", 3, 1,
                       {{"type", "XY"}, {"undo"}, {"redo"}}});
        result.append({"typing-then-commit", mode, "base", -1, -1,
                       {{"type", "XY"}, input({}, "Z"), {"undo"}, {"undo"}, {"redo"}}});
        result.append({"preedit-cursor-hidden", mode, "base", -1, -1,
                       {input(QString::fromUtf8("你𠮷"), {}, 0, 0, -1), input({}, "X"), {"undo"}}});
        result.append({"commit-newline", mode, "base", -1, -1,
                       {input({}, "a\nb\tc"), {"undo"}, {"redo"}}});
        result.append({"preedit-then-backspace", mode, "base", -1, -1,
                       {input("ni"), {"backspace"}, input(), {"undo"}}});
        result.append({"selected-paste", mode, "abcd", 3, 1,
                       {{"paste", "X\nY\tZ"}, {"undo"}, {"redo"}}});
        result.append({"selected-cut", mode, "abcd", 1, 3,
                       {{"cut"}, {"undo"}, {"redo"}}});
    }
    result.append({"paragraph-replacement", "multiline", "one\ntwo\nend", 7, 7,
                   {input({}, "X", -4, 5), {"undo"}, {"redo"}}});
    result.append({"paragraph-selected-preedit", "multiline", "one\ntwo\nend", 9, 2,
                   {input("ni"), input({}, "X"), {"undo"}, {"undo"}, {"redo"}}});
    result.append({"multiline-enter", "multiline", "base", -1, -1,
                   {input("ni"), {"enter"}, input(), {"undo"}, {"undo"}}});
    return result;
}
