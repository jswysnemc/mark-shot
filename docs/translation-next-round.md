# 翻译服务图形配置

设置窗口新增“翻译服务”页面。翻译仍由 OCR 结果窗口或截图翻译动作主动触发；打开设置不会发送原文，也不会自动查询模型。

## 默认服务

未配置其他服务时，`provider = "auto"` 使用哔哩哔哩 Index-Translate。现有已配置服务保持优先，已有命令模板继续优先于网络服务。图形界面选择服务或保存厂商字段后会写入明确的服务编号；切换到网络服务时清除命令覆盖。

- 地址：`https://index-translate.bilibili.com/v1`
- 默认模型：`Index-Translate-9B`
- 密钥留空时使用占位值 `empty`
- 默认请求 `chat_template_kwargs.enable_thinking = false`
- 使用原生 HTTP 客户端和 `Mark-Shot/<版本>` User-Agent，不附加浏览器 Origin

2026-11-08 通过 `/v1/models` 核实模型编号，并成功完成两段中文到日语的真实请求，保留了原文编号 0、2。帖子早期示例中的 `index-mt-9b` 与当前目录编号不同，因此默认使用实际验证的 `Index-Translate-9B`。目录变化后可以重新查询或手动填写。

## 图形界面

在设置的“翻译服务”页面选择协议，点击字段修改，按 Enter 或“保存”提交，按 Esc 放弃当前编辑。字段分别保存，不需要重启应用。窗口缩小时使用滚动条、滚轮或 PageUp/PageDown；模型菜单支持方向键、Home、End 和 Enter。

目标语言使用可搜索下拉框，中文界面显示中文名称及英文名称；选择后保存稳定语言值。已有自定义目标会作为当前选项保留。服务、模型和通用设置选项共用字段旁的浮层，长列表支持搜索与滚动；Enter 确认，Esc、点击外部或窗口失焦取消，不会改写配置。

| 服务 | 可配置项目 | 模型目录 |
| --- | --- | --- |
| 哔哩哔哩 | 地址、密钥、模型、超时、思考选项、自定义正文 | OpenAI 格式 `/models` |
| OpenAI Chat Completions | 同上 | `/models` |
| OpenAI Responses | 同上 | `/models` |
| Anthropic Messages | 同上 | `/models`，支持 `after_id` 分页 |
| Gemini | 同上 | `/models`，支持 `pageToken` 分页，过滤不支持 `generateContent` 的条目 |
| 腾讯机器翻译 | 接入点、Secret ID、Secret Key、区域、项目编号、超时 | 不适用 |
| 百度翻译 | 地址、应用编号、密钥、超时 | 不适用 |
| 有道翻译 | 地址、应用编号、密钥、超时 | 不适用 |
| 自定义命令 / 外部程序 | 命令模板或原有 helper 模式 | 不适用 |

“查询模型”使用该服务当前保存的地址及凭据；查询期间按钮变为“取消查询”。查询失败不会覆盖模型编号，手动输入始终可用。切换服务、修改地址或密钥、离开页面都会取消旧查询。每次查询有总超时、20 页和 5000 个模型上限；响应体上限 4 MiB，禁止自动重定向。

密钥默认遮蔽，编辑时可点“显示”。环境变量中的密钥不复制进配置页面。配置文件复用现有权限检查和原子写入；遮蔽只是显示行为，文件内密钥仍是明文。保存单字段会在锁内合并到最新配置，保留其他服务、未知字段和未修改的 TOML 注释。

## 思考与自定义正文

“过滤译文中的思考标签”与“请求关闭思考”独立控制。前者默认开启，移除文本中的 `<think>` / `<thinking>` 块；未闭合标签之后的内容也不当作译文。协议中独立的 reasoning / thinking 块始终不参与译文解析。Gemini 拼接首个候选的全部非思考文本块；Responses 只读取 message 中的 output_text，未完成生成不作为成功译文。

| 协议 | 请求关闭思考时发送 |
| --- | --- |
| 哔哩哔哩 | `chat_template_kwargs.enable_thinking: false` |
| OpenAI Chat Completions | `reasoning_effort: "none"` |
| OpenAI Responses | `reasoning.effort: "none"` |
| Anthropic | `thinking.type: "disabled"` |
| Gemini | `generationConfig.thinkingConfig.thinkingBudget: 0` |

这些选项需要具体模型支持。部分 Gemini 模型不能关闭思考，部分 OpenAI 兼容服务使用自己的字段；界面明确标注模型支持要求，不自动把“最低思考”当作“已关闭”。遇到参数不兼容，可以关闭该开关并使用自定义正文设置该服务支持的参数。

自定义正文接受 JSON 对象，支持粘贴多行，保存前验证语法。为兼容 TOML 的类型限制，持久化为 JSON 字符串，原有对象格式仍可读取。合并顺序为标准正文、思考开关、自定义正文；对象递归合并，数组和 null 整体覆盖。模型、待翻译内容及非流式响应模式最后固定，扩展不能替换这些必需字段。示例：

```json
{
  "temperature": 0.1,
  "chat_template_kwargs": { "enable_thinking": false }
}
```

## 模块与验证

```text
src/translation/
  resolve.rs                   默认服务、独立配置与凭据脱敏
  request_options.rs           扩展正文、思考开关与文本过滤
  models.rs                    模型目录、分页与可取消任务
  providers/llm.rs              五种模型服务请求适配
  providers/model_response.rs  Chat、Responses、Gemini、Anthropic 响应解析
src/config/translation/preferences.rs  字段原子保存与旧字段别名清理
src/features/preferences/translation/
  fields.rs                    服务目录与字段定义
  state.rs                     表单值、保存与查询状态
  layout.rs                    自适应布局与命中区域
  draw.rs                      原生控件绘制及密钥遮蔽
  input.rs                     鼠标、键盘与异步结果处理
```

验证覆盖请求协议、目录分页及 URL 编码、实际 HTTP GET 与应用请求头、取消后关闭连接、正文过滤、配置隔离、非法 JSON 拒绝写入，以及两种新增服务的真实回环 HTTP 翻译任务。公开接口集成测试使用固定测试句，默认忽略，显式执行：

```sh
cargo test --test translation_bilibili --locked -- --ignored --nocapture
```

其余模型服务使用本地协议测试，未使用用户付费密钥。Windows 通过 MSVC 交叉编译静态检查，原生 GUI 实际交互在隔离 Wayland 会话验证。

## 接口依据

- 用户提供的帖子正文：<https://linux.do/t/topic/2977401>
- 实际模型目录：<https://index-translate.bilibili.com/v1/models>
- OpenAI Responses：<https://developers.openai.com/api/reference/resources/responses/methods/create>
- Anthropic 模型目录：<https://platform.claude.com/docs/en/api/models/list>
- Gemini 模型目录与思考选项：<https://ai.google.dev/api/models>、<https://ai.google.dev/gemini-api/docs/thinking>
