"""【原生发布】【清单校验】固定预览版来源和已经通过验证的二进制"""

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re


REPOSITORY = "jswysnemc/mark-shot"
BRANCH = "rewrite/native-wayland"
ARCHIVE_NAME = "mark-shot-native-linux-x86_64.tar.gz"
ARTIFACT_NAME = "mark-shot-native-linux-x86_64"
GUI_GROUPS = {
    "niri-layer": 5,
    "niri-window": 5,
    "sway-layer": 5,
    "sway-window": 6,
    "niri-caret": 1,
    "sway-caret": 1,
}
WINDOW_GROUPS = {"niri-window-ime", "sway-window-ime"}
HISTORY_GROUPS = {"niri-history", "niri-history-edges", "niri-history-pointer"}
HISTORY_DIALOG_GROUP = "niri-history-dialog"
SOURCE_MENU_GROUPS = {"niri-source-menu", "sway-source-menu"}
SOURCE_MENU_FLAGS = (
    "native_popup", "preserved_right_click_selection", "right_button_and_shortcut_isolation",
    "clipboard_after_popup_destruction", "copy_cut_paste_delete_undo_redo_select_all",
    "disabled_and_mnemonic_navigation", "outside_click_not_forwarded", "no_idle_frames", "no_ocr",
)


def require(condition, message):
    """【原生发布】【约束检查】condition 为假时抛出含 message 的异常，成功不返回值"""
    if not condition:
        raise ValueError(message)


def sha256_file(path):
    """【原生发布】【摘要计算】读取 path 的文件内容，返回 SHA-256 十六进制摘要"""
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


