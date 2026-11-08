// Manages OCR models: downloadable assets, local disk cache, and active selection.
//
// Model files are stored in `~/.local/share/mark-shot/models`, and active choice
// in `~/.local/state/mark-shot/ocr-model`.
//
// Uses a shared text detector across all languages, while each language has its own
// recognizer and dictionary file.

use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::download::{Downloader, Job, Update};

pub const BASE_URL: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0";

#[derive(Debug, Clone, Copy)]
pub struct Asset {
    pub file: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub note: &'static str,
    pub recognizer: Asset,
    pub dict: Asset,
}

impl Model {
    /// 【识别模型】【语言名称】按 chinese 返回本地化的语言组名称，无文件或网络访问
    pub fn label(&self, chinese: bool) -> &'static str {
        if !chinese {
            return self.name;
        }
        match self.id {
            "latin" => "英文与拉丁语系",
            "cyrillic" => "西里尔语系",
            "arabic" => "阿拉伯语系",
            "greek" => "希腊语",
            "chinese" => "中文与日文",
            "korean" => "韩语",
            "thai" => "泰语",
            _ => self.name,
        }
    }
}

pub const DETECTOR: Asset = Asset {
    file: "pp-ocrv5_mobile_det.onnx",
    size: 4_826_518,
    sha256: "1eb7b4f7ab657ebd1c66d5f79bca7497f29768a2e3c15e52daecbba1a8e4a039",
};

