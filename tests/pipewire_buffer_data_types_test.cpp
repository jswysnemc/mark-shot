#include "pipewire/pipewire_buffer_data_types.h"

#include <spa/buffer/buffer.h>

#include <QtTest/QtTest>

class PipeWireBufferDataTypesTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 modifier 格式只声明 DMA-BUF 类型。
     * @return 无返回值。
     */
    void modifierFormatUsesDmaBuf()
    {
        const std::uint32_t mask = markshot::pipewire::bufferDataTypeMask(true);

        QVERIFY(mask & (1u << SPA_DATA_DmaBuf));
        QVERIFY(!(mask & (1u << SPA_DATA_MemPtr)));
        QVERIFY(!(mask & (1u << SPA_DATA_MemFd)));
    }

    /**
     * 验证线性格式声明 CPU 可映射缓冲类型。
     * @return 无返回值。
     */
    void linearFormatUsesCpuMappableBuffers()
    {
        const std::uint32_t mask = markshot::pipewire::bufferDataTypeMask(false);

        QVERIFY(mask & (1u << SPA_DATA_MemPtr));
        QVERIFY(mask & (1u << SPA_DATA_MemFd));
    }

    /**
     * 验证 raw 录制优先协商无 modifier 的共享内存格式。
     * @return 无返回值。
     */
    void rawRecordingPrefersLinearFormat()
    {
        QCOMPARE(markshot::pipewire::modifierPreference(true),
                 (std::array<bool, 2>{false, true}));
        QCOMPARE(markshot::pipewire::modifierPreference(false),
                 (std::array<bool, 2>{true, false}));
    }
};

QTEST_MAIN(PipeWireBufferDataTypesTest)
#include "pipewire_buffer_data_types_test.moc"
