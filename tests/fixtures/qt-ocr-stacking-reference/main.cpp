/** 【OCR 置顶对照】【真实窗口】只读连接完整原版，核对独立配置、按钮回退和状态期限，不使用用户配置 */
#include "app_config_store.h"
#include "window_detection.h"
#include "ocr_result_window/ocr_result_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QTemporaryDir>
#include <QtTest/QTest>
#include <iostream>

/** 【OCR 置顶对照】【状态采集】按 window 读取真实按钮、独立配置和提示，返回 JSON，不改写窗口成员 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    auto *pin = window.findChild<QPushButton *>("ocrPinButton");
    auto *status = window.findChild<QLabel *>("ocrStatus");
    const auto config = markshot::readAppConfigRoot();
    const QRect bounds(pin->mapTo(&window, QPoint()), pin->size());
    return {{"checked", pin->isChecked()}, {"enabled", pin->isEnabled()},
        {"accessible", pin->accessibleName()}, {"tooltip", pin->toolTip()},
        {"rect", QJsonArray{bounds.x(), bounds.y(), bounds.width(), bounds.height()}},
        {"status", status->text()}, {"status_visible", status->isVisible()},
        {"saved", config.value("ocrResultWindow").toObject().value("alwaysOnTop")},
        {"pinned_image_saved", config.value("pinnedWindow").toObject().value("alwaysOnTop")}};
}

/** 【OCR 置顶对照】【入口】argc、argv 指定私有输出目录，返回执行结果；实际原版代码和配置始终不修改 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    qputenv("QT_QPA_PLATFORM", "offscreen");
    qunsetenv("DBUS_SESSION_BUS_ADDRESS");
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}},
        {"pinnedWindow", QJsonObject{{"alwaysOnTop", true}}}})) return 3;
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("base", nullptr, image, {});
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(760, 520); window.show(); QTest::qWait(20);
    auto *pin = window.findChild<QPushButton *>("ocrPinButton");
    QJsonArray states;
    const auto record = [&](const QString &name) { states.append(QJsonObject{{"name", name}, {"state", snapshot(window)}}); };
    record("initial");
    pin->click(); QTest::qWait(20); record("enabled");
    pin->click(); QTest::qWait(20); record("disabled");
    QFile icon(":/icons/ui/pin.svg");
    if (!icon.open(QIODevice::ReadOnly)) return 4;
    QFile asset(output + "/pin.svg");
    if (!asset.open(QIODevice::WriteOnly) || asset.write(icon.readAll()) < 0) return 5;
    // 1. 【OCR 置顶对照】【私有失败】将隔离配置文件暂时替换为目录，制造真实保存失败，不改变原版保存逻辑
    const QString path = markshot::appConfigPath();
    if (!QFile::rename(path, path + ".backup") || !QDir().mkpath(path)) return 6;
    pin->click(); QTest::qWait(20); record("save-failed");
    QTest::qWait(2600); record("failure-expired");
    QFile result(output + "/result.json");
    if (!result.open(QIODevice::WriteOnly)) return 7;
    result.write(QJsonDocument(QJsonObject{{"states", states}, {"source", "actual OcrResultWindow"}}).toJson());
    std::cout << "recorded\n";
    return 0;
}
