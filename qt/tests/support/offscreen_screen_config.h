#pragma once

#include <QByteArray>
#include <QDir>
#include <QFile>
#include <QTemporaryDir>

namespace markshot::test {

class OffscreenScreenConfig final {
public:
    /**
     * 【测试环境】【虚拟屏幕】写入配置并使用相对文件名，避免平台参数截断 Windows 盘符。
     * 此对象须在 QApplication 创建前构造，并在 QApplication 销毁后析构。
     * @param directory 配置文件所在的隔离目录，须比此对象存活更久。
     * @param configuration 虚拟屏幕布局的 JSON 内容。
     * @return 无返回值，初始化结果通过 isValid 读取。
     */
    OffscreenScreenConfig(const QTemporaryDir &directory, const QByteArray &configuration)
        : m_previousDirectory(QDir::currentPath()),
          m_previousPlatform(qgetenv("QT_QPA_PLATFORM")),
          m_platformWasSet(qEnvironmentVariableIsSet("QT_QPA_PLATFORM"))
    {
        // 1. 【测试环境】【配置写入】确保配置完整写入，并在平台插件读取前关闭文件
        if (!directory.isValid()) {
            return;
        }
        QFile config(directory.filePath(QStringLiteral("screens.json")));
        if (!config.open(QIODevice::WriteOnly)
            || config.write(configuration) != configuration.size()) {
            return;
        }
        config.close();

        // 2. 【测试环境】【参数配置】平台参数只传文件名，工作目录负责定位真实路径
        if (!QDir::setCurrent(directory.path())) {
            return;
        }
        qputenv("QT_QPA_PLATFORM", "offscreen:configfile=screens.json");
        m_valid = true;
    }

    /** 【测试环境】【环境恢复】在应用销毁后恢复原工作目录和平台参数，无参数和返回值。 */
    ~OffscreenScreenConfig()
    {
        if (!m_valid) {
            return;
        }
        // 1. 【测试环境】【目录恢复】先离开隔离目录，保证临时目录能够正常删除
        QDir::setCurrent(m_previousDirectory);
        // 2. 【测试环境】【参数恢复】恢复原值，并保留原变量未设置的状态
        if (m_platformWasSet) {
            qputenv("QT_QPA_PLATFORM", m_previousPlatform);
        } else {
            qunsetenv("QT_QPA_PLATFORM");
        }
    }

    /** 【测试环境】【配置状态】读取配置是否准备完成，无参数，返回初始化结果。 */
    bool isValid() const { return m_valid; }

    Q_DISABLE_COPY_MOVE(OffscreenScreenConfig)

private:
    QString m_previousDirectory;
    QByteArray m_previousPlatform;
    bool m_platformWasSet = false;
    bool m_valid = false;
};

} // namespace markshot::test
