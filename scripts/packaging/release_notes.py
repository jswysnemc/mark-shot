"""【原生发布】【页面说明】组合版本更新说明、源码标识与折叠安装包摘要列表"""

from html import escape
from pathlib import Path
from urllib.parse import quote

from artifact_metadata import checksum


def render(notes, context, files, repository):
    """【原生发布】【说明生成】按 notes、context、files 和 repository 返回含下载链接及 SHA-256 的 Markdown"""
    description = Path(notes).read_text(encoding="utf-8").strip()
    if not description:
        raise ValueError("release update description must not be empty")
    base = f"https://github.com/{repository}"
    lines = [
        description,
        "",
        f"Source / 源码：[`{context.commit}`]({base}/commit/{context.commit})",
        "",
        "<details>",
        "<summary>Files and SHA-256 / 文件列表与 SHA-256</summary>",
        "",
        "| File / 文件 | Bytes / 字节 | SHA-256 |",
        "| --- | ---: | --- |",
    ]
    for path in sorted(files, key=lambda item: item.name):
        url = f"{base}/releases/download/{quote(context.tag, safe='')}/{quote(path.name, safe='')}"
        lines.append(
            f"| [{escape(path.name)}]({url}) | {path.stat().st_size} | `{checksum(path)}` |"
        )
    lines.extend(
        [
            "",
            "</details>",
            "",
            "SHA-256 values are shown here; checksum files are not release assets.  "
            "SHA-256 在此页面展示，不再单独上传摘要文件。",
            "",
        ]
    )
    return "\n".join(lines)
