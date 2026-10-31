#include "app_config_store.h"
#include "clipboard_paste/clipboard_content.h"
#include "clipboard_paste/clipboard_pin_launch.h"
#include "clipboard_paste/clipboard_reader.h"
#include "pinned_window/pinned_image_window.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QBuffer>
#include <QClipboard>
#include <QMimeData>
#include <QTemporaryDir>
#include <QtTest>

class ClipboardContentTest final : public QObject {
    Q_OBJECT

private slots:
    /// @brief 图片优先于文字，并保留透明度和像素比例，无参数和返回值
    void preservesImage()
    {
        QImage image(80, 40, QImage::Format_ARGB32);
        image.fill(QColor(20, 70, 100, 140));
        image.setDevicePixelRatio(2);
        QMimeData mime;
        mime.setImageData(image);
        mime.setText(QStringLiteral("not the image"));
        QString error;
        const QImage result = markshot::clipboard::imageFromContent(mime, QFont(), QPalette(), &error);
        QCOMPARE(result, image);
        QCOMPARE(result.devicePixelRatio(), 2.0);
        QVERIFY(error.isEmpty());
    }

    /// @brief 支持原始 PNG 数据且不依赖 Qt 私有图像格式，无参数和返回值
    void decodesPng()
    {
        QImage image(80, 40, QImage::Format_RGB32);
        image.fill(Qt::red);
        QByteArray bytes;
        QBuffer buffer(&bytes);
        QVERIFY(buffer.open(QIODevice::WriteOnly));
        QVERIFY(image.save(&buffer, "PNG"));
        QMimeData mime;
        mime.setData(QStringLiteral("image/png"), bytes);
        QCOMPARE(markshot::clipboard::imageFromContent(mime, QFont(), QPalette()), image);
    }

    /// @brief 覆盖明暗主题、换行、长单词和 HTML 字面量，无参数和返回值
    void rendersText_data()
    {
        QTest::addColumn<bool>("dark");
        QTest::addColumn<QString>("text");
        QTest::newRow("light") << false << QStringLiteral("Hello\nClipboard");
        QTest::newRow("dark") << true << QStringLiteral("<b>plain text</b>");
        QTest::newRow("wrap") << false << QString(250, QLatin1Char('W'));
    }

    /// @brief 文字图像紧凑且使用传入配色，无参数和返回值
    void rendersText()
    {
        QFETCH(bool, dark);
        QFETCH(QString, text);
        QPalette palette;
        palette.setColor(QPalette::Base, dark ? Qt::black : Qt::white);
        palette.setColor(QPalette::Text, dark ? Qt::white : Qt::black);
        QMimeData mime;
        mime.setText(text);
        const QImage image = markshot::clipboard::imageFromContent(mime, QFont(), palette);
        QVERIFY(!image.isNull());
        QVERIFY(image.width() <= 520);
        QVERIFY(image.height() > 24);
        QCOMPARE(image.pixelColor(0, 0), palette.color(QPalette::Base));
        int ink = 0;
        for (int y = 12; y < image.height() - 12; ++y) {
            for (int x = 12; x < image.width() - 12; ++x) {
                ink += image.pixelColor(x, y) != palette.color(QPalette::Base);
            }
        }
        QVERIFY(ink > 0);
    }

    /// @brief 空内容和过长文字明确失败，不静默截断，无参数和返回值
    void rejectsInvalidContent()
    {
        QMimeData mime;
        QString error;
        QVERIFY(markshot::clipboard::imageFromContent(mime, QFont(), QPalette(), &error).isNull());
        QVERIFY(!error.isEmpty());
        mime.setText(QString(20001, QLatin1Char('x')));
        QVERIFY(markshot::clipboard::imageFromContent(mime, QFont(), QPalette(), &error).isNull());
        QVERIFY(!error.isEmpty());
        mime.setText(QStringLiteral("x\n").repeated(4000));
        QVERIFY(markshot::clipboard::imageFromContent(mime, QFont(), QPalette(), &error).isNull());
    }

    /// @brief 真实剪贴板入口显示贴图，不消耗或更改剪贴板，无参数和返回值
    void pinsClipboardWithoutCapture()
    {
        QVERIFY(markshot::writeAppConfigRoot({{QStringLiteral("pinnedWindow"),
            QJsonObject{{QStringLiteral("alwaysOnTop"), false}, {QStringLiteral("autoOcr"), false}}}}));
        QApplication::clipboard()->setText(QStringLiteral("Clipboard sticker"));
        const auto content = markshot::clipboard::readContent();
        QCOMPARE(content->text(), QStringLiteral("Clipboard sticker"));
        QString error;
        std::unique_ptr<QWidget> window(markshot::clipboard::pinContent(&error));
        QVERIFY2(window, qPrintable(error));
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        QVERIFY(window->isVisible());
        QVERIFY(dynamic_cast<markshot::shot::PinnedImageWindow *>(window.get()));
        QCOMPARE(QApplication::clipboard()->text(), QStringLiteral("Clipboard sticker"));
    }
};

/// @brief 使用隔离配置运行剪贴板贴图测试
/// @param argc 参数数量
/// @param argv 参数列表
/// @return 测试退出码
int main(int argc, char **argv)
{
    QTemporaryDir directory;
    if (!markshot::test::isolateConfig(directory)) {
        return 1;
    }
    QApplication app(argc, argv);
    ClipboardContentTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "clipboard_content_test.moc"
