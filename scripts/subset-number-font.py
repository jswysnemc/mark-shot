"""【序号字体】【构建入口】从有许可的思源黑体生成独立命名的二十三字形子集。"""

import argparse
import hashlib
import json
from pathlib import Path

from fontTools import __version__
from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

CHARACTERS = "零一二三四五六七八九十百千甲乙丙丁戊己庚辛壬癸"
FAMILY = "Mark Shot Numerals"
POSTSCRIPT = "MarkShotNumerals-Black"


def subset_font(source: Path, output: Path) -> dict:
    """【序号字体】【子集生成】用 source 字体生成 output；返回来源、字形和构建指纹。"""
    font = TTFont(source, recalcTimestamp=False)
    version = font["name"].getDebugName(5)
    missing = set(map(ord, CHARACTERS)) - set(font.getBestCmap())
    if missing:
        raise ValueError(f"source font is missing code points: {sorted(missing)}")

    options = Options()
    options.hinting = False
    options.desubroutinize = True
    options.layout_features = []
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14, 16, 17]
    options.name_languages = ["*"]
    options.drop_tables.extend(["BASE", "vhea", "vmtx", "VORG", "DSIG"])
    subsetter = Subsetter(options=options)
    subsetter.populate(text=CHARACTERS)
    subsetter.subset(font)

    names = {
        1: FAMILY,
        2: "Black",
        3: POSTSCRIPT,
        4: f"{FAMILY} Black",
        6: POSTSCRIPT,
        16: FAMILY,
        17: "Black",
    }
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    if "CFF " in font:
        cff = font["CFF "].cff
        cff.fontNames[0] = POSTSCRIPT
        top = cff.topDictIndex[0]
        top.FullName = f"{FAMILY} Black"
        top.FamilyName = FAMILY
        if hasattr(top, "FDArray"):
            for index, dictionary in enumerate(top.FDArray):
                dictionary.FontName = f"MarkShotNumerals-FD{index}"
    output.parent.mkdir(parents=True, exist_ok=True)
    font.save(output, reorderTables=True)
    with TTFont(output) as result:
        assert set(result.getBestCmap()) == set(map(ord, CHARACTERS))
        assert result["name"].getDebugName(1) == FAMILY
    return {
        "source": str(source),
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "source_version": version,
        "fonttools_version": __version__,
        "characters": CHARACTERS,
        "character_count": len(CHARACTERS),
        "output": str(output),
        "bytes": output.stat().st_size,
        "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
    }


def main() -> None:
    """【序号字体】【命令执行】解析来源和输出参数，生成子集并打印可复现指纹；无返回值。"""
    parser = argparse.ArgumentParser(description="Build the bundled numbering font subset")
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=Path("assets/fonts/MarkShotNumerals.otf"))
    args = parser.parse_args()
    print(json.dumps(subset_font(args.source, args.output), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
