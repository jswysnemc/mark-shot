#pragma once

#include <QColor>
#include <QVector>
#include <QWidget>

class QLineEdit;
class QMouseEvent;
class QPaintEvent;

namespace markshot::ui {

// Saturation x Value picker. Renders a 2D field for the active hue and
// emits the chosen S/V coordinates.
class SVField : public QWidget {
    /// @brief Qt meta-object macro for SVField.
    Q_OBJECT
public:
    explicit SVField(QWidget *parent = nullptr);

    int saturation() const { return m_sat; }
    int value() const { return m_val; }

    void setHue(int hue);
    void setSv(int sat, int val);

signals:
    void changed(int sat, int val);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;

private:
    void selectAtPos(QPoint pos);

    int m_hue = 0;
    int m_sat = 255;
    int m_val = 255;
};

// Horizontal hue gradient with a draggable knob.
class HueSlider : public QWidget {
    /// @brief Qt meta-object macro for HueSlider.
    Q_OBJECT
public:
    explicit HueSlider(QWidget *parent = nullptr);

    int hue() const { return m_hue; }
    void setHue(int hue);

signals:
    void changed(int hue);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;

private:
    void selectAtPos(int x);

    int m_hue = 0;
};

// Horizontal alpha track. The base color is opaque on the right and fully
// transparent on the left, with a checkerboard backdrop showing through.
class AlphaSlider : public QWidget {
    /// @brief Qt meta-object macro for AlphaSlider.
    Q_OBJECT
public:
    explicit AlphaSlider(QWidget *parent = nullptr);

    int alpha() const { return m_alpha; }
    void setAlpha(int alpha);
    void setBaseColor(const QColor &color);

signals:
    void changed(int alpha);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;

private:
    void selectAtPos(int x);

    int m_alpha = 255;
    QColor m_baseColor{255, 0, 0};
};

// Square preview swatch with a checkerboard backdrop so partial alpha is
// visually obvious.
class ColorSwatch : public QWidget {
    /// @brief Qt meta-object macro for ColorSwatch.
    Q_OBJECT
public:
    explicit ColorSwatch(QWidget *parent = nullptr);
    QColor color() const { return m_color; }
    void setColor(const QColor &color);

signals:
    void clicked();

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;

private:
    QColor m_color{255, 255, 255};
};

// Compact HSV+alpha picker with a hex field. No buttons; the host panel is
// expected to provide framing.
class ColorPicker : public QWidget {
    /// @brief Qt meta-object macro for ColorPicker.
    Q_OBJECT
public:
    explicit ColorPicker(QWidget *parent = nullptr);

    QColor color() const { return m_color; }
    void setColor(const QColor &color);

signals:
    void colorChanged(const QColor &color);

private:
    void emitColor();
    void rebuildHex();
    void onHexEdited();
    void refreshHistorySwatches();
    void updateCurrentHistorySwatch();

    QColor m_color{255, 0, 0, 255};
    int m_hue = 0;
    int m_sat = 255;
    int m_val = 255;
    int m_alpha = 255;
    bool m_settingColor = false;

    SVField *m_svField = nullptr;
    HueSlider *m_hueSlider = nullptr;
    AlphaSlider *m_alphaSlider = nullptr;
    ColorSwatch *m_swatch = nullptr;
    QLineEdit *m_hex = nullptr;
    QVector<ColorSwatch *> m_historySwatches;
    QVector<QColor> m_historyColors;
};

}  // namespace markshot::ui
