#pragma once

#include <QCommandLineParser>

namespace markshot::cli {

// Registers the headless capture options (--capture-to, --region, --display,
// --include-cursor, --list-displays) on the given parser.
void addHeadlessCaptureOptions(QCommandLineParser *parser);

// Runs the headless capture flow when --capture-to or --list-displays is
// present. Returns an application exit code, or -1 when no headless option is
// set and normal startup should continue.
int runHeadlessCaptureIfRequested(const QCommandLineParser &parser);

} // namespace markshot::cli
