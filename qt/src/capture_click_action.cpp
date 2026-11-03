#include "capture_click_action.h"

#include "config_value.h"

#include <utility>

namespace markshot {

std::optional<CaptureClickAction> captureClickActionFromText(QString value)
{
    const QString text = config::normalizedKey(std::move(value));
    if (text.isEmpty()) {
        return std::nullopt;
    }

    if (text == QStringLiteral("none")
        || text == QStringLiteral("disabled")
        || text == QStringLiteral("off")
        || text == QStringLiteral("noop")
        || text == QStringLiteral("nothing")) {
        return CaptureClickAction::None;
    }

    if (text == QStringLiteral("copy")
        || text == QStringLiteral("copyandclose")
        || text == QStringLiteral("copyclipboard")
        || text == QStringLiteral("clipboard")) {
        return CaptureClickAction::Copy;
    }

    if (text == QStringLiteral("save")
        || text == QStringLiteral("saveandclose")
        || text == QStringLiteral("quicksave")
        || text == QStringLiteral("savedefault")) {
        return CaptureClickAction::Save;
    }

    if (text == QStringLiteral("saveas")
        || text == QStringLiteral("savedialog")
        || text == QStringLiteral("saveasdialog")) {
        return CaptureClickAction::SaveAs;
    }

    if (text == QStringLiteral("pin")
        || text == QStringLiteral("pintoscreen")
        || text == QStringLiteral("pinned")) {
        return CaptureClickAction::Pin;
    }

    if (text == QStringLiteral("cancel")
        || text == QStringLiteral("close")
        || text == QStringLiteral("abort")
        || text == QStringLiteral("discard")) {
        return CaptureClickAction::Cancel;
    }

    return std::nullopt;
}

QString captureClickActionName(CaptureClickAction action)
{
    switch (action) {
    case CaptureClickAction::None:
        return QStringLiteral("none");
    case CaptureClickAction::Copy:
        return QStringLiteral("copy");
    case CaptureClickAction::Save:
        return QStringLiteral("save");
    case CaptureClickAction::SaveAs:
        return QStringLiteral("save-as");
    case CaptureClickAction::Pin:
        return QStringLiteral("pin");
    case CaptureClickAction::Cancel:
        return QStringLiteral("cancel");
    }
    return QStringLiteral("none");
}

}  // namespace markshot
