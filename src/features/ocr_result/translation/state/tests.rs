//! 【OCR 翻译测试】【完整状态】六个原版宽度的十二状态逐项比较，旧任务不得覆盖新任务或修改原文

use super::*;
use crate::translation::{ErrorKind, Provider, Segment};
use serde_json::Value;

/// 【OCR 翻译测试】【固定结果】按 text 返回本地命令输出，无网络或用户配置
fn output(text: &str) -> Output {
    Output {
        provider: Provider::Command,
        segments: vec![Segment {
            id: 0,
            text: text.into(),
        }],
    }
}

/// 【OCR 翻译测试】【错误数据隔离】中文提示只转换原版标准错误，服务标识和任意详情不参与界面字典查询，无参数、无返回值
#[test]
fn localized_notices_preserve_arbitrary_provider_and_error_values() {
    let mut state = State::default();
    state.notice = Some(Notice::Failed(
        "Translation failed (command): Copy %1".into(),
    ));
    assert_eq!(
        state.localized_notice(crate::ui::language::Language::Chinese),
        "翻译失败（command）：Copy %1"
    );
    state.notice = Some(Notice::Failed(
        "Translation failed (unknown provider): timed out".into(),
    ));
    assert_eq!(
        state.localized_notice(crate::ui::language::Language::Chinese),
        "翻译失败（未知 Provider）：超时"
    );
    state.notice = Some(Notice::Busy);
    assert_eq!(
        state.localized_notice(crate::ui::language::Language::Chinese),
        "翻译中..."
    );
    assert_eq!(
        state.localized_notice(crate::ui::language::Language::English),
        "Translating..."
    );
}

/// 【OCR 翻译测试】【原版状态序列】全部 72 个记录的显示、只读与提示匹配原版，不假设宽度改变业务状态，无参数、无返回值
#[test]
fn state_sequence_matches_all_actual_qt_window_records() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ocr-translation.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut state = State::default();
        let mut translated = String::new();
        for step in case["states"].as_array().unwrap() {
            let widgets = &step["state"]["widgets"];
            let source = widgets["ocrEditor"]["text"].as_str().unwrap();
            let target = widgets["ocrLanguageCombo"]["text"].as_str().unwrap();
            match step["name"].as_str().unwrap() {
                "initial" => {}
                "busy" => {
                    assert!(state.begin(1, source, target));
                }
                "busy-source-edited" => {
                    state.refresh(source, target);
                }
                "canceled" => {
                    assert_eq!(state.cancel(source, target, true), Some(1));
                }
                "translated" => {
                    assert!(state.begin(2, source, target));
                    translated = state
                        .finish(
                            2,
                            Ok(output("Translated first\nTranslated second")),
                            source,
                            target,
                        )
                        .unwrap();
                }
                "source-stale" | "language-stale" | "fresh-again" => {
                    state.refresh(source, target);
                }
                "translation-edited" => {
                    translated.push_str(" edited");
                }
                "translation-copy-feedback" => {}
                "hidden-kept-text" => {
                    state.cancel(source, target, false);
                    state.visible = false;
                }
                "reopened-without-request" => {
                    state.visible = true;
                }
                other => panic!("unexpected reference step: {other}"),
            }
            assert_eq!(
                state.visible,
                widgets["ocrTranslationPane"]["visible"].as_bool().unwrap(),
                "{step}"
            );
            assert_eq!(
                state.busy(),
                widgets["ocrTranslationEditor"]["read_only"]
                    .as_bool()
                    .unwrap(),
                "{step}"
            );
            assert_eq!(
                translated,
                widgets["ocrTranslationEditor"]["text"].as_str().unwrap(),
                "{step}"
            );
            assert_eq!(
                state.notice(),
                widgets["translation:ocrPaneNotice"]["text"]
                    .as_str()
                    .unwrap(),
                "{step}"
            );
            assert_eq!(
                state.error(),
                widgets["translation:ocrPaneNotice"]["error"]
                    .as_bool()
                    .unwrap(),
                "{step}"
            );
        }
    }
}

/// 【OCR 翻译测试】【完整原文快照】过期依据完整 trim 后正文，内部空白变化仍过期，忙碌中继续编辑不覆盖原文，无参数、无返回值
#[test]
fn freshness_uses_complete_source_not_flattened_service_segments() {
    let mut state = State::default();
    state.begin(1, " A\tB\n second ", "English");
    assert_eq!(
        state.finish(1, Ok(output("translated")), "A\tB\n second", "English"),
        Some("translated".into())
    );
    assert_eq!(state.notice(), "");
    state.refresh("A B\n second", "English");
    assert_eq!(state.notice.as_ref(), Some(&Notice::Stale));
    state.refresh(" A\tB\n second ", "English");
    assert_eq!(state.notice(), "");
    state.begin(2, "A\tB\n second", "English");
    state.refresh("edited while busy", "English");
    assert_eq!(state.notice(), "Translating...");
    state.finish(
        2,
        Ok(output("new translation")),
        "edited while busy",
        "English",
    );
    assert_eq!(state.notice.as_ref(), Some(&Notice::Stale));
}

/// 【OCR 翻译测试】【晚到结果隔离】取消后的旧任务和重复完成不能改变新任务，失败保留已有成功快照且不伪装成成功，无参数、无返回值
#[test]
fn late_results_and_failures_never_replace_current_task_identity() {
    let mut state = State::default();
    state.begin(1, "first", "English");
    state.cancel("first", "English", true);
    state.begin(2, "second", "English");
    assert!(
        state
            .finish(1, Ok(output("old")), "second", "English")
            .is_none()
    );
    assert!(state.busy());
    assert_eq!(
        state.finish(2, Ok(output("new")), "second", "English"),
        Some("new".into())
    );
    assert!(
        state
            .finish(2, Ok(output("duplicate")), "second", "English")
            .is_none()
    );
    state.begin(3, "third", "English");
    state.finish(
        3,
        Err(Error::new(ErrorKind::Timeout, "timed out")),
        "third",
        "English",
    );
    assert!(state.error());
    let notice = state.notice().to_owned();
    state.refresh("second", "English");
    assert_eq!(state.notice(), notice);
    assert!(!state.busy());
}
