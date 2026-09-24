# 翻译服务提供方

Mark Shot 的贴图窗口 OCR 结果可以直接翻译。翻译能力由 provider 插件提供，除原有的 OpenAI 兼容接口外，现在还支持腾讯云机器翻译、百度翻译开放平台与网易有道智云三家传统机器翻译接口。

## 可用的 provider

| providerId | 展示名 | 接口类型 | 需要的凭据 |
| :--- | :--- | :--- | :--- |
| `openai-compatible` | OpenAI Compatible | 大模型 chat/completions | apiBase、apiKey、model |
| `gemini` | Google Gemini | Google Gemini API (generateContent) | apiKey、model |
| `anthropic` | Anthropic Claude | Anthropic Messages API | apiKey、model |
| `tencent-tmt` | Tencent Machine Translation | 腾讯云 API 3.0（TC3-HMAC-SHA256） | SecretId、SecretKey |
| `baidu-fanyi` | Baidu Translate | 百度翻译通用文本翻译（MD5 签名） | APPID、密钥 |
| `youdao-nmt` | Youdao Translate | 有道智云文本翻译（v3 签名） | 应用 ID、应用密钥 |

各插件均以独立动态库形式安装到 `<libdir>/mark-shot/plugins/`，缺少凭据的插件在设置界面显示为不可用，不会影响其他 provider。

## 选择 provider

设置窗口的「插件」页有 `Translation Provider` 下拉框，也可以直接写配置：

```json
{
  "translation": {
    "provider": "plugin:gemini"
  }
}
```

取值形式：

- `auto`（默认）：按 `openai-compatible` → `gemini` → `anthropic` → `tencent-tmt` → `baidu-fanyi` → `youdao-nmt` 的固定顺序，选中第一个凭据完整的插件。这个顺序是写死的，不随插件加载顺序变化。
- `plugin:<providerId>`：显式指定某个插件；该插件不可用时回退到 helper 脚本。
- `builtin`：使用主程序内置的 OpenAI 兼容实现。
- `helper`：使用 `mark-shot-translate` Python 脚本。

`translation.command` 非空时优先级最高，会绕过全部插件直接执行该命令。

## 凭据配置

云翻译与大模型服务的凭据写在 `translation` 下的厂商子节，也可以在设置窗口的「集成」页填写。字段留空时插件会去读环境变量，凭据可以不落盘。

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

| 字段 | 环境变量（按顺序） | 默认值 |
| :--- | :--- | :--- |
| `apiKey` | `GEMINI_API_KEY`、`MARK_SHOT_GEMINI_API_KEY` | 无，必填 |
| `model` | `GEMINI_MODEL`、`MARK_SHOT_GEMINI_MODEL` | `gemini-3.5-flash-lite` |
| `endpoint` | `GEMINI_API_BASE`、`MARK_SHOT_GEMINI_API_BASE` | `https://generativelanguage.googleapis.com/v1beta` |
| `systemPrompt` | 无 | 内置翻译提示词 |
| `temperature` | 无 | 不下发，使用模型默认值 |
| `thinkingLevel` | 无 | 不下发，使用模型默认值 |
| `timeoutMs` | 无 | `60000` |

插件直连 Google Gemini 原生 REST API（`:generateContent`），鉴权头为 `x-goog-api-key`，启用 `responseMimeType: "application/json"`。`thinkingLevel` 可取 `minimal`、`low`、`medium`、`high`，各模型支持的档位不同，因此只在显式配置时下发。

### Anthropic Claude

| 字段 | 环境变量（按顺序） | 默认值 |
| :--- | :--- | :--- |
| `apiKey` | `ANTHROPIC_API_KEY`、`MARK_SHOT_ANTHROPIC_API_KEY` | 无，必填 |
| `model` | `ANTHROPIC_MODEL`、`MARK_SHOT_ANTHROPIC_MODEL` | `claude-haiku-4-5` |
| `endpoint` | `ANTHROPIC_API_BASE`、`MARK_SHOT_ANTHROPIC_API_BASE` | `https://api.anthropic.com/v1` |
| `systemPrompt` | 无 | 内置翻译提示词 |
| `maxTokens` | 无 | `4096` |
| `temperature` | 无 | 不下发，使用模型默认值 |
| `timeoutMs` | 无 | `60000` |

插件直连 Anthropic Messages API（`/v1/messages`），鉴权头为 `x-api-key` 与 `anthropic-version: 2023-06-01`。Claude Opus 4.7 及更新的模型不接受 `temperature`，因此该字段只在显式配置时下发。

两个插件只读取各自的 `translation.gemini` / `translation.anthropic` 子节。公共节中的 `translation.apiKey`、`translation.model`、`translation.systemPrompt` 属于 OpenAI 兼容服务，不会被继承。

### 腾讯云机器翻译

