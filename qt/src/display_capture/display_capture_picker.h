#pragma once

#include "display_capture/display_capture_target.h"
#include "ui/toolbar_palette.h"

#include <QString>
#include <QVector>
#include <QWidget>

class QKeyEvent;
class QVBoxLayout;

namespace markshot::display_capture {

class DisplayCapturePicker final : public QWidget {
    Q_OBJECT

public:
    /**
     * 创建显示器快速截取选择器。
     * @param palette 截图会话的工具栏配色。
     * @param parent 父级窗口。
     */
    explicit DisplayCapturePicker(const ui::ToolbarPalette &palette, QWidget *parent = nullptr);
    void setTargets(const QVector<Target> &targets);

signals:
    void copyRequested(int index);
    void editRequested(int index);
    void saveRequested(int index);
    void dismissed();

protected:
    void keyPressEvent(QKeyEvent *event) override;

private:
    QVBoxLayout *m_cardLayout = nullptr;
};

}  // namespace markshot::display_capture
