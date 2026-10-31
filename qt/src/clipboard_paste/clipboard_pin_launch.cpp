#include "clipboard_paste/clipboard_pin_launch.h"

#include "app_config_store.h"
#include "clipboard_paste/clipboard_content.h"
#include "clipboard_paste/clipboard_reader.h"
#include "pinned_window/pinned_image_window.h"
#include "pinned_window/pinned_non_window_chrome.h"
#include "settings/settings_design_tokens.h"
#include "ui/theme.h"

namespace markshot::clipboard {

QWidget *pinContent(QString *error)
{
    const auto mode = ui::effectiveUiThemeMode(ui::uiThemeModeFromConfigRoot(readAppConfigRoot()));
    const auto content = readContent();
    QImage image = imageFromContent(*content, theme::uiFont(11), settings::tokens::settingsPalette(mode), error);
    if (image.isNull()) {
        return nullptr;
    }
    auto *window = new shot::PinnedImageWindow(std::move(image));
    shot::presentPinnedImage(window);
    return window;
}

void pinContentOrNotify()
{
    QString error;
    if (!pinContent(&error)) {
        shot::sendDesktopNotification(QStringLiteral("Mark Shot"), error);
    }
}

}
