#pragma once

#include <QString>

namespace markshot::i18n {

enum class Language {
    English,
    Chinese,
};

// Detects the UI language from the MARK_SHOT_LANG environment variable, then
// falls back to the system locale. Call once after QApplication is created.
void initialize();

void setLanguage(Language language);
Language language();

// Returns the localized text for the given English source string. Unknown
// strings are returned unchanged, so the English text doubles as the lookup
// key and any missing translation falls back cleanly.
QString translate(const QString &source);

}  // namespace markshot::i18n

// Convenience wrapper for translating compile-time literals at call sites.
#define MS_TR(text) (::markshot::i18n::translate(QStringLiteral(text)))
