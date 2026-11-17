// Handles background model downloads using a single sequential queue.
//
// Downloads are saved to temporary `.part` files, verified with SHA-256,
// and renamed only after successful verification. Partial downloads resume automatically.
//
// Downloads stop immediately when the window closes (`Downloader::stop`)
// and do not auto-resume on the next run.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use super::models::{Asset, BASE_URL, on_disk};

const CHUNK: usize = 256 * 1024;
const STOPPED: &str = "stopped: the program is closing";

pub struct Job {
    pub id: u64,
    pub model: usize,
    pub dir: PathBuf,
    pub assets: [Asset; 3],
    pub cancel: Arc<AtomicBool>,
}

pub enum Update {
    Progress {
        id: u64,
        model: usize,
        percent: u8,
    },
    Done {
        id: u64,
        model: usize,
        result: Result<(), String>,
    },
}

impl Update {
    pub fn job(&self) -> (u64, usize) {
        match self {
            Update::Progress { id, model, .. } | Update::Done { id, model, .. } => (*id, *model),
        }
    }
}

pub struct Downloader {
    jobs: Option<Sender<Job>>,
    updates_tx: Sender<Update>,
    updates: Receiver<Update>,
    stop: Arc<AtomicBool>,
    pending: usize,
}

impl Default for Downloader {
    fn default() -> Self {
        let (updates_tx, updates) = channel();
        Self {
            jobs: None,
            updates_tx,
            updates,
            stop: Arc::new(AtomicBool::new(false)),
            pending: 0,
        }
    }
}

impl Downloader {
    /// 【识别模型】【任务排空】返回是否仍有未消费的真实任务完成消息，取消后的任务也需要排空，避免保留空闲工作通道
    pub fn has_pending(&self) -> bool {
        self.pending > 0
    }
    pub fn push(&mut self, job: Job) {
        if self.stop.load(Ordering::Relaxed) {
            return;
        }
        self.pending += 1;
        let job = match &self.jobs {
            Some(tx) => match tx.send(job) {
                Ok(()) => return,
                Err(err) => err.0,
            },
            None => job,
        };
        let (tx, rx) = channel();
        let updates = self.updates_tx.clone();
        let stop = self.stop.clone();
        std::thread::spawn(move || run(rx, updates, stop));
        let _ = tx.send(job);
        self.jobs = Some(tx);
    }

    /// 【识别模型】【下载完成】接收进度或结果；队列为空时关闭工作通道，释放空闲下载线程
    pub fn try_recv(&mut self) -> Option<Update> {
        let update = self.updates.try_recv().ok()?;
        if matches!(update, Update::Done { .. }) {
            self.pending = self.pending.saturating_sub(1);
            if self.pending == 0 {
                self.jobs = None;
            }
        }
        Some(update)
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.jobs = None;
    }
}

impl Drop for Downloader {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(jobs: Receiver<Job>, updates: Sender<Update>, stop: Arc<AtomicBool>) {
    let agent = agent();
    for job in jobs {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let result = fetch_model(&agent, &job, &updates, &stop);
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let done = Update::Done {
            id: job.id,
            model: job.model,
            result,
        };
        if updates.send(done).is_err() {
            return;
        }
    }
}

pub fn agent() -> ureq::Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .tls_config(tls)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(600)))
        .build();
    ureq::Agent::new_with_config(config)
}

fn fetch_model(
    agent: &ureq::Agent,
    job: &Job,
    updates: &Sender<Update>,
    stop: &AtomicBool,
) -> Result<(), String> {
    fs::create_dir_all(&job.dir).map_err(err)?;

    let missing: Vec<&Asset> = job
        .assets
        .iter()
        .filter(|a| !on_disk(&job.dir, a))
        .collect();
    let total: u64 = missing.iter().map(|a| a.size).sum();
    let report = |percent: u8| {
        let _ = updates.send(Update::Progress {
            id: job.id,
            model: job.model,
            percent,
        });
    };
    report(0);

    let mut finished = 0;
    let mut shown = 0;
    for asset in missing {
        let original = format!("{BASE_URL}/{}", asset.file);
        let urls = super::mirror::candidates(&original);
        fetch_candidates(agent, &urls, asset, &job.dir, &job.cancel, stop, |got| {
            let percent = ((finished + got) * 100 / total.max(1)) as u8;
            if percent != shown {
                shown = percent;
                report(percent);
            }
        })?;
        finished += asset.size;
    }
    Ok(())
}

