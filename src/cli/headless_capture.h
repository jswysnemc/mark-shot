#pragma once

#include <QCommandLineParser>

namespace markshot::cli {

/**
 * 注册无界面截图命令行参数。
 * @param parser 需要添加参数的命令行解析器。
 * @return 无返回值。
 */
void addHeadlessCaptureOptions(QCommandLineParser *parser);

/**
 * 在请求无界面截图或显示器列表时执行对应流程。
 * @param parser 已完成参数解析的命令行解析器。
 * @return 应用退出码；未请求无界面功能时返回 -1。
 */
int runHeadlessCaptureIfRequested(const QCommandLineParser &parser);

} // namespace markshot::cli
