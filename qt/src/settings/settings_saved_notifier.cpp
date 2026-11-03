#include "settings/settings_saved_notifier.h"

namespace markshot::settings {

SettingsSavedNotifier::SettingsSavedNotifier(QObject *parent)
    : QObject(parent)
{
}

SettingsSavedNotifier &SettingsSavedNotifier::instance()
{
    static SettingsSavedNotifier notifier;
    return notifier;
}

void SettingsSavedNotifier::notifySaved()
{
    emit saved();
}

}  // namespace markshot::settings