pub const MODELS: &[Model] = &[
    Model {
        id: "latin",
        name: "Latin",
        note: "English, French, German and etc",
        recognizer: Asset {
            file: "latin_pp-ocrv5_mobile_rec.onnx",
            size: 8_069_614,
            sha256: "e3a6bfeea1c8a01d6fccfd480a0bd363fd907f8c65931e228bb2736f5c3e142f",
        },
        dict: Asset {
            file: "ppocrv5_latin_dict.txt",
            size: 2_616,
            sha256: "ccbcc45730b3fbbd9050c5bc74db6a99067141ef1035e3d14889a84a6b9b1aff",
        },
    },
    Model {
        id: "cyrillic",
        name: "Cyrillic",
        note: "Russian, Ukrainian, Bulgarian",
        recognizer: Asset {
            file: "cyrillic_pp-ocrv5_mobile_rec.onnx",
            size: 8_076_390,
            sha256: "a18d96d7c8d73d90f2ed056549caa1de3a8e6cb744cccba16cd593ea8cd2d569",
        },
        dict: Asset {
            file: "ppocrv5_cyrillic_dict.txt",
            size: 2_781,
            sha256: "db40aa52ceb112055be80c694afdf655d5d2c4f7873704524cc16a447ca913ba",
        },
    },
    Model {
        id: "arabic",
        name: "Arabic",
        note: "Arabic, Persian, Urdu",
        recognizer: Asset {
            file: "arabic_pp-ocrv5_mobile_rec.onnx",
            size: 8_026_538,
            sha256: "2768206d9a0ce48eba45b59619184e18161dde8f44115f029920ca17a9dc0384",
        },
        dict: Asset {
            file: "ppocrv5_arabic_dict.txt",
            size: 2_369,
            sha256: "7f92f7dbb9b75a4787a83bfb4f6d14a8ab515525130c9d40a9036f61cf6999e9",
        },
    },
    Model {
        id: "greek",
        name: "Greek",
        note: "Modern Greek",
        recognizer: Asset {
            file: "el_pp-ocrv5_mobile_rec.onnx",
            size: 7_836_326,
            sha256: "5a4a020e48e8783e035e1af135423c2161a363acab9ef16e48238c3d181f0f71",
        },
        dict: Asset {
            file: "ppocrv5_el_dict.txt",
            size: 1_103,
            sha256: "31defc62c0c3ad3674a82da6192226a2ba98ef4ff014a7045cb88d59f9c3de31",
        },
    },
    Model {
        id: "chinese",
        name: "Chinese & Japanese",
        note: "Hanzi, kanji and kana",
        recognizer: Asset {
            file: "pp-ocrv5_mobile_rec.onnx",
            size: 16_562_373,
            sha256: "243a0f06d826761323e9045e9b113ab2c191c3aa50565585e628300b8eda0224",
        },
        dict: Asset {
            file: "ppocrv5_dict.txt",
            size: 74_012,
            sha256: "d1979e9f794c464c0d2e0b70a7fe14dd978e9dc644c0e71f14158cdf8342af1b",
        },
    },
    Model {
        id: "korean",
        name: "Korean",
        note: "Hangul",
        recognizer: Asset {
            file: "korean_pp-ocrv5_mobile_rec.onnx",
            size: 13_446_374,
            sha256: "2d7ed96308065a86103325d22af07a88c4d06afc009f21602a4882342c0cc054",
        },
        dict: Asset {
            file: "ppocrv5_korean_dict.txt",
            size: 47_451,
            sha256: "a88071c68c01707489baa79ebe0405b7beb5cca229f4fc94cc3ef992328802d7",
        },
    },
    Model {
        id: "thai",
        name: "Thai",
        note: "Thai script",
        recognizer: Asset {
            file: "th_pp-ocrv5_mobile_rec.onnx",
            size: 7_918_606,
            sha256: "5f6ee21242691681261fee01bc39867da9cc8ff9b889f2f048b3cb7f74380217",
        },
        dict: Asset {
            file: "ppocrv5_th_dict.txt",
            size: 1_767,
            sha256: "57f5406f94bb6688fb7077f7be65f08bbd71cecf48c01ea26c522cb5c4836b7a",
        },
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFiles {
    pub detector: PathBuf,
    pub recognizer: PathBuf,
    pub dict: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStatus {
    Missing,
    Queued,
    Downloading(u8),
    Installed,
    Failed,
}

pub enum ModelEvent {
    Progress,
    Ready(usize),
    Failed(usize, String),
}

pub struct OcrModels {
    pub library: super::runtime_library::Installer,
    dir: Option<PathBuf>,
    status: Vec<ModelStatus>,
    active: Option<usize>,
    recommended: usize,
    detector_on_disk: bool,
    // Task ID and cancellation flag. Responses from canceled tasks are filtered out by ID.
    jobs: Vec<Option<(u64, Arc<AtomicBool>)>>,
    next_job: u64,
    downloader: Downloader,
}

impl OcrModels {
    pub fn load() -> Self {
        let mut models = Self {
            library: super::runtime_library::Installer::default(),
            dir: models_dir(),
            status: vec![ModelStatus::Missing; MODELS.len()],
            active: None,
            recommended: recommended_for_locale(),
            detector_on_disk: false,
            jobs: vec![None; MODELS.len()],
            next_job: 0,
            downloader: Downloader::default(),
        };
        models.refresh();
        let installed = |idx: &usize| models.status[*idx] == ModelStatus::Installed;
        models.active = read_choice()
            .and_then(|id| MODELS.iter().position(|m| m.id == id.trim()))
            .filter(installed)
            .or_else(|| (0..MODELS.len()).find(installed));
        models
    }

    pub fn status(&self, idx: usize) -> ModelStatus {
        self.status[idx]
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    pub fn is_downloading(&self) -> bool {
        self.status.iter().copied().any(in_flight)
    }

    /// 【识别模型】【取消排空】返回下载器是否仍需消费进度或完成消息，无参数，取消不以界面状态代替实际队列结束
    pub fn needs_poll(&self) -> bool {
        self.downloader.has_pending() || self.library.needs_poll()
    }

    pub fn recommended(&self) -> usize {
        self.recommended
    }

    pub fn installed_count(&self) -> usize {
        self.status
            .iter()
            .filter(|&&status| status == ModelStatus::Installed)
            .count()
    }

    pub fn download_progress(&self) -> Option<u8> {
        let mut queued = false;
        for status in &self.status {
            match status {
                ModelStatus::Downloading(percent) => return Some(*percent),
                ModelStatus::Queued => queued = true,
                _ => {}
            }
        }
        queued.then_some(0)
    }

    pub fn active_installed(&self) -> bool {
        self.active
            .is_some_and(|idx| self.status[idx] == ModelStatus::Installed)
    }

    /// 【识别模型】【损坏恢复】只删除应用模型目录中校验失败的当前资源；返回是否需要重新下载
    pub fn repair_active(&mut self) -> bool {
        let Some(index) = self.active else {
            return false;
        };
        let Some(files) = self.files(index) else {
            return false;
        };
        let Err(error) = super::model_integrity::verify(&files) else {
            return false;
        };
        if error.path.parent() != self.dir.as_deref() {
            return false;
        }
        if let Err(remove_error) = std::fs::remove_file(&error.path)
            && remove_error.kind() != std::io::ErrorKind::NotFound
        {
            error!("【识别模型】【损坏恢复】无法删除损坏资源: {remove_error}");
        }
        self.refresh();
        self.status[index] = ModelStatus::Failed;
        self.active = None;
        self.save_choice();
        true
    }

    pub fn files(&self, idx: usize) -> Option<ModelFiles> {
        if self.status[idx] != ModelStatus::Installed {
            return None;
        }
        let dir = self.dir.as_deref()?;
        let model = &MODELS[idx];
        Some(ModelFiles {
            detector: dir.join(DETECTOR.file),
            recognizer: dir.join(model.recognizer.file),
            dict: dir.join(model.dict.file),
        })
    }

    pub fn download_size(&self, idx: usize) -> u64 {
        let model = &MODELS[idx];
        let detector = if self.detector_on_disk {
            0
        } else {
            DETECTOR.size
        };
        detector + model.recognizer.size + model.dict.size
    }

    pub fn set_active(&mut self, idx: Option<usize>) {
        if self.active == idx {
            return;
        }
        self.active = idx;
        self.save_choice();
    }

    pub fn download(&mut self, idx: usize) {
        let Some(dir) = self.dir.clone() else {
            error!("ocr: no data directory to download models into");
            self.status[idx] = ModelStatus::Failed;
            return;
        };
        let model = &MODELS[idx];
        let cancel = Arc::new(AtomicBool::new(false));
        self.next_job += 1;
        self.jobs[idx] = Some((self.next_job, cancel.clone()));
        self.status[idx] = ModelStatus::Queued;
        self.downloader.push(Job {
            id: self.next_job,
            model: idx,
            dir,
            assets: [DETECTOR, model.recognizer, model.dict],
            cancel,
        });
    }

    pub fn shutdown(&mut self) {
        self.downloader.stop();
        self.library.cancel();
    }

    pub fn cancel(&mut self, idx: usize) {
        if let Some((_, flag)) = self.jobs[idx].take() {
            flag.store(true, Ordering::Relaxed);
        }
        self.status[idx] = ModelStatus::Missing;
    }

    /// detecor delete only when the last model is removed
    pub fn remove(&mut self, idx: usize) {
        let Some(dir) = self.dir.clone() else { return };
        let model = &MODELS[idx];
        let others = (0..MODELS.len()).any(|i| {
            i != idx && (self.status[i] == ModelStatus::Installed || in_flight(self.status[i]))
        });

        let mut files = vec![model.recognizer.file, model.dict.file];
        if !others {
            files.push(DETECTOR.file);
        }
        for file in files {
            if let Err(e) = std::fs::remove_file(dir.join(file))
                && e.kind() != std::io::ErrorKind::NotFound
            {
                info!("ocr: failed to remove {file}: {e}");
            }
        }
        self.refresh();
    }

    pub fn poll(&mut self) -> Vec<ModelEvent> {
        let mut events = Vec::new();
        if self.library.poll() {
            events.push(ModelEvent::Progress);
        }
        while let Some(update) = self.downloader.try_recv() {
            let (id, idx) = update.job();
            if self.jobs[idx]
                .as_ref()
                .is_none_or(|(current, _)| *current != id)
            {
                continue;
            }
            match update {
                Update::Progress { percent, .. } => {
                    self.status[idx] = ModelStatus::Downloading(percent);
                    events.push(ModelEvent::Progress);
                }
                Update::Done { result, .. } => {
                    self.jobs[idx] = None;
                    match result {
                        Ok(()) => {
                            self.detector_on_disk = true;
                            self.status[idx] = ModelStatus::Installed;
                            events.push(ModelEvent::Ready(idx));
                        }
                        Err(e) => {
                            self.status[idx] = ModelStatus::Failed;
                            events.push(ModelEvent::Failed(idx, e));
                        }
                    }
                }
            }
        }
        events
    }

    fn refresh(&mut self) {
        let Some(dir) = self.dir.as_deref() else {
            return;
        };
        self.detector_on_disk = on_disk(dir, &DETECTOR);
        for (idx, model) in MODELS.iter().enumerate() {
            if in_flight(self.status[idx]) {
                continue;
            }
            let installed = self.detector_on_disk
                && on_disk(dir, &model.recognizer)
                && on_disk(dir, &model.dict);
            self.status[idx] = if installed {
                ModelStatus::Installed
            } else {
                ModelStatus::Missing
            };
        }
    }

    fn save_choice(&self) {
        let Some(path) = choice_path() else { return };
        let id = self.active.map_or("", |idx| MODELS[idx].id);
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&path, id));
        if let Err(e) = written {
            info!("ocr: failed to save the model choice: {e}");
        }
    }
}

pub(super) fn on_disk(dir: &Path, asset: &Asset) -> bool {
    std::fs::metadata(dir.join(asset.file)).is_ok_and(|m| m.len() == asset.size)
}

fn in_flight(status: ModelStatus) -> bool {
    matches!(status, ModelStatus::Queued | ModelStatus::Downloading(_))
}

fn models_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("mark-shot").join("models"))
}

fn choice_path() -> Option<PathBuf> {
    Some(dirs::state_dir()?.join("mark-shot").join("ocr-model"))
}

fn read_choice() -> Option<String> {
    std::fs::read_to_string(choice_path()?).ok()
}

fn recommended_for_locale() -> usize {
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default();
    let lang = locale.split(['_', '.', '@']).next().unwrap_or_default();

    let id = match lang {
        "ru" | "uk" | "be" | "bg" | "sr" | "mk" | "kk" | "ky" | "tg" | "mn" => "cyrillic",
        "ar" | "fa" | "ur" | "ps" | "ug" => "arabic",
        "el" => "greek",
        "zh" | "ja" => "chinese",
        "ko" => "korean",
        "th" => "thai",
        _ => "latin",
    };
    MODELS.iter().position(|m| m.id == id).unwrap_or(0)
}
