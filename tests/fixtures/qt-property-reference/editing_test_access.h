// 【原版探针】【友元入口】集中声明测试访问入口，数值与编辑事务各自保留独立实现

#pragma once
#include "shot_window.h"
#include <QJsonArray>
#include <QJsonObject>

class TextAnnotationEditingTest {
public:
    /** 【字号对照】【已有文字】向 window 加入选中文字，无返回值，不调用私有桌面捕获。 */
    static void select(ShotWindow &window);
    /** 【字号对照】【完整样本】按 source 创建真实窗口，返回各输入入口和撤销后的实际字号与框宽。 */
    static QJsonArray sizes(const QImage &source);
    /** 【文字编辑对照】【完整样本】按 source 返回真实编辑、提交和撤销序列，不代写原版业务逻辑。 */
    static QJsonArray editing(const QImage &source);
    /** 【文字菜单对照】【完整样本】按 source 执行真实菜单动作，prefix 保存菜单缓冲；返回实际状态和动作可用性。 */
    static QJsonArray menus(const QImage &source, const QString &prefix);
    /** 【文字格式对照】【完整样本】按 source 返回实际样式变化、局部撤销与提交状态，不修改原版对象。 */
    static QJsonArray formats(const QImage &source);
    /** 【文字光标对照】【完整样本】按 source 与 prefix 记录真实闪烁、焦点与选区，返回时序和像素统计。 */
    static QJsonObject caret(const QImage &source, const QString &prefix);
    /** 【输入法对照】【完整样本】按 source 与 prefix 记录真实预编辑、提交、替换和撤销，返回事件与状态序列 */
    static QJsonArray inputMethod(const QImage &source, const QString &prefix);
    /** 【字段输入法对照】【完整样本】从 source 创建原版字段，prefix 保存画面，返回真实候选、验证器与确认状态 */
    static QJsonArray lineInputMethod(const QImage &source, const QString &prefix);
private:
    /** 【输入法对照】【状态记录】读取 window 文档、预编辑与上下文，并保存 path 画面，返回实际状态 */
    static QJsonObject inputMethodState(ShotWindow &window, const QString &path);
    /** 【字号对照】【控件滚轮】向 source 对应的真实控件发送高分辨率和像素滚轮，将状态追加到 result，无返回值。 */
    static void controls(const QImage &source, QJsonArray &result);
    /** 【字号对照】【窗口构造】按 source 和 wrapped 返回原版编辑窗口，使用固定文字和私有配置。 */
    static std::unique_ptr<ShotWindow> create(const QImage &source, bool wrapped);
    /** 【字号对照】【状态记录】读取 window 选中对象、默认字号和余量；返回真实数值，不计算期望结果。 */
    static QJsonObject state(const ShotWindow &window);
    /** 【字号对照】【滚轮发送】把 delta 作为角度或 pixels 像素事件送入 window，无返回值。 */
    static void wheel(ShotWindow &window, int delta, bool pixels);
    /** 【字号对照】【撤销记录】对 window 连续撤销 count 次，返回每次实际状态，默认值不手动还原。 */
    static QJsonArray undo(ShotWindow &window, int count);
    /** 【文字编辑对照】【窗口准备】按 source 与 existing 返回实际编辑窗口，已有标注不替换业务逻辑。 */
    static std::unique_ptr<ShotWindow> editingWindow(const QImage &source, bool existing);
    /** 【文字编辑对照】【状态记录】返回 window 的持久文字、编辑器状态和选中标识。 */
    static QJsonObject editingState(const ShotWindow &window);
    /** 【文字格式对照】【格式状态】读取 window 的默认样式、编辑器字形格式和持久对象，返回实际状态。 */
    static QJsonObject formatState(const ShotWindow &window);
    /** 【文字格式对照】【窗口准备】按 source、existing 构造仅一次开始的实际编辑器，返回隔离窗口。 */
    static std::unique_ptr<ShotWindow> formatWindow(const QImage &source, bool existing);
    /** 【文字编辑对照】【按键发送】向 window 发送 key、modifiers 与 text，无返回值。 */
    static void editorKey(ShotWindow &window, int key, Qt::KeyboardModifiers modifiers, const QString &text = {});
};
