#include "settings/settings_ui_helpers.h"

#include "ui/i18n.h"
#include "ui/theme.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFormLayout>
#include <QFrame>
#include <QKeySequenceEdit>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QDoubleSpinBox>
#include <QSpinBox>
#include <QVBoxLayout>

namespace markshot::settings {

QVBoxLayout *createSettingsPageLayout(QWidget *parent)
{
    auto *layout = new QVBoxLayout(parent);
    layout->setContentsMargins(20, 20, 20, 20);
    layout->setSpacing(14);
    return layout;
}

QFrame *createSettingsCard(const QString &title, const QString &description, QWidget *parent)
{
    auto *card = new QFrame(parent);
    card->setObjectName(QStringLiteral("settingsCard"));
    auto *layout = new QVBoxLayout(card);
    layout->setContentsMargins(16, 14, 16, 16);
    layout->setSpacing(8);

    auto *titleLabel = new QLabel(title, card);
    titleLabel->setObjectName(QStringLiteral("settingsCardTitle"));
    layout->addWidget(titleLabel);

    if (!description.isEmpty()) {
        auto *descriptionLabel = new QLabel(description, card);
        descriptionLabel->setObjectName(QStringLiteral("settingsCardDescription"));
        descriptionLabel->setWordWrap(true);
        layout->addWidget(descriptionLabel);
    }

    auto *form = new QFormLayout;
    form->setObjectName(QStringLiteral("settingsCardForm"));
    form->setLabelAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    form->setFormAlignment(Qt::AlignTop);
    form->setHorizontalSpacing(18);
    form->setVerticalSpacing(10);
    layout->addLayout(form);
    return card;
}

QFormLayout *settingsCardForm(QFrame *card)
{
    return card ? card->findChild<QFormLayout *>(QStringLiteral("settingsCardForm")) : nullptr;
}

QCheckBox *addSwitchRow(QFormLayout *form, const QString &label, const QString &description)
{
    auto *box = new QCheckBox(description);
    box->setCursor(Qt::PointingHandCursor);
    form->addRow(label, box);
    return box;
}

QLineEdit *addTextRow(QFormLayout *form, const QString &label, const QString &placeholder)
{
    auto *edit = new QLineEdit;
    edit->setPlaceholderText(placeholder);
    edit->setContextMenuPolicy(Qt::CustomContextMenu);
    QObject::connect(edit, &QLineEdit::customContextMenuRequested, edit, [edit](const QPoint &pos) {
        QMenu *menu = edit->createStandardContextMenu();
        menu->setStyleSheet(markshot::theme::menuStyleSheet());
        menu->exec(edit->mapToGlobal(pos));
        menu->deleteLater();
    });
    form->addRow(label, edit);
    return edit;
}

QPlainTextEdit *addPlainTextRow(QFormLayout *form, const QString &label, const QString &placeholder)
{
    auto *edit = new QPlainTextEdit;
    edit->setPlaceholderText(placeholder);
    edit->setMinimumHeight(74);
    edit->setContextMenuPolicy(Qt::CustomContextMenu);
    QObject::connect(edit, &QPlainTextEdit::customContextMenuRequested, edit, [edit](const QPoint &pos) {
        QMenu *menu = edit->createStandardContextMenu();
        menu->setStyleSheet(markshot::theme::menuStyleSheet());
        menu->exec(edit->viewport()->mapToGlobal(pos));
        menu->deleteLater();
    });
    form->addRow(label, edit);
    return edit;
}

QSpinBox *addSpinRow(QFormLayout *form, const QString &label, int minimum, int maximum, const QString &suffix)
{
    auto *spin = new QSpinBox;
    spin->setRange(minimum, maximum);
    spin->setSuffix(suffix);
    spin->setContextMenuPolicy(Qt::NoContextMenu);
    form->addRow(label, spin);
    return spin;
}

QDoubleSpinBox *addDoubleRow(QFormLayout *form, const QString &label, double minimum, double maximum, int decimals)
{
    auto *spin = new QDoubleSpinBox;
    spin->setRange(minimum, maximum);
    spin->setDecimals(decimals);
    spin->setContextMenuPolicy(Qt::NoContextMenu);
    form->addRow(label, spin);
    return spin;
}

QComboBox *addComboRow(QFormLayout *form, const QString &label)
{
    auto *combo = new QComboBox;
    combo->setCursor(Qt::PointingHandCursor);
    combo->setContextMenuPolicy(Qt::NoContextMenu);
    form->addRow(label, combo);
    return combo;
}

QKeySequenceEdit *addShortcutRow(QFormLayout *form, const QString &label)
{
    auto *edit = new QKeySequenceEdit;
    edit->setContextMenuPolicy(Qt::NoContextMenu);
    if (auto *le = edit->findChild<QLineEdit *>()) {
        le->setContextMenuPolicy(Qt::NoContextMenu);
    }
    form->addRow(label, edit);
    return edit;
}

QString colorButtonStyleSheet(const QColor &color)
{
    const QString name = color.isValid()
        ? color.name(QColor::HexRgb).toUpper()
        : markshot::theme::kDefaultAnnotationColor.name(QColor::HexRgb).toUpper();
    return QStringLiteral(
               "QPushButton {"
               " min-height: 30px;"
               " border-radius: 8px;"
               " border: 1px solid #334155;"
               " background: %1;"
               " color: #0F172A;"
               " font-weight: 700;"
               "}"
               "QPushButton:hover { border-color: #5EEAD4; }")
        .arg(name);
}

}  // namespace markshot::settings
