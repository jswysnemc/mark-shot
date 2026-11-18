"""【输入法回归】【运行准备】编译私有 Rime 词库并隔离总线服务，不读取用户输入法数据"""

import subprocess
from xml.sax.saxutils import escape


def prepare_rime(root, environment):
    """【输入法回归】【词库部署】按 root 和 environment 预先编译系统拼音方案；无返回值，首次部署不能与输入回归竞争"""
    data = root / "data/fcitx5/rime"
    data.mkdir(parents=True, exist_ok=True)
    (data / "default.custom.yaml").write_text(
        "patch:\n  schema_list:\n    - schema: luna_pinyin\n"
    )
    with (root / "rime-deployment.log").open("w") as log:
        subprocess.run(
            [
                "rime_deployer",
                "--build",
                str(data),
                "/usr/share/rime-data",
                str(data / "build"),
            ],
            env=environment,
            stdout=log,
            stderr=log,
            check=True,
            timeout=30,
        )


def private_bus(root):
    """【输入法回归】【总线隔离】按 root 创建不自动启动桌面服务的 D-Bus 配置；返回配置路径，不连接用户总线"""
    config = root / "dbus.conf"
    config.write_text(
        "<busconfig><type>session</type>"
        f"<listen>unix:tmpdir={escape(str(root))}</listen>"
        '<auth>EXTERNAL</auth><policy context="default">'
        '<allow send_destination="*" eavesdrop="true"/>'
        '<allow eavesdrop="true"/><allow own="*"/>'
        "</policy></busconfig>"
    )
    return config


def remote(environment, *arguments):
    """【输入法回归】【状态查询】用 environment 的独立总线执行 arguments；返回 Fcitx 状态文字，超时即失败"""
    return subprocess.check_output(
        ["fcitx5-remote", *arguments],
        env=environment,
        text=True,
        stderr=subprocess.DEVNULL,
        timeout=3,
    ).strip()
