//! 【翻译服务】【模型目录】按需查询各协议目录，限制分页并支持窗口取消

use super::{
    Error, ErrorKind, Provider, Result,
    http::{Http, Request},
    providers::llm,
    resolve::{self, Llm, Service},
};
use serde_json::Value;
use std::{collections::BTreeSet, sync::mpsc, thread::JoinHandle};
use tokio::sync::oneshot;

pub(crate) struct Task {
    result: mpsc::Receiver<Result<Vec<String>>>,
    cancel: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Task {
    /// 【模型目录】【启动查询】按 root 和 provider 解析配置，wake 接收一次完成通知；返回可取消任务
    pub(crate) fn start(
        root: &Value,
        provider: &str,
        wake: impl FnOnce() + Send + 'static,
    ) -> Result<Self> {
        let service = resolve::service(root, provider, &|name| std::env::var(name).ok())?;
        let Service::Llm(config) = &service else {
            return Err(Error::new(
                ErrorKind::Configuration,
                "此服务没有模型目录 / This service has no model catalog",
            ));
        };
        let config = config.clone();
        let (sender, result) = mpsc::channel();
        let (cancel, canceled) = oneshot::channel();
        let worker = std::thread::Builder::new().name("translation-models".into()).spawn(move || {
            let mut result = tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|_| Error::new(ErrorKind::StartFailed, "cannot start model query"))
                .and_then(|runtime| runtime.block_on(async {
                    tokio::select! {
                        _ = canceled => Err(Error::new(ErrorKind::Canceled, "model query canceled")),
                        result = tokio::time::timeout(config.timeout, list(&config)) => result.unwrap_or_else(|_| Err(Error::new(ErrorKind::Timeout, "model query timed out"))),
                    }
                }));
            if let Err(error) = &mut result { service.redact(error); }
            let _ = sender.send(result);
            wake();
        }).map_err(|_| Error::new(ErrorKind::StartFailed, "cannot start model query"))?;
        Ok(Self {
            result,
            cancel: Some(cancel),
            worker: Some(worker),
        })
    }

    /// 【模型目录】【接收结果】无参数，返回已经完成的查询结果；未完成不阻塞窗口
    pub(crate) fn finished(&self) -> Option<Result<Vec<String>>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::new(
                ErrorKind::StartFailed,
                "model query stopped",
            ))),
        }
    }
}

impl Drop for Task {
    /// 【模型目录】【取消回收】无参数，取消网络等待并回收已退出线程，返回空值且不阻塞事件循环
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            let _ = self.worker.take().unwrap().join();
        }
    }
}

/// 【模型目录】【目录地址】按 config 去除生成路径，返回同一服务目录地址；不接受 URL 内嵌凭据
fn endpoint(config: &Llm) -> Result<url::Url> {
    let generation = llm::endpoint(config);
    let suffix = match config.provider {
        Provider::Responses => "/responses",
        Provider::Anthropic => "/messages",
        Provider::Gemini => "/models/",
        _ => "/chat/completions",
    };
    let base = generation
        .rsplit_once(suffix)
        .map(|(base, _)| base)
        .ok_or_else(|| Error::new(ErrorKind::Configuration, "invalid models endpoint"))?;
    let url = url::Url::parse(&format!("{base}/models"))
        .map_err(|_| Error::new(ErrorKind::Configuration, "invalid models endpoint"))?;
    Ok(url)
}

/// 【模型目录】【分页解析】按 provider 和 body 返回模型及下一页标记，过滤 Gemini 非文本生成模型
fn page(provider: Provider, body: &Value) -> Result<(Vec<String>, Option<String>)> {
    let key = if provider == Provider::Gemini {
        "models"
    } else {
        "data"
    };
    let entries = body
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| Error::new(ErrorKind::Protocol, "model response missing list"))?;
    let models = entries
        .iter()
        .filter(|entry| {
            provider != Provider::Gemini
                || entry["supportedGenerationMethods"]
                    .as_array()
                    .is_none_or(|methods| methods.iter().any(|method| method == "generateContent"))
        })
        .filter_map(|entry| {
            entry[if provider == Provider::Gemini {
                "name"
            } else {
                "id"
            }]
            .as_str()
        })
        .map(|name| name.trim_start_matches("models/").to_owned())
        .filter(|name| !name.is_empty() && name.len() <= 512 && !name.chars().any(char::is_control))
        .collect();
    let next = match provider {
        Provider::Gemini => body["nextPageToken"].as_str(),
        Provider::Anthropic if body["has_more"] == true => Some(
            body["last_id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| Error::new(ErrorKind::Protocol, "model page missing cursor"))?,
        ),
        _ => None,
    }
    .filter(|next| !next.is_empty())
    .map(str::to_owned);
    Ok((models, next))
}

/// 【模型目录】【完整查询】按 config 获取最多二十页模型，返回排序去重列表；重复游标和超限明确报错
async fn list(config: &Llm) -> Result<Vec<String>> {
    let http = Http::new()?;
    let base = endpoint(config)?;
    let mut models = BTreeSet::new();
    let mut cursors = BTreeSet::new();
    let mut cursor: Option<String> = None;
    for _ in 0..20 {
        let mut url = base.clone();
        if let Some(cursor) = &cursor {
            url.query_pairs_mut().append_pair(
                if config.provider == Provider::Gemini {
                    "pageToken"
                } else {
                    "after_id"
                },
                cursor,
            );
        }
        let body = http
            .get(Request {
                provider: config.provider,
                endpoint: url.to_string(),
                content_type: "application/json",
                headers: llm::headers(config),
                body: Vec::new(),
                timeout: config.timeout,
            })
            .await?;
        let (entries, next) = page(config.provider, &body)?;
        models.extend(entries);
        if models.len() > 5000 {
            break;
        }
        let Some(next) = next else {
            return Ok(models.into_iter().collect());
        };
        if !cursors.insert(next.clone()) {
            break;
        }
        cursor = Some(next);
    }
    Err(Error::new(
        ErrorKind::Protocol,
        "model directory exceeded pagination limit",
    ))
}

#[cfg(test)]
mod tests;
