#include "pipewire/pipewire_dmabuf_policy.h"

#include <QtTest/QtTest>

namespace {

/**
 * 构造 KWin 搭配 NVIDIA 专有驱动的单显卡环境。
 * @return 环境探测结果。
 */
markshot::pipewire::DmaBufEnvironment kwinWithNvidia()
{
    markshot::pipewire::DmaBufEnvironment environment;
    environment.kdeSession = true;
    environment.nvidiaProprietaryDriver = true;
    environment.renderNodeCount = 1;
    return environment;
}

}  // namespace

class PipeWireDmaBufPolicyTest final : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 KWin 搭配 NVIDIA 专有驱动时避开 DMA-BUF。
     * @return 无返回值。
     */
    void avoidsDmaBufOnKwinWithNvidia()
    {
        QVERIFY(markshot::pipewire::shouldAvoidDmaBuf(kwinWithNvidia()));
    }

    /**
     * 验证混合显卡机器保持使用 DMA-BUF。
     * @return 无返回值。
     */
    void keepsDmaBufOnHybridGraphics()
    {
        markshot::pipewire::DmaBufEnvironment environment = kwinWithNvidia();
        // 存在第二个渲染节点时 KWin 通常渲染在集显上，DMA-BUF 可用
        environment.renderNodeCount = 2;
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf(environment));
    }

    /**
     * 验证其他桌面环境不受该规避影响。
     * @return 无返回值。
     */
    void keepsDmaBufOnOtherDesktops()
    {
        markshot::pipewire::DmaBufEnvironment environment = kwinWithNvidia();
        environment.kdeSession = false;
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf(environment));
    }

    /**
     * 验证没有 NVIDIA 专有驱动时不做规避。
     * @return 无返回值。
     */
    void keepsDmaBufWithoutNvidiaDriver()
    {
        markshot::pipewire::DmaBufEnvironment environment = kwinWithNvidia();
        environment.nvidiaProprietaryDriver = false;
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf(environment));
    }

    /**
     * 验证环境变量可以显式禁用 DMA-BUF。
     * @return 无返回值。
     */
    void environmentCanDisableDmaBuf()
    {
        markshot::pipewire::DmaBufEnvironment environment;
        environment.disabledByEnvironment = true;
        QVERIFY(markshot::pipewire::shouldAvoidDmaBuf(environment));
    }

    /**
     * 验证强制开关优先级高于自动规避。
     * @return 无返回值。
     */
    void forcedEnvironmentOverridesAvoidance()
    {
        markshot::pipewire::DmaBufEnvironment environment = kwinWithNvidia();
        environment.forcedByEnvironment = true;
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf(environment));

        environment.disabledByEnvironment = true;
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf(environment));
    }

    /**
     * 验证环境探测读取到的开关与环境变量一致。
     * @return 无返回值。
     */
    void readsEnvironmentSwitches()
    {
        qputenv("MARK_SHOT_DISABLE_DMABUF", "1");
        QVERIFY(markshot::pipewire::currentDmaBufEnvironment().disabledByEnvironment);
        QVERIFY(markshot::pipewire::shouldAvoidDmaBuf());
        qunsetenv("MARK_SHOT_DISABLE_DMABUF");

        qputenv("MARK_SHOT_FORCE_DMABUF", "1");
        QVERIFY(markshot::pipewire::currentDmaBufEnvironment().forcedByEnvironment);
        QVERIFY(!markshot::pipewire::shouldAvoidDmaBuf());
        qunsetenv("MARK_SHOT_FORCE_DMABUF");
    }
};

QTEST_APPLESS_MAIN(PipeWireDmaBufPolicyTest)

#include "pipewire_dmabuf_policy_test.moc"
