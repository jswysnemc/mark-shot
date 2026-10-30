#include "annotation_snapshot_codec.h"
#include "editor_history.h"
#include "support/isolated_config.h"

#include <QtTest>

namespace {

/** 构造 index 对应的完整 Qt 快照，返回含全部标注字段和选择状态的数据。 */
markshot::shot::types::HistorySnapshot sample(int index)
{
    using namespace markshot::shot::types;
    Annotation annotation;
    annotation.id = index;
    annotation.tool = Tool::Ruler;
    annotation.rect = QRectF(3745.1561043464453, 2.75, 40.5, 70.25);
    annotation.points = {{1.25, -2.5}, {70.75, 80.125}};
    annotation.text = QStringLiteral("第一行\nsecond line");
    annotation.number = 3;
    annotation.color = QColor(12, 34, 56, 128);
    annotation.backgroundColor = QColor(89, 123, 231, 64);
    annotation.width = 4.5;
    annotation.filled = true;
    annotation.cornerRadius = 8.5;
    annotation.arrowStyle = ArrowStyle::BidirectionalKde;
    annotation.arrowLineStyle = ArrowLineStyle::Dotted;
    annotation.highlighterStyle = HighlighterStyle::Freehand;
    annotation.rotationDegrees = -42.5;
    annotation.magnifierScale = 4.25;
    annotation.magnifierShape = MagnifierShape::Rectangle;
    annotation.numberStyle = NumberStyle::Chinese;
    annotation.fontFamily = QStringLiteral("Noto Sans CJK SC");
    annotation.fontWeight = QFont::Bold;
    annotation.textItalic = true;
    annotation.rectangleStyle = RectangleStyle::Spotlight;
    annotation.markerShape = MarkerShape::SpeechBubble;
    annotation.mosaicStyle = MosaicStyle::Brighten;
    annotation.rulerMode = RulerMode::Angle;
    HistorySnapshot snapshot;
    snapshot.annotations = {annotation};
    snapshot.selectedAnnotationId = index;
    snapshot.selectedAnnotationIds = {index, index + 1};
    snapshot.nextNumber = index + 10;
    snapshot.nextAnnotationId = index + 2;
    return snapshot;
}

} // namespace

/// 检查 Qt 数据转换与实际 Rust 历史会话之间的完整契约。
class RustEditorContractTest final : public QObject {
    Q_OBJECT

private slots:
    /** 验证完整字段在 JSON 往返和 Rust 撤销重做中保持一致，无参数、无返回值。 */
    void preservesCompleteSnapshot()
    {
        const auto before = sample(1);
        const auto after = sample(2);
        const auto encoded = markshot::rust::encodeSnapshot(before);
        const auto decoded = markshot::rust::decodeSnapshot(encoded);
        QVERIFY(decoded.has_value());
        QCOMPARE(markshot::rust::encodeSnapshot(*decoded), encoded);

        markshot::rust::EditorHistory history;
        QVERIFY(history.checkpoint(before));
        const auto undone = history.step(after, false);
        QVERIFY(undone.has_value());
        QCOMPARE(markshot::rust::encodeSnapshot(*undone), encoded);
        const auto redone = history.step(*undone, true);
        QVERIFY(redone.has_value());
        QCOMPARE(markshot::rust::encodeSnapshot(*redone), markshot::rust::encodeSnapshot(after));
    }

    /** 验证尚未到期的滚轮事务可以直接撤销，重复事件不会增加记录，无参数、无返回值。 */
    void coalescesWheelEventsBeforeTimerCommit()
    {
        markshot::rust::EditorHistory history;
        QVERIFY(history.queue(17, sample(1)));
        QVERIFY(history.queue(17, sample(2)));
        QCOMPARE(history.info().undo_count, size_t(0));
        QCOMPARE(history.info().pending, 1);
        const auto undone = history.step(sample(3), false);
        QVERIFY(undone.has_value());
        QCOMPARE(undone->nextAnnotationId, sample(1).nextAnnotationId);
        QCOMPARE(history.info().undo_count, size_t(0));
        QCOMPARE(history.info().redo_count, size_t(1));
        QCOMPARE(history.info().pending, 0);
    }

    /** 验证清空后迟到的计时器提交不能恢复旧文档，无参数、无返回值。 */
    void resetDiscardsPendingWheelTransaction()
    {
        markshot::rust::EditorHistory history;
        QVERIFY(history.checkpoint(sample(1)));
        QVERIFY(history.queue(4, sample(2)));
        history.clear();
        history.commit();
        QCOMPARE(history.info().undo_count, size_t(0));
        QCOMPARE(history.info().redo_count, size_t(0));
        QCOMPARE(history.info().pending, 0);
        QVERIFY(!history.step(sample(3), false).has_value());
    }

    /** 验证空画布和空选择在 Rust 往返后仍为空，无参数、无返回值。 */
    void preservesEmptyDocument()
    {
        markshot::rust::EditorHistory history;
        const markshot::shot::types::HistorySnapshot empty;
        QVERIFY(history.checkpoint(empty));
        const auto result = history.step(sample(1), false);
        QVERIFY(result.has_value());
        QVERIFY(result->annotations.isEmpty());
        QVERIFY(!result->selectedAnnotationId.has_value());
        QVERIFY(result->selectedAnnotationIds.isEmpty());
        QCOMPARE(result->nextNumber, 1);
        QCOMPARE(result->nextAnnotationId, 1);
    }
};

/** 在临时配置环境执行测试，argc/argv 为命令行参数，返回测试退出码。 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) {
        return 1;
    }
    QApplication application(argc, argv);
    RustEditorContractTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "rust_editor_contract_test.moc"
