# Translation providers

Mark Shot can translate OCR results directly in the pinned window. Translation is supplied by provider plugins. Besides the original OpenAI-compatible endpoint, Tencent Machine Translation, Baidu Translate, and Youdao are now supported.

## Available providers

| providerId | Display name | API style | Required credentials |
| :--- | :--- | :--- | :--- |
| `openai-compatible` | OpenAI Compatible | LLM chat/completions | apiBase, apiKey, model |
| `gemini` | Google Gemini | Google Gemini API (generateContent) | apiKey, model |
| `anthropic` | Anthropic Claude | Anthropic Messages API | apiKey, model |
| `tencent-tmt` | Tencent Machine Translation | Tencent Cloud API 3.0 (TC3-HMAC-SHA256) | SecretId, SecretKey |
| `baidu-fanyi` | Baidu Translate | Baidu general text translation (MD5 signature) | APPID, secret key |
| `youdao-nmt` | Youdao Translate | Youdao text translation (v3 signature) | app key, app secret |

All provider plugins ship as separate shared libraries under `<libdir>/mark-shot/plugins/`. A plugin without credentials reports itself as unavailable in the settings window and does not affect the others.

## Selecting a provider

The Plugins page of the settings window has a `Translation Provider` combo box. The same choice can be written directly to the config file:

```json
{
  "translation": {
    "provider": "plugin:gemini"
  }
}
```

Accepted values:

- `auto` (default): picks the first plugin with complete credentials, in the fixed order `openai-compatible` → `gemini` → `anthropic` → `tencent-tmt` → `baidu-fanyi` → `youdao-nmt`. This order is hard-coded and does not depend on plugin load order.
- `plugin:<providerId>`: selects one plugin explicitly, falling back to the helper script if that plugin is unavailable.
- `builtin`: uses the OpenAI-compatible implementation built into the main binary.
- `helper`: uses the `mark-shot-translate` Python script.

A non-empty `translation.command` takes precedence over everything else and bypasses the plugins entirely.

## Credentials

Credentials for the three cloud services live in per-vendor sub-objects under `translation`, and can also be entered on the Integrations page of the settings window. When a field is left empty the plugin reads the corresponding environment variable instead, so credentials need not be written to disk.

```json
{
  "translation": {
    "provider": "auto",
    "targetLanguage": "Simplified Chinese",
    "tencent": {
      "secretId": "",
      "secretKey": "",
      "region": "ap-guangzhou",
      "projectId": 0,
      "timeoutMs": 30000
    },
    "baidu": {
      "appId": "",
      "appKey": "",
      "timeoutMs": 30000
    },
    "youdao": {
      "appKey": "",
      "appSecret": "",
      "timeoutMs": 30000
    },
    "gemini": {
      "apiKey": "",
      "model": "gemini-3.5-flash-lite",
      "endpoint": "https://generativelanguage.googleapis.com/v1beta",
      "timeoutMs": 60000
    },
    "anthropic": {
      "apiKey": "",
      "model": "claude-haiku-4-5",
      "endpoint": "https://api.anthropic.com/v1",
      "timeoutMs": 60000
    }
  }
}
```

### Google Gemini

| Field | Environment variables (in order) | Default |
| :--- | :--- | :--- |
| `apiKey` | `GEMINI_API_KEY`, `MARK_SHOT_GEMINI_API_KEY` | none, required |
| `model` | `GEMINI_MODEL`, `MARK_SHOT_GEMINI_MODEL` | `gemini-3.5-flash-lite` |
| `endpoint` | `GEMINI_API_BASE`, `MARK_SHOT_GEMINI_API_BASE` | `https://generativelanguage.googleapis.com/v1beta` |
| `systemPrompt` | none | built-in translator prompt |
| `temperature` | none | not sent, model default |
| `thinkingLevel` | none | not sent, model default |
| `timeoutMs` | none | `60000` |

The plugin connects to the native Google Gemini REST API (`:generateContent`), authenticates via `x-goog-api-key`, and requests `responseMimeType: "application/json"`. `thinkingLevel` accepts `minimal`, `low`, `medium` or `high`; supported levels differ per model, so it is only sent when set explicitly.

### Anthropic Claude

| Field | Environment variables (in order) | Default |
| :--- | :--- | :--- |
| `apiKey` | `ANTHROPIC_API_KEY`, `MARK_SHOT_ANTHROPIC_API_KEY` | none, required |
| `model` | `ANTHROPIC_MODEL`, `MARK_SHOT_ANTHROPIC_MODEL` | `claude-haiku-4-5` |
| `endpoint` | `ANTHROPIC_API_BASE`, `MARK_SHOT_ANTHROPIC_API_BASE` | `https://api.anthropic.com/v1` |
| `systemPrompt` | none | built-in translator prompt |
| `maxTokens` | none | `4096` |
| `temperature` | none | not sent, model default |
| `timeoutMs` | none | `60000` |

The plugin connects to the Anthropic Messages API (`/v1/messages`), passing `x-api-key` and `anthropic-version: 2023-06-01`. `temperature` is only sent when set explicitly, because Claude Opus 4.7 and newer reject it.

