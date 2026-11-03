mod char_alignment;
use char_alignment::char_boundaries;

// Runs PP-OCRv5 mobile models via ONNX Runtime using the `oar-ocr` crate.
//
// Highlights:
// - Uses the model picked in the language list (see `models`), e.g. Cyrillic supports Cyrillic, English, digits, and punctuation.
// - Processes image crops one by one. Batching or multi-threading is avoided
//   because it slows down execution and wastes memory cache.
// - Uses direct low-level API access to get exact character positions (`return_word_box`)
//   for precise sub-line text selection.
// - Logs performance metrics (`ocr: timing`) to stderr. (temporary).

use log::info;
use std::time::Instant;

use oar_ocr::core::config::{OrtExecutionProvider, OrtSessionConfig};
use oar_ocr::core::traits::task::ImageTaskInput;
use oar_ocr::core::traits::{AdapterBuilder, OrtConfigurable};
use oar_ocr::domain::adapters::TextRecognitionAdapterBuilder;
use oar_ocr::domain::tasks::{TextDetectionConfig, TextRecognitionConfig, TextRecognitionTask};
use oar_ocr::predictors::{TaskPredictorCore, TextDetectionPredictor};
use oar_ocr::processors::{LimitType, Point};
use oar_ocr::utils::get_rotate_crop_image;
use tiny_skia::Rect;

use crate::config::OcrFilter;

use super::layout;
use super::lookalikes;
use super::models::ModelFiles;
use super::{OcrBackend, OcrError, OcrImage, OcrLine, OcrText};

type Recognizer = TaskPredictorCore<TextRecognitionTask>;

/// not so sure how its going to work on low res or high res monitors
/// (TODO)
pub struct PaddleBackend {
    detector: TextDetectionPredictor,
    recognizer: Recognizer,
}

impl PaddleBackend {
    pub fn new(files: &ModelFiles) -> Result<Self, OcrError> {
        Self::build(files, false)
    }

    pub fn on_gpu(files: &ModelFiles) -> Result<Self, OcrError> {
        Self::build(files, true)
    }

    fn build(files: &ModelFiles, gpu: bool) -> Result<Self, OcrError> {
        let total = Instant::now();
        super::model_integrity::verify(files)?;
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(crate::config::get().ocr.threads.clamp(1, 16));
        super::parallelism::initialize(threads);
        let mut providers = Vec::new();
        if gpu {
            super::dawn::load()?;
            providers.push(OrtExecutionProvider::WebGPU);
        }
        // Explicitly configuring the CPU Execution Provider disables ORT's arena allocator.
        // With the arena allocator enabled, scanning a 1920x2160 frame retains +-1.1 GB resident memory indefinitely;
        // without it, memory drops to ~145 MB, at the cost of a ~5-10% latency penalty per scan.
        providers.push(OrtExecutionProvider::CPU);
        let mut ort = OrtSessionConfig::new()
            .with_intra_threads(threads)
            .with_execution_providers(providers)
            .add_config_entry("session.intra_op.allow_spinning", "0");
        if gpu {
            // WebGPU keeps buffers sized for the largest image
            // Without cache it stays under +-130 MB, with only a +-5% slowdown on scans.
            for kind in ["storage", "uniform", "queryResolve", "default"] {
                ort = ort.add_config_entry(
                    format!("ep.webgpuexecutionprovider.{kind}BufferCacheMode"),
                    "disabled",
                );
            }
        }

        let stage = Instant::now();
        let detect = &crate::config::get().ocr.detect;
        let detector = TextDetectionPredictor::builder()
            .with_config(TextDetectionConfig {
                score_threshold: detect.score_threshold,
                box_threshold: detect.box_threshold,
                unclip_ratio: detect.unclip_ratio,
                max_candidates: detect.max_candidates,
                // longest side the detector sees; the stock 960 halves a 1080p grab and loses small UI text
                limit_side_len: Some(detect.max_side),
                limit_type: Some(LimitType::Max),
                max_side_len: Some(4096),
            })
            .with_ort_config(ort.clone())
            .build(files.detector.as_path())?;
        let detector_ms = ms(stage);

        let stage = Instant::now();
        let dict: Vec<String> = std::fs::read_to_string(&files.dict)?
            .lines()
            .map(str::to_owned)
            .collect();
        let rec_config = TextRecognitionConfig {
            score_threshold: 0.0,
        };
        let rec_adapter = TextRecognitionAdapterBuilder::new()
            .with_config(rec_config.clone())
            .character_dict(dict)
            .return_word_box(true) // per-character x positions
            .with_ort_config(ort.clone())
            .build(files.recognizer.as_path())?;
        let recognizer = TaskPredictorCore::new(
            Box::new(rec_adapter),
            TextRecognitionTask::new(rec_config.clone()),
            rec_config,
        );
        let recognizer_ms = ms(stage);

        info!(
            "ocr: engine ready in {:.0}ms  (detector {:.0} | recognizer {:.0})",
            ms(total),
            detector_ms,
            recognizer_ms,
        );

        Ok(Self {
            detector,
            recognizer,
        })
    }
}

