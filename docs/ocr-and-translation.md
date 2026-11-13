# OCR and translation

**English** | [简体中文](zh-CN/ocr-and-translation.md)

## Text recognition

Mark Shot recognizes text locally with PP-OCRv5 models running on ONNX Runtime. No image leaves
your machine, and no Python or external OCR program is involved. Ordinary screenshots never load
the models.

### First use

OCR needs two resources. Neither is downloaded automatically; the OCR panel shows a download
button, the size and the progress for each.

| Resource | Content |
| --- | --- |
| Inference library | ONNX Runtime 1.28.0, verified by SHA-256 |
| Language model | Latin, Cyrillic, Arabic, Greek, Chinese and Japanese, Korean, or Thai |

Downloads can be cancelled and resumed. Once installed, OCR works offline. A build with the
`bundled-ocr` feature embeds the inference library, see [Installation](installation.md).

### Starting recognition

| From | How |
| --- | --- |
| Command line | `mark-shot --ocr`, then select a region |
| Editor | Press `O`, or use the OCR button on the action toolbar |
| Pinned image | Right-click menu: **Copy Image Text** or **Text Selection Copy** |

### Result window

Recognition opens a separate result window.

- **Text page.** The recognized text is editable, with undo and redo. `Ctrl+C` copies the
  selection and `Ctrl+Shift+C` copies everything.
- **Image page.** The original region; scroll to zoom, drag to pan, double-click to reset.
- **Table page.** Shown when the text forms rows and columns. Cells are editable and the table can
  be copied as TSV, CSV, Markdown or HTML.
- **Translation.** Translates the text into the target language next to the original.

The capture process exits once the result window is ready, so the models do not stay in memory.

To select text directly on the screenshot instead of opening the window:

```toml
[ocr]
result_panel = false
```

### Engine options

```toml
[ocr]
threads = 4
mode = "on-demand"        # on-demand | at-launch | daemon
device = "auto"           # auto | cpu | gpu; used by the daemon
daemon_idle_secs = 900    # 0 keeps the daemon running
```

| Mode | Behaviour |
| --- | --- |
| `on-demand` | The engine is built when OCR is first used in a capture. Default |
| `at-launch` | The engine is built at every launch |
| `daemon` | A background process keeps the engine ready and exits after the idle time |

On Windows, the first OCR asks which engine to use: **Windows OCR**, which uses the language packs
installed in the system and downloads nothing, or **local model OCR**, which works as described
above. The choice is stored in `ocr.engine` (`ask`, `system` or `paddle`) and can be changed on the
OCR settings page.

## Translation

Translation is available in three places: the translate button in the editor, which translates the
text inside the selection; the OCR result window; and the menu of a pinned image.

Text is sent to a service only when you start a translation. Opening the settings window sends
nothing.

### Services

| Service | Needs |
| --- | --- |
| Bilibili Index-Translate | Nothing; this is the default when no other service is configured |
| OpenAI Chat Completions, or any compatible endpoint | Address, API key, model |
| OpenAI Responses | Address, API key, model |
| Anthropic Messages | API key, model |
| Gemini | API key, model |
| Tencent Machine Translation | Secret ID, secret key, region |
| Baidu Translate | App ID, key |
| Youdao Translate | App key, app secret |
| Custom command | A command that performs the translation |

### Setting up a service

1. Open **Settings → Translation**.
2. Choose the target language and the service.
3. Fill in the address and credentials. Each field is saved separately.
4. For model-based services, click **Query models** to list the models of the endpoint, or type a
   model name.

Keys are masked on screen but stored as plain text in the configuration file. To keep a key out of
the file, set it in the environment instead:

| Service | Variables |
| --- | --- |
| OpenAI-compatible | `MARK_SHOT_LLM_API_BASE`, `MARK_SHOT_LLM_API_KEY`, `MARK_SHOT_LLM_MODEL` |
| Gemini | `MARK_SHOT_GEMINI_API_BASE`, `MARK_SHOT_GEMINI_API_KEY`, `MARK_SHOT_GEMINI_MODEL` |
| Anthropic | `MARK_SHOT_ANTHROPIC_API_KEY`, `MARK_SHOT_ANTHROPIC_MODEL` |
| Tencent | `MARK_SHOT_TENCENT_SECRET_ID`, `MARK_SHOT_TENCENT_SECRET_KEY`, `MARK_SHOT_TENCENT_REGION` |
| Baidu | `MARK_SHOT_BAIDU_APP_ID`, `MARK_SHOT_BAIDU_APP_KEY` |
| Youdao | `MARK_SHOT_YOUDAO_APP_KEY`, `MARK_SHOT_YOUDAO_APP_SECRET` |

The file stores the general choice in `[translation]`:

```toml
[translation]
target_language = "Simplified Chinese"
provider = "auto"
command = ""
```

### Model options

Two switches apply to model-based services:

- **Request thinking off** asks the model not to reason before answering. Not every model
  supports this; turn the switch off if the service rejects the request.
- **Filter thinking tags** removes `<think>` blocks from the answer. It is on by default.

**Custom body** accepts a JSON object that is merged into the request, for parameters such as
`temperature`. The model, the text to translate and the response mode cannot be replaced this way.

```json
{ "temperature": 0.1 }
```