Both plugins read only their own `translation.gemini` / `translation.anthropic` sub-object. The shared `translation.apiKey`, `translation.model` and `translation.systemPrompt` keys belong to the OpenAI-compatible provider and are not inherited.

### Tencent Machine Translation

| Field | Environment variables (in order) | Default |
| :--- | :--- | :--- |
| `secretId` | `TENCENTCLOUD_SECRET_ID`, `MARK_SHOT_TENCENT_SECRET_ID` | none, required |
| `secretKey` | `TENCENTCLOUD_SECRET_KEY`, `MARK_SHOT_TENCENT_SECRET_KEY` | none, required |
| `region` | `TENCENTCLOUD_REGION`, `MARK_SHOT_TENCENT_REGION` | `ap-guangzhou` |
| `endpoint` | none | `tmt.tencentcloudapi.com` |
| `projectId` | none | `0` |
| `timeoutMs` | none | `30000` |

The plugin calls `TextTranslateBatch` to translate several segments per request and splits input into 2000-character batches. The free tier is 5 million characters per month.

One caveat: Tencent removed `TextTranslate` from its public API documentation on 2026-07-08, and `TextTranslateBatch` earlier on 2026-03-12; the current API overview lists only `ImageTranslateLLM`. The server-side routes are still live and the pricing page still lists text translation as an active sub-product, but the endpoint is no longer publicly maintained. On `ActionOffline` or `InvalidAction` the plugin automatically falls back to per-segment `TextTranslate` calls. If both endpoints are retired, switch to another provider.

### Baidu Translate

| Field | Environment variables (in order) | Default |
| :--- | :--- | :--- |
| `appId` | `MARK_SHOT_BAIDU_APP_ID`, `BAIDU_TRANSLATE_APP_ID` | none, required |
| `appKey` | `MARK_SHOT_BAIDU_APP_KEY`, `BAIDU_TRANSLATE_APP_KEY` | none, required |
| `endpoint` | none | `https://fanyi-api.baidu.com/api/trans/vip/translate` |
| `timeoutMs` | none | `30000` |

Segments are joined with newlines into a single request and split into 1800-character batches. Free quota depends on account verification: 50k characters per month and a 1000-character request limit without verification, 1M with personal verification, 2M with business verification. Unverified accounts are limited to 1 QPS, which can trigger `54003` when there are many segments.

### Youdao

| Field | Environment variables (in order) | Default |
| :--- | :--- | :--- |
| `appKey` | `MARK_SHOT_YOUDAO_APP_KEY`, `YOUDAO_APP_KEY` | none, required |
| `appSecret` | `MARK_SHOT_YOUDAO_APP_SECRET`, `YOUDAO_APP_SECRET` | none, required |
| `endpoint` | none | `https://openapi.youdao.com/api` |
| `timeoutMs` | none | `30000` |

Segments are sent as repeated `q` form fields and split into 4000-character batches (the per-request limit is 5000 characters). New accounts receive trial credit, after which usage is billed per character.

## Target language

`translation.targetLanguage` accepts an English name, a Chinese name, or a BCP-47 tag. The value is normalized once and then mapped to each vendor's own code, so `Simplified Chinese`, `简体中文`, `zh-CN`, and `zh` all resolve to Simplified Chinese.

| Language | Tencent | Baidu | Youdao |
| :--- | :---: | :---: | :---: |
| Simplified Chinese | `zh` | `zh` | `zh-CHS` |
| Traditional Chinese | `zh-TW` | `cht` | `zh-CHT` |
| English | `en` | `en` | `en` |
| Japanese | `ja` | `jp` | `ja` |
| Korean | `ko` | `kor` | `ko` |
| French | `fr` | `fra` | `fr` |
| German | `de` | `de` | `de` |
| Russian | `ru` | `ru` | `ru` |
| Spanish | `es` | `spa` | `es` |
| Portuguese | `pt` | `pt` | `pt` |
| Italian | `it` | `it` | `it` |
| Arabic | `ar` | `ara` | `ar` |
| Thai | `th` | `th` | `th` |
| Vietnamese | `vi` | `vie` | `vi` |
| Dutch | unsupported | `nl` | `nl` |
| Cantonese | unsupported | `yue` | `yue` |

When a target language is outside a vendor's range the plugin returns an explicit error instead of silently falling back. Tencent's language pairs are not fully connected (Japanese, for example, only pairs with Chinese, English, and Korean); unsupported directions are rejected server-side with `UnsupportedOperation.UnSupportedTargetLanguage`.

## Troubleshooting

Failure messages carry the vendor's original error code. Common cases:

- Baidu `54001`: signature error. Verify that the APPID and secret key belong to the same application and carry no leading or trailing whitespace.
- Baidu `58001`: unsupported language direction, usually an unverified account requesting a language outside the common set of 28.
- Youdao `202`: signature verification failed. If the app key and secret are correct, this is normally a text encoding problem.
- Youdao `206`: invalid timestamp; check the system clock.
- Tencent `AuthFailure.SignatureExpire`: the request timestamp differs from the server by more than five minutes, again a clock problem.
- Tencent `FailedOperation.NoFreeAmount`: the monthly free quota is exhausted and pay-as-you-go is not enabled.

With debug logging enabled the plugin load results are written to the log:

```bash
DEBUG=1 mark-shot
```