impl OcrBackend for PaddleBackend {
    fn recognize(
        &self,
        image: OcrImage,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OcrText, OcrError> {
        let total = Instant::now();

        let stage = Instant::now();
        let (width, height, (ox, oy)) = (image.width, image.height, image.origin);
        // By value, so the full-screen buffer isn't copied.
        let img = image::RgbImage::from_raw(width, height, image.rgb)
            .ok_or("ocr: RGB buffer does not match its dimensions")?;
        let area = Rect::from_xywh(ox, oy, width as f32, height as f32);
        // adding pad for image otherwise on small images where text is close to border
        // some text will be missing, if not all
        let (img, (left, top)) = pad_for_detection(img);
        // real positinos of text, since image could be changed
        let (ox, oy) = (ox - left as f32, oy - top as f32);
        let decode_ms = ms(stage);

        let stage = Instant::now();
        // `predict` takes ownership and the adapter unwraps it, so this clone
        // is unavoidable: the pixels are needed again to cut the line crops.
        let detected = self.detector.predict(vec![img.clone()])?;
        let detect_ms = ms(stage);
        if cancelled() {
            return Err(super::CANCELLED.into());
        }
        let Some(regions) = detected.detections.into_iter().next() else {
            return Ok(OcrText::default());
        };

        // Boxes in image coordinates, keeping each quad for the rotated crop.
        let stage = Instant::now();
        let mut quads: Vec<Vec<Point>> = Vec::new();
        let mut boxes: Vec<Rect> = Vec::new();
        for region in regions {
            let pts = region.bbox.points;
            if pts.len() != 4 {
                continue;
            }
            let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for p in &pts {
                l = l.min(p.x);
                t = t.min(p.y);
                r = r.max(p.x);
                b = b.max(p.y);
            }
            let Some(rect) = Rect::from_ltrb(l, t, r, b) else {
                continue;
            };
            quads.push(pts);
            boxes.push(rect);
        }
        if boxes.is_empty() {
            return Ok(OcrText::default());
        }

        // Specks of window chrome come back as detections. Dropping them here,
        // by size alone, keeps them out of the merged crops below.
        let body_height = median_height(&boxes);
        let keep: Vec<usize> = (0..boxes.len())
            .filter(|&i| !is_speck(&boxes[i], body_height))
            .collect();

        // One crop per visual line rather than per detection box: the detector
        // splits a line wherever the spacing widens, and each extra call costs
        // a fixed ~9ms on top of the per-pixel work.
        let rows = layout::row_groups(
            &keep.iter().map(|&i| boxes[i]).collect::<Vec<_>>(),
            |a, b| has_rule(&img, a, b),
        );
        let mut rects: Vec<Rect> = Vec::with_capacity(rows.len());
        let mut crops: Vec<image::RgbImage> = Vec::with_capacity(rows.len());
        for row in &rows {
            let members: Vec<usize> = row.iter().map(|&r| keep[r]).collect();
            let crop = if let [only] = members[..] {
                // A lone box may be skewed, so go through the quad.
                match get_rotate_crop_image(&img, &quads[only]) {
                    Ok(crop) => crop,
                    Err(_) => continue,
                }
            } else {
                match axis_aligned_crop(&img, &union_of(&members, &boxes)) {
                    Some(crop) => crop,
                    None => continue,
                }
            };
            let merged = union_of(&members, &boxes);
            let Some(global) = Rect::from_ltrb(
                merged.left() + ox,
                merged.top() + oy,
                merged.right() + ox,
                merged.bottom() + oy,
            ) else {
                continue;
            };
            rects.push(global);
            crops.push(crop);
        }
        let crop_ms = ms(stage);
        if crops.is_empty() {
            return Ok(OcrText::default());
        }

        let stage = Instant::now();
        let mut reads: Vec<Read> = Vec::with_capacity(crops.len());
        for crop in &crops {
            if cancelled() {
                return Err(super::CANCELLED.into());
            }
            reads.push(recognize_one(&self.recognizer, crop));
        }
        let recognize_ms = ms(stage);

        let stage = Instant::now();
        let mut lines: Vec<OcrLine> = rects
            .into_iter()
            .zip(reads)
            .filter(|(bounds, read)| !is_noise(bounds, read, body_height))
            .map(|(bounds, read)| {
                let text = if crate::config::get().ocr.lookalikes {
                    lookalikes::fix(&read.text)
                } else {
                    read.text
                };
                let char_x = char_boundaries(&bounds, &text, &read.positions);
                OcrLine {
                    text,
                    bounds,
                    char_x,
                }
            })
            // recognizer's positions are fractions of the unclipped crop
            // this is why its on purpose after char_boundaries
            .filter_map(|line| match area {
                Some(area) => clip_line(line, area),
                None => Some(line),
            })
            .collect();
        lines.sort_by(|a, b| {
            a.bounds
                .top()
                .partial_cmp(&b.bounds.top())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.bounds
                        .left()
                        .partial_cmp(&b.bounds.left())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let sort_ms = ms(stage);

        info!(
            "ocr: timing {}x{}px -> {} lines in {:.0}ms  (decode {:.0} | detect {:.0} | \
             crop {:.0} [{} boxes -> {} crops] | recognize {:.0} | sort {:.1})",
            width,
            height,
            lines.len(),
            ms(total),
            decode_ms,
            detect_ms,
            crop_ms,
            boxes.len(),
            crops.len(),
            recognize_ms,
            sort_ms,
        );

        Ok(OcrText { lines })
    }
}

/// adds padding around text
/// since oar-ocr rounds each side to the nearest multiple of 32 on its own, so
/// small crops get squashed or stretched, causing the detector to lose text closer
/// to edge.
///
/// Big regions are skipped
fn pad_for_detection(img: image::RgbImage) -> (image::RgbImage, (u32, u32)) {
    // not sure if it needs to be configurable
    // but i will leave it as it is at least for testing (with or without pad (pad=0))
    let detect = &crate::config::get().ocr.detect;
    let (w, h) = img.dimensions();
    let grow = |n: u32| (u64::from(n) + 2 * u64::from(detect.pad)).div_ceil(32) * 32;
    let (pw, ph) = (grow(w), grow(h));
    if detect.pad == 0 || w == 0 || h == 0 || pw.max(ph) > u64::from(detect.max_side) {
        return (img, (0, 0));
    }
    let (pw, ph) = (pw as u32, ph as u32);
    let (left, top) = ((pw - w) / 2, (ph - h) / 2);
    let mut out = image::RgbImage::from_pixel(pw, ph, border_median(&img));
    image::imageops::replace(&mut out, &img, left.into(), top.into());
    (out, (left, top))
}

/// Practically a copy of oar-ocr's `calculate_smart_padding_color`,
/// but without its +-16 away from black and white
fn border_median(img: &image::RgbImage) -> image::Rgb<u8> {
    let (w, h) = img.dimensions();
    let ring = (0..w)
        .flat_map(|x| [(x, 0), (x, h - 1)])
        .chain((1..h.saturating_sub(1)).flat_map(|y| [(0, y), (w - 1, y)]));
    let mut channels: [Vec<u8>; 3] = Default::default();
    for (x, y) in ring {
        for (values, &c) in channels.iter_mut().zip(&img.get_pixel(x, y).0) {
            values.push(c);
        }
    }
    image::Rgb(channels.map(|mut values| {
        let mid = values.len() / 2;
        *values.select_nth_unstable(mid).1
    }))
}

/// Detector boxes are wider than the text, so near the selection's edge
/// they stick out into the padding. Cut them back to the selection.
///
/// A line with nothing left inside is dropped
fn clip_line(mut line: OcrLine, area: Rect) -> Option<OcrLine> {
    line.bounds = line
        .bounds
        .intersect(&area)
        .filter(|b| b.width() > 0.0 && b.height() > 0.0)?;
    for x in &mut line.char_x {
        *x = x.clamp(line.bounds.left(), line.bounds.right());
    }
    Some(line)
}

/// One crop's recognition result.
#[derive(Default)]
struct Read {
    text: String,
    /// Per-character position, 0..1 across the crop's own width.
    positions: Vec<f32>,
    confidence: f32,
}

/// Milliseconds since `start`, for the timing line.
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn recognize_one(recognizer: &Recognizer, crop: &image::RgbImage) -> Read {
    let Ok(mut out) = recognizer.predict(ImageTaskInput::new(vec![crop.clone()])) else {
        return Read::default();
    };
    let steps = out.sequence_lengths.first().copied().unwrap_or(0);
    let positions = out.char_positions.drain(..).next().unwrap_or_default();
    Read {
        text: out.texts.drain(..).next().unwrap_or_default(),
        positions: to_crop_space(positions, steps, crop.width(), crop.height()),
        confidence: out.scores.first().copied().unwrap_or(0.0),
    }
}

// Adjusts character positions returned by the OCR model.
//
// recognizer scales image crops to 48px high and pads them to a fixed width.
// Because of this extra padding, character coordinates can get squished
// to the left, especially on short lines.
const REC_HEIGHT: f32 = 48.0;
const REC_WIDTH: f32 = 320.0;
const REC_MAX_WIDTH: f32 = 3200.0;

/// Padded-tensor positions -> fractions of the crop itself.
fn to_crop_space(mut positions: Vec<f32>, steps: usize, w: u32, h: u32) -> Vec<f32> {
    let ratio = w as f32 / h.max(1) as f32;
    let tensor_w = (REC_HEIGHT * ratio.max(REC_WIDTH / REC_HEIGHT))
        .floor()
        .min(REC_MAX_WIDTH);
    let content_w = (REC_HEIGHT * ratio).ceil().min(tensor_w).max(1.0);

    let half_step = if steps > 0 { 0.5 / steps as f32 } else { 0.0 };
    for p in &mut positions {
        *p = ((*p + half_step) * tensor_w / content_w).clamp(0.0, 1.0);
    }
    positions
}

fn filter() -> &'static OcrFilter {
    &crate::config::get().ocr.filter
}

/// A detection far smaller than the body text in both directions, which is
/// what visual noise looks like. The recognizer cannot answer "not text" and
/// would return its best single-character guess for it.
fn is_speck(bounds: &Rect, body_height: f32) -> bool {
    let f = filter();
    if bounds.height() < f.speck_size * body_height && bounds.width() < f.speck_size * body_height {
        return true;
    }
    // A detection this much taller than the body text, and no wider than it is
    // tall, is an icon or a logo rather than a line of text.
    bounds.height() > f.glyph_height * body_height
        && bounds.width() < f.glyph_aspect * bounds.height()
}

/// Whether a read should be dropped instead of becoming a line.
fn is_noise(bounds: &Rect, read: &Read, body_height: f32) -> bool {
    let f = filter();
    if read.text.trim().is_empty() || read.confidence < f.min_confidence {
        return true;
    }
    read.text.chars().count() <= 1
        && (bounds.height() < f.single_char_height * body_height
            || read.confidence < f.single_char_confidence)
}

// Checks for a vertical ruling line in the gap between two boxes on the same line.
// Separated boxes are split into distinct crops for OCR processing.
//
// rule_contrast: minimum brightness difference between a line pixel and the background.
// rule_coverage: minimum fraction of a segment that must be such pixels.
// rule_reach: vertical search past a text line, as a fraction of its height.
// Cell borders usually extend beyond text boundaries, unlike pipe characters (`|`).
fn has_rule(img: &image::RgbImage, a: &Rect, b: &Rect) -> bool {
    let f = filter();
    let (top, bottom) = (a.top().min(b.top()), a.bottom().max(b.bottom()));
    let reach = (bottom - top) * f.rule_reach;
    let clamp_y = |y: f32| (y.max(0.0) as u32).min(img.height());
    let x0 = a.right().ceil().max(0.0) as u32;
    let x1 = (b.left().floor().max(0.0) as u32).min(img.width());
    let (above, y0, y1, below) = (
        clamp_y(top - reach),
        clamp_y(top),
        clamp_y(bottom),
        clamp_y(bottom + reach),
    );
    if x1 <= x0 || y1 <= y0 {
        return false;
    }

    let luma = |x: u32, y: u32| {
        let [r, g, b] = img.get_pixel(x, y).0;
        (i32::from(r) * 299 + i32::from(g) * 587 + i32::from(b) * 114) / 1000
    };
    let mut values: Vec<i32> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .map(|(x, y)| luma(x, y))
        .collect();
    let mid = values.len() / 2;
    let background = *values.select_nth_unstable(mid).1;

    let covered = |x: u32, from: u32, to: u32| {
        let needed = ((to - from) as f32 * f.rule_coverage).ceil() as usize;
        to > from
            && (from..to)
                .filter(|&y| (luma(x, y) - background).abs() > f.rule_contrast)
                .count()
                >= needed
    };
    (x0..x1).any(|x| covered(x, y0, y1) && (covered(x, above, y0) || covered(x, y1, below)))
}

/// Smallest rectangle covering the listed boxes.
fn union_of(members: &[usize], boxes: &[Rect]) -> Rect {
    let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &i in members {
        l = l.min(boxes[i].left());
        t = t.min(boxes[i].top());
        r = r.max(boxes[i].right());
        b = b.max(boxes[i].bottom());
    }
    Rect::from_ltrb(l, t, r, b).unwrap_or(boxes[members[0]])
}

/// Cut `rect` out of `img`, clamped to the image.
fn axis_aligned_crop(img: &image::RgbImage, rect: &Rect) -> Option<image::RgbImage> {
    let x = rect.left().floor().max(0.0) as u32;
    let y = rect.top().floor().max(0.0) as u32;
    let right = (rect.right().ceil() as u32).min(img.width());
    let bottom = (rect.bottom().ceil() as u32).min(img.height());
    if right <= x || bottom <= y {
        return None;
    }
    Some(image::imageops::crop_imm(img, x, y, right - x, bottom - y).to_image())
}

/// Median detection-box height, used as the body text size.
fn median_height(rects: &[Rect]) -> f32 {
    if rects.is_empty() {
        return 1.0;
    }
    let mut heights: Vec<f32> = rects.iter().map(|r| r.height()).collect();
    heights.sort_by(f32::total_cmp);
    heights[heights.len() / 2].max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_boundaries_sit_on_the_box_edges() {
        let bounds = Rect::from_ltrb(100.0, 0.0, 610.0, 115.0).unwrap();
        let xs = char_boundaries(&bounds, "MONDAY", &[0.2, 0.35, 0.45, 0.6, 0.75, 0.9]);
        assert_eq!(xs.len(), 7);
        assert_eq!((xs[0], xs[6]), (100.0, 610.0));
        assert!(xs.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn small_images_are_padded_to_multiples_of_32() {
        let mut img = image::RgbImage::from_pixel(100, 20, image::Rgb([200, 210, 220]));
        // this pixel is used to check the image lands at (left, top) unchanged
        // since even with pads change it stay the same
        img.put_pixel(50, 10, image::Rgb([0, 0, 0]));
        let (out, (left, top)) = pad_for_detection(img);
        assert_eq!((out.width() % 32, out.height() % 32), (0, 0));
        assert!(left >= 16 && top >= 16);
        assert_eq!(out.get_pixel(left + 50, top + 10).0, [0, 0, 0]);
        assert_eq!(out.get_pixel(0, 0).0, [200, 210, 220]);
    }

    #[test]
    fn images_near_max_side_are_not_padded() {
        let (out, offset) = pad_for_detection(image::RgbImage::new(1900, 100));
        assert_eq!((out.dimensions(), offset), ((1900, 100), (0, 0)));
    }

    #[test]
    fn lines_are_cut_to_the_scanned_area() {
        let line = OcrLine {
            text: "ab".into(),
            bounds: Rect::from_ltrb(-3.0, 5.0, 40.0, 30.0).unwrap(),
            char_x: vec![-3.0, 20.0, 40.0],
        };
        let area = Rect::from_ltrb(0.0, 0.0, 30.0, 25.0).unwrap();
        let cut = clip_line(line.clone(), area).unwrap();
        assert_eq!(cut.bounds, Rect::from_ltrb(0.0, 5.0, 30.0, 25.0).unwrap());
        assert_eq!(cut.char_x, vec![0.0, 20.0, 30.0]);
        let beside = Rect::from_ltrb(50.0, 0.0, 60.0, 10.0).unwrap();
        let touching = Rect::from_ltrb(0.0, 0.0, 30.0, 5.0).unwrap();
        assert!(clip_line(line.clone(), beside).is_none());
        assert!(clip_line(line, touching).is_none());
    }
}