@dataclass(frozen=True)
class ReleaseManifest:
    """【原生发布】【不可变清单】保存发布标签、来源、产物与说明文档摘要"""

    tag: str
    source_commit: str
    run_id: int
    artifact_id: int
    archive_sha256: str
    binary_sha256: str
    binary_bytes: int
    verification_sha256: str
    notes_sha256: str

    @classmethod
    def load(cls, path):
        """【原生发布】【清单读取】解析 path 并拒绝未知字段及非法来源，返回经过验证的清单"""
        data = json.loads(Path(path).read_text())
        expected = set(cls.__dataclass_fields__) | {"schema", "repository", "branch"}
        require(set(data) == expected, "release manifest fields changed")
        require(type(data["schema"]) is int and data["schema"] == 1, "unsupported release schema")
        require(data["repository"] == REPOSITORY, "unexpected release repository")
        require(data["branch"] == BRANCH, "unexpected release branch")
        require(isinstance(data["tag"], str) and re.fullmatch(r"native-v\d+\.\d+\.\d+-preview\.\d+", data["tag"]), "only native preview tags are allowed")
        for name, length in (("source_commit", 40), ("archive_sha256", 64), ("binary_sha256", 64), ("verification_sha256", 64), ("notes_sha256", 64)):
            require(isinstance(data[name], str) and re.fullmatch(rf"[0-9a-f]{{{length}}}", data[name]), f"invalid {name}")
        for name in ("run_id", "artifact_id", "binary_bytes"):
            require(type(data[name]) is int and data[name] > 0, f"invalid {name}")
        require(data["binary_bytes"] <= 128 * 1024 * 1024, "binary size exceeds release limit")
        return cls(**{name: data[name] for name in cls.__dataclass_fields__})

    def verify_documents(self, verification, notes):
        """【原生发布】【证据校验】校验 verification 和 notes 对应本构建，返回界面验证报告"""
        require(sha256_file(verification) == self.verification_sha256, "verification document digest mismatch")
        require(sha256_file(notes) == self.notes_sha256, "release notes digest mismatch")
        report = json.loads(Path(verification).read_text())
        require(report.get("source_commit") == self.source_commit, "verification source mismatch")
        require(report.get("run_id") == self.run_id, "verification run mismatch")
        require(report.get("binary") == {"sha256": self.binary_sha256, "bytes": self.binary_bytes}, "verification binary mismatch")
        require(report.get("original_goal_complete") is False, "preview must not claim a complete rewrite")
        groups = report.get("gui_groups", [])
        require(len(groups) == len(GUI_GROUPS), "incomplete GUI verification")
        require({item.get("name") for item in groups} == set(GUI_GROUPS), "unexpected GUI verification groups")
        for group in groups:
            require(group.get("passed") is True, "failed GUI verification group")
            require(group.get("binary_sha256") == self.binary_sha256, "GUI group uses another binary")
            require(group.get("scenario_count") == GUI_GROUPS[group["name"]], "GUI scenario count mismatch")
        require(report.get("user_configuration_modified") is False, "GUI verification modified user configuration")
        require(report.get("original_qt_sources_modified") is False, "GUI verification modified original Qt sources")
        if int(self.tag.rsplit(".", 1)[1]) >= 2:
            windows = report.get("window_gui_groups", [])
            require(len(windows) == len(WINDOW_GROUPS) and {item.get("name") for item in windows} == WINDOW_GROUPS, "incomplete window input method verification")
            for window in windows:
                require(window.get("passed") is True and window.get("scenario_count") == 2, "failed window input method verification")
                require(window.get("binary_sha256") == self.binary_sha256, "window input method uses another binary")
                require(window.get("native_protocol") is True and window.get("no_ocr") is True and window.get("no_idle_surface_requests") is True, "missing native window isolation evidence")
        if int(self.tag.rsplit(".", 1)[1]) >= 3:
            histories = report.get("history_gui_groups", [])
            expected = HISTORY_GROUPS | ({HISTORY_DIALOG_GROUP} if int(self.tag.rsplit(".", 1)[1]) >= 4 else set())
            require(len(histories) == len(expected) and {item.get("name") for item in histories} == expected, "incomplete history verification")
            for history in histories:
                require(history.get("passed") is True, "failed history verification")
                require(history.get("binary_sha256") == self.binary_sha256, "history verification uses another binary")
                require(re.fullmatch(r"[0-9a-f]{64}", history.get("evidence_sha256", "")) is not None, "missing history evidence digest")
                if history.get("name") == HISTORY_DIALOG_GROUP:
                    result = history.get("result", {})
                    require(result.get("sha256") == self.binary_sha256, "dialog verification uses another binary")
                    for key in ("same_process_child", "native_parent_relationship", "default_enter_canceled", "wm_close_canceled", "parent_copy_delete_isolated", "parent_close_reclaimed_child", "space_release_confirmed_once", "unrelated_files_and_links_preserved"):
                        require(result.get(key) is True, f"missing dialog evidence: {key}")
        if int(self.tag.rsplit(".", 1)[1]) >= 5:
            for window in report["window_gui_groups"]:
                source = window.get("source", {})
                require(source.get("source_menu_preedit_canceled") is True and source.get("source_menu_stale_commit_ignored") is True, "missing source menu input method isolation")
            menus = report.get("source_menu_gui_groups", [])
            require(len(menus) == len(SOURCE_MENU_GROUPS) and {item.get("name") for item in menus} == SOURCE_MENU_GROUPS, "incomplete source menu verification")
            for menu in menus:
                require(menu.get("passed") is True and menu.get("binary_sha256") == self.binary_sha256, "source menu verification uses another binary or failed")
                require(re.fullmatch(r"[0-9a-f]{64}", menu.get("evidence_sha256", "")) is not None, "missing source menu evidence digest")
                result = menu.get("result", {})
                require(result.get("sha256") == self.binary_sha256 and result.get("bytes") == self.binary_bytes, "source menu result uses another binary")
                require(result.get("qt_menu_size") == [225, 229], "source menu geometry differs from Qt")
                for key in SOURCE_MENU_FLAGS:
                    require(result.get(key) is True, f"missing source menu evidence: {key}")
        return report