/// 【识别模型】【多源下载】依次尝试 urls 候选地址执行下载，取消或退出立即终止，镜像失败自动回退后续源
pub fn fetch_candidates(
    agent: &ureq::Agent,
    urls: &[String],
    asset: &Asset,
    dir: &Path,
    cancel: &AtomicBool,
    stop: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    let mut last_error = String::from("no download URL configured");
    for (index, url) in urls.iter().enumerate() {
        match fetch(agent, url, asset, dir, cancel, stop, &mut progress) {
            Ok(()) => return Ok(()),
            Err(error) => {
                if stop.load(Ordering::Relaxed) || cancel.load(Ordering::Relaxed) {
                    return Err(error);
                }
                if index + 1 < urls.len() {
                    log::info!(
                        "【识别模型】【镜像回退】文件={}，失败原因={}，切换备用源",
                        asset.file,
                        error
                    );
                }
                last_error = error;
            }
        }
    }
    Err(last_error)
}

pub fn fetch(
    agent: &ureq::Agent,
    url: &str,
    asset: &Asset,
    dir: &Path,
    cancel: &AtomicBool,
    stop: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    let stopped = || stop.load(Ordering::Relaxed);
    if stopped() {
        return Err(STOPPED.into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let part_path = dir.join(format!("{}.part", asset.file));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&part_path)
        .map_err(err)?;
    let mut file = crate::utils::fs_lock::acquire(file, Duration::ZERO)
        .map_err(|_| format!("{} is being downloaded by another window", asset.file))?;

    let mut have = file.metadata().map_err(err)?.len();
    if have > asset.size {
        file.set_len(0).map_err(err)?;
        have = 0;
    }

    if have < asset.size {
        log::info!(
            "【识别模型】【开始下载】文件={}，总计={}字节，已缓存={}字节，目标目录={}，地址={url}",
            asset.file,
            asset.size,
            have,
            dir.display()
        );
        let mut request = agent.get(url);
        if have > 0 {
            request = request.header("Range", format!("bytes={have}-"));
        }
        let response = request.call().map_err(err)?;
        if stopped() {
            return Err(STOPPED.into());
        }
        let status = response.status().as_u16();
        log::info!(
            "【识别模型】【下载响应】文件={}，状态码=HTTP {status}，地址={url}",
            asset.file
        );
        match status {
            206 => {}
            // if server ignored Range
            200 => {
                file.set_len(0).map_err(err)?;
                have = 0;
            }
            code => return Err(format!("{} answered HTTP {code}", asset.file)),
        }
        file.seek(SeekFrom::Start(have)).map_err(err)?;

        let mut body = response
            .into_body()
            .into_with_config()
            .limit(asset.size - have)
            .reader();
        let mut buf = vec![0u8; CHUNK];
        while have < asset.size {
            if cancel.load(Ordering::Relaxed) {
                drop(file);
                let _ = fs::remove_file(&part_path);
                return Err("cancelled".into());
            }
            let n = body.read(&mut buf).map_err(err)?;
            if stopped() {
                return Err(STOPPED.into());
            }
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(err)?;
            have += n as u64;
            progress(have);
        }
    }

    if have != asset.size {
        return Err(format!("{} ended early", asset.file));
    }
    if !matches_sha256(&mut file, asset.sha256).map_err(err)? {
        drop(file);
        let _ = fs::remove_file(&part_path);
        return Err(format!("{} failed the checksum", asset.file));
    }
    if stopped() {
        return Err(STOPPED.into());
    }
    file.sync_all().map_err(err)?;
    drop(file);
    let destination = dir.join(asset.file);
    fs::rename(&part_path, &destination).map_err(err)?;
    log::info!(
        "【识别模型】【下载完成】文件={}，校验通过并写入={}",
        asset.file,
        destination.display()
    );
    Ok(())
}

/// 【识别模型】【校验和】流式读取 file 并比较 expected；返回是否一致，不把整个模型载入内存
pub(super) fn matches_sha256(file: &mut File, expected: &str) -> std::io::Result<bool> {
    file.seek(SeekFrom::Start(0))?;
    let mut hash = hmac_sha256::Hash::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    let hex: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(hex == expected)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
