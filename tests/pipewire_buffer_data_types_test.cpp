#include "pipewire/pipewire_buffer_data_types.h"
#include "pipewire/pipewire_drm_fourcc.h"

#include <spa/buffer/buffer.h>
#include <spa/param/video/raw.h>

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
        qunsetenv("MARK_SHOT_DISABLE_DMABUF");
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
        qunsetenv("MARK_SHOT_DISABLE_DMABUF");
        const std::uint32_t mask = markshot::pipewire::bufferDataTypeMask(false);

        QVERIFY(mask & (1u << SPA_DATA_MemPtr));
        QVERIFY(mask & (1u << SPA_DATA_MemFd));
    }

    /**
     * 验证 MARK_SHOT_DISABLE_DMABUF 强制共享内存缓冲类型。
     * @return 无返回值。
     */
    void disableDmabufForcesSharedMemoryBuffers()
    {
        qputenv("MARK_SHOT_DISABLE_DMABUF", "1");
        const std::uint32_t mask = markshot::pipewire::bufferDataTypeMask(true);
        QVERIFY(mask & (1u << SPA_DATA_MemPtr));
        QVERIFY(mask & (1u << SPA_DATA_MemFd));
        QVERIFY(!(mask & (1u << SPA_DATA_DmaBuf)));
        QCOMPARE(markshot::pipewire::modifierPreference(true),
                 (std::array<bool, 2>{false, false}));
        qunsetenv("MARK_SHOT_DISABLE_DMABUF");
    }

    /**
     * 验证 raw 录制优先协商无 modifier 的共享内存格式。
     * @return 无返回值。
     */
    void rawRecordingPrefersLinearFormat()
    {
        qunsetenv("MARK_SHOT_DISABLE_DMABUF");
        QCOMPARE(markshot::pipewire::modifierPreference(true),
                 (std::array<bool, 2>{false, true}));
        QCOMPARE(markshot::pipewire::modifierPreference(false),
                 (std::array<bool, 2>{true, false}));
    }

    /**
     * 验证 labwc 常见的 SPA format 8 (BGRx) 可映射 DRM fourcc。
     * @return 无返回值。
     */
    void spaBgRxMapsToDrmFourcc()
    {
        const auto fourcc =
            markshot::pipewire::drmFourccForSpaFormat(SPA_VIDEO_FORMAT_BGRx);
        QVERIFY(fourcc.has_value());
        QCOMPARE(*fourcc, static_cast<std::uint32_t>(DRM_FORMAT_XRGB8888));
        QCOMPARE(static_cast<int>(SPA_VIDEO_FORMAT_BGRx), 8);
    }
};

QTEST_MAIN(PipeWireBufferDataTypesTest)
#include "pipewire_buffer_data_types_test.moc"