| 字段 | 环境变量（按顺序） | 默认值 |
| :--- | :--- | :--- |
| `secretId` | `TENCENTCLOUD_SECRET_ID`、`MARK_SHOT_TENCENT_SECRET_ID` | 无，必填 |
| `secretKey` | `TENCENTCLOUD_SECRET_KEY`、`MARK_SHOT_TENCENT_SECRET_KEY` | 无，必填 |
| `region` | `TENCENTCLOUD_REGION`、`MARK_SHOT_TENCENT_REGION` | `ap-guangzhou` |
| `endpoint` | 无 | `tmt.tencentcloudapi.com` |
| `projectId` | 无 | `0` |
| `timeoutMs` | 无 | `30000` |

插件默认调用 `TextTranslateBatch` 一次翻译多段，按 2000 字符分批。免费额度为每月 500 万字符。

需要注意：腾讯云已于 2026-07-08 把 `TextTranslate` 从官方文档中删除，`TextTranslateBatch` 更早在 2026-03-12 删除，现行 API 概览只保留 `ImageTranslateLLM`。服务端路由目前仍然存活，计费文档也仍将文本翻译列为在售子产品，但该接口已不再公开维护。插件在收到 `ActionOffline` 或 `InvalidAction` 时会自动降级为逐段调用 `TextTranslate`；若两个接口都下线，请改用其他 provider。

### 百度翻译

| 字段 | 环境变量（按顺序） | 默认值 |
| :--- | :--- | :--- |
| `appId` | `MARK_SHOT_BAIDU_APP_ID`、`BAIDU_TRANSLATE_APP_ID` | 无，必填 |
| `appKey` | `MARK_SHOT_BAIDU_APP_KEY`、`BAIDU_TRANSLATE_APP_KEY` | 无，必填 |
| `endpoint` | 无 | `https://fanyi-api.baidu.com/api/trans/vip/translate` |
| `timeoutMs` | 无 | `30000` |

多段文本以换行连接后一次提交，按 1800 字符分批。免费额度按认证等级区分：未认证 5 万字符/月且单次上限 1000 字符，个人认证 100 万字符/月，企业认证 200 万字符/月。未认证账号的 QPS 限制为 1，分段较多时可能触发 `54003`。

### 网易有道

| 字段 | 环境变量（按顺序） | 默认值 |
| :--- | :--- | :--- |
| `appKey` | `MARK_SHOT_YOUDAO_APP_KEY`、`YOUDAO_APP_KEY` | 无，必填 |
| `appSecret` | `MARK_SHOT_YOUDAO_APP_SECRET`、`YOUDAO_APP_SECRET` | 无，必填 |
| `endpoint` | 无 | `https://openapi.youdao.com/api` |
| `timeoutMs` | 无 | `30000` |

多段文本用多个同名 `q` 表单字段提交，按 4000 字符分批（单次查询上限 5000 字符）。新用户注册赠送体验金，之后按量计费。

## 目标语言

`translation.targetLanguage` 接受英文名、中文名或 BCP-47 标记，插件内部统一归一化后再映射到各家的语言代码。例如 `Simplified Chinese`、`简体中文`、`zh-CN`、`zh` 都会被识别为简体中文。

| 语言 | 腾讯 | 百度 | 有道 |
| :--- | :---: | :---: | :---: |
| 简体中文 | `zh` | `zh` | `zh-CHS` |
| 繁体中文 | `zh-TW` | `cht` | `zh-CHT` |
| 英语 | `en` | `en` | `en` |
| 日语 | `ja` | `jp` | `ja` |
| 韩语 | `ko` | `kor` | `ko` |
| 法语 | `fr` | `fra` | `fr` |
| 德语 | `de` | `de` | `de` |
| 俄语 | `ru` | `ru` | `ru` |
| 西班牙语 | `es` | `spa` | `es` |
| 葡萄牙语 | `pt` | `pt` | `pt` |
| 意大利语 | `it` | `it` | `it` |
| 阿拉伯语 | `ar` | `ara` | `ar` |
| 泰语 | `th` | `th` | `th` |
| 越南语 | `vi` | `vie` | `vi` |
| 荷兰语 | 不支持 | `nl` | `nl` |
| 粤语 | 不支持 | `yue` | `yue` |

目标语言超出某家支持范围时，该插件返回明确的错误信息而不是静默回退。腾讯的语言方向不是全连通（例如日语只能与中文、英语、韩语互译），不支持的方向由服务端返回 `UnsupportedOperation.UnSupportedTargetLanguage`。

## 排查

翻译失败时的错误信息会带上厂商原始错误码。常见情况：

- 百度 `54001`：签名错误。检查 APPID 与密钥是否配对，注意密钥不要带首尾空格。
- 百度 `58001`：译文语言方向不支持，通常出现在未认证账号请求常见 28 种以外的语言。
- 有道 `202`：签名校验失败。若确认 appKey 与 appSecret 正确，通常是文本编码问题。
- 有道 `206`：时间戳无效，检查系统时钟。
- 腾讯 `AuthFailure.SignatureExpire`：请求时间戳与服务器相差超过 5 分钟，同样是时钟问题。
- 腾讯 `FailedOperation.NoFreeAmount`：当月免费额度用尽且未开通后付费。

开启调试日志后，插件加载情况会记录在日志中：

```bash
DEBUG=1 mark-shot
```
