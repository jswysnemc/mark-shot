#include "shot_window_module.h"

using namespace markshot::shot;

void ShotWindow::initializeShortcuts()
{
    auto shortcutBlockedByTextInput = [this] {
        if (m_textEditor && m_textEditor->isVisible()) {
            return true;
        }
        QWidget *focusWidget = QApplication::focusWidget();
        return qobject_cast<QLineEdit *>(focusWidget) != nullptr
            || qobject_cast<QTextEdit *>(focusWidget) != nullptr;
    };
    auto addPlainShortcut = [this, shortcutBlockedByTextInput](const QKeySequence &sequence, auto callback) {
        if (sequence.isEmpty()) {
            return;
        }
        auto *shortcut = new QShortcut(sequence, this);
        shortcut->setContext(Qt::WindowShortcut);
        shortcut->setAutoRepeat(false);
        connect(shortcut, &QShortcut::activated, this, [this, shortcutBlockedByTextInput, callback] {
            if (shortcutBlockedByTextInput()) {
                return;
            }
            callback();
        });
    };
    auto addToolShortcut = [this, addPlainShortcut](Tool tool) {
        const QKeySequence sequence = this->shortcutForTool(tool);
        addPlainShortcut(sequence, [this, tool, sequence] {
            if (m_mode == Mode::Selecting && sequence == m_startupColorPickerShortcut) {
                setStartupTool(StartupTool::ColorPicker);
                return;
            }
            if (m_mode == Mode::Selecting && sequence == m_startupRulerShortcut) {
                setStartupTool(StartupTool::Ruler);
                return;
            }
            if (m_mode == Mode::Selecting && sequence == m_startupCodeScannerShortcut) {
                setStartupTool(StartupTool::CodeScanner);
                return;
            }
            if (m_mode == Mode::Selecting && sequence == m_startupDisplayCaptureShortcut) {
                toggleDisplayCapturePicker();
                return;
            }
            setTool(tool);
        });
    };
    for (Tool tool : {Tool::Move,
                      Tool::Select,
                      Tool::Pen,
                      Tool::Line,
                      Tool::Highlighter,
                      Tool::Rectangle,
                      Tool::Ellipse,
                      Tool::Arrow,
                      Tool::Text,
                      Tool::Number,
                      Tool::Mosaic,
                      Tool::Magnifier,
                      Tool::Laser}) {
        addToolShortcut(tool);
    }
    auto sequenceUsedByTool = [this](const QKeySequence &sequence) {
        for (const QKeySequence &toolSequence : m_toolShortcuts) {
            if (!sequence.isEmpty() && sequence == toolSequence) {
                return true;
            }
        }
        return false;
    };
    if (!sequenceUsedByTool(m_startupColorPickerShortcut)) {
        addPlainShortcut(m_startupColorPickerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::ColorPicker);
            }
        });
    }
    if (!sequenceUsedByTool(m_startupRulerShortcut)) {
        addPlainShortcut(m_startupRulerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::Ruler);
            }
        });
    }
    if (!sequenceUsedByTool(m_startupCodeScannerShortcut)) {
        addPlainShortcut(m_startupCodeScannerShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                setStartupTool(StartupTool::CodeScanner);
            }
        });
    }
    if (!sequenceUsedByTool(m_startupDisplayCaptureShortcut)) {
        addPlainShortcut(m_startupDisplayCaptureShortcut, [this] {
            if (m_mode == Mode::Selecting) {
                toggleDisplayCapturePicker();
            }
        });
    }
    auto addActionShortcut = [this, addPlainShortcut](Action action, auto callback) {
        addPlainShortcut(this->shortcutForAction(action), callback);
    };
    addActionShortcut(Action::ToggleCaptureScope, [this] { toggleCaptureScope(); });
    addActionShortcut(Action::Pin, [this] { pinSelection(); });
    addActionShortcut(Action::Copy, [this] {
        commitTextEditor();
        copySelection();
    });
    addActionShortcut(Action::Save, [this] {
        commitTextEditor();
        saveSelection();
    });
    addActionShortcut(Action::Undo, [this] { undoAnnotationEdit(); });
    addActionShortcut(Action::Redo, [this] { redoAnnotation(); });
    addActionShortcut(Action::OpenWith, [this] { toggleOpenWithPanel(); });
    addActionShortcut(Action::Extensions, [this] { toggleExtensionPanel(); });
    addActionShortcut(Action::ScrollCapture, [this] { startScrollCapture(); });
    addActionShortcut(Action::OcrCopy, [this] { ocrCopySelection(); });
    addActionShortcut(Action::Upload, [this] { uploadSelection(); });
    addActionShortcut(Action::Clear, [this] { clearAnnotations(); });
    addActionShortcut(Action::ToggleToolbarLayout, [this] { toggleToolbarLayout(); });
    addActionShortcut(Action::Cancel, [this] {
        if (m_mode == Mode::Selecting && m_startupTool != StartupTool::None) {
            leaveStartupTool();
            return;
        }
        emit sessionCancelRequested();
        close();
    });
}
