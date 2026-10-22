//! Title recogniser (ADR 0002): ocrs, recognition model only. We supply the two line
//! boxes from our own mask and line split, so ocrs' detection model is not shipped.

use crate::Frame;
use crate::prep::{Numeral, PAD, Prepared, Rect, prepare};
use image::{GrayImage, RgbImage};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem, TextLine};
use rten::Model;
use rten_imageproc::{RectF, RotatedRect};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The bundled ocrs recognition model (CC-BY-SA-4.0, see `models/NOTICE.md`).
static RECOGNITION_MODEL: &[u8] = include_bytes!("../models/text-recognition.onnx");

/// Margin around each line box handed to ocrs (it fills outside the box with black).
const LINE_MARGIN: f32 = 3.0;

/// How many lines' readings [`RememberedLines`] keeps before it forgets them all at once.
const REMEMBERED: usize = 512;

/// What lines of ink read as, remembered by their ink: a frame read again unchanged costs
/// no recognition, the costly part of reading it.
#[derive(Default)]
pub(crate) struct RememberedLines {
    read: HashMap<(u32, Vec<u8>), Option<String>>,
}

impl RememberedLines {
    /// What each of `inks` (black on white) reads as, enlarged `upscale` times for ocrs:
    /// those not read before, all at once.
    pub fn read(&mut self, recogniser: &Recogniser, inks: &[GrayImage], upscale: u32) -> Vec<Option<String>> {
        let key = |ink: &GrayImage| (ink.width(), ink.as_raw().clone());
        let keys: Vec<(u32, Vec<u8>)> = inks.iter().map(key).collect();
        let unknown: Vec<usize> = (0..inks.len()).filter(|&i| !self.read.contains_key(&keys[i])).collect();
        if !unknown.is_empty() {
            let read = recogniser.read_stacked(&unknown.iter().map(|&i| &inks[i]).collect::<Vec<_>>(), upscale);
            if self.read.len() + unknown.len() > REMEMBERED {
                self.read.clear();
            }
            for (&i, text) in unknown.iter().zip(read) {
                self.read.insert(keys[i].clone(), text);
            }
        }
        keys.iter().map(|key| self.read.get(key).cloned().flatten()).collect()
    }
}

/// The two raw text lines read from the minimap title, before they are snapped to a
/// real map.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TitleLines {
    pub street: String,
    pub map: String,
}

/// What one Title Region frame reads as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleReading {
    /// `None` when the frame is unreadable (loading screen, minimap hidden or covered).
    pub lines: Option<TitleLines>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// Reads Title Region frames with the bundled recogniser (ADR 0002). Reading takes
/// tens of milliseconds, so the shell keeps one on its own thread and hands the core
/// each [`TitleReading`] (`Minerva::on_reading`).
pub struct TitleReader {
    recogniser: Recogniser,
}

impl TitleReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled() }
    }

    pub fn read(&self, frame: &Frame) -> TitleReading {
        let lines = frame_to_rgb(frame).and_then(|img| self.recogniser.read(&img));
        TitleReading { lines, at_ms: frame.at_ms }
    }
}

pub(crate) fn frame_to_rgb(frame: &Frame) -> Option<RgbImage> {
    let (w, h) = (frame.width as usize, frame.height as usize);
    if frame.rgba.len() != w * h * 4 {
        return None;
    }
    let rgb = frame.rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
    RgbImage::from_raw(frame.width, frame.height, rgb)
}

/// The bundled engine, loaded on first use and shared by every reader.
fn engine() -> &'static OcrEngine {
    static ENGINE: OnceLock<OcrEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let model = Model::load(RECOGNITION_MODEL.to_vec()).expect("bundled recognition model loads");
        OcrEngine::new(OcrEngineParams { recognition_model: Some(model), ..Default::default() })
            .expect("ocrs engine builds")
    })
}

/// The bundled recogniser: reads lines of black-on-white text at boxes we found.
#[derive(Clone, Copy)]
pub(crate) struct Recogniser {
    engine: &'static OcrEngine,
}

impl Recogniser {
    pub fn bundled() -> Self {
        Self { engine: engine() }
    }

    /// Read the street line and the map line. `None` means unreadable.
    fn read(&self, img: &RgbImage) -> Option<TitleLines> {
        let prepared = prepare(img)?;
        self.read_prepared(&prepared)
    }

    /// Read each line box (canvas coordinates) of a black-on-white canvas: one entry per
    /// box, `None` where nothing was read. `None` overall if ocrs couldn't run.
    pub fn read_lines(&self, canvas: &GrayImage, lines: &[Rect]) -> Option<Vec<Option<TextLine>>> {
        let (w, h) = canvas.dimensions();
        let src = ImageSource::from_bytes(canvas.as_raw(), (w, h)).ok()?;
        let input = self.engine.prepare_input(src).ok()?;
        let boxes: Vec<Vec<RotatedRect>> = lines
            .iter()
            .map(|r| {
                vec![RotatedRect::from_rect(RectF::from_tlhw(
                    r.y as f32 - LINE_MARGIN,
                    r.x as f32 - LINE_MARGIN,
                    r.h as f32 + 2.0 * LINE_MARGIN,
                    r.w as f32 + 2.0 * LINE_MARGIN,
                ))]
            })
            .collect();
        self.engine.recognize_text(&input, &boxes).ok()
    }

    /// ocrs's reading of each line's ink (black on white), stacked on one canvas and enlarged
    /// `upscale` times: `None` where it read nothing, or for every line if it couldn't run.
    pub fn read_stacked(&self, inks: &[&GrayImage], upscale: u32) -> Vec<Option<String>> {
        let width = inks.iter().map(|ink| ink.width()).max().unwrap_or(0) + 2 * PAD;
        let height = inks.iter().map(|ink| ink.height() + PAD).sum::<u32>() + PAD;
        let mut canvas = GrayImage::from_pixel(width, height, image::Luma([255]));
        let mut rects = Vec::with_capacity(inks.len());
        let mut y = PAD;
        for ink in inks {
            image::imageops::replace(&mut canvas, *ink, PAD as i64, y as i64);
            rects.push(Rect { x: PAD * upscale, y: y * upscale, w: ink.width() * upscale, h: ink.height() * upscale });
            y += ink.height() + PAD;
        }
        let canvas = image::imageops::resize(&canvas, width * upscale, height * upscale, image::imageops::FilterType::Triangle);
        let Some(lines) = self.read_lines(&canvas, &rects) else { return vec![None; inks.len()] };
        let text = |line: Option<&TextLine>| {
            let text = line?.to_string();
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then_some(text)
        };
        (0..inks.len()).map(|i| text(lines.get(i).and_then(|line| line.as_ref()))).collect()
    }

    fn read_prepared(&self, p: &Prepared) -> Option<TitleLines> {
        let out = self.read_lines(&p.canvas, &p.lines)?;
        let line = |i: usize| out.get(i).and_then(|l| l.as_ref());
        let street = line(0).map(|l| l.to_string().trim().to_string());
        let map = match (line(1), p.numeral) {
            (line, Some(numeral)) => Some(with_numeral(line, numeral)),
            (Some(line), None) => Some(line.to_string().trim().to_string()),
            (None, None) => None,
        };
        let nonempty = |s: Option<String>| s.filter(|s| !s.is_empty());
        Some(TitleLines { street: nonempty(street)?, map: nonempty(map)? })
    }
}

/// The map line as read, with whatever ocrs made of its last word (an unknown glyph
/// for "II", say) replaced by the numeral counted from the mask.
fn with_numeral(line: Option<&TextLine>, numeral: Numeral) -> String {
    let before: String = line
        .map(|l| l.chars())
        .unwrap_or_default()
        .iter()
        .filter(|c| (c.rect.left() + c.rect.right()) < 2 * numeral.x as i32)
        .map(|c| c.char)
        .collect();
    format!("{} {}", before.trim(), "I".repeat(numeral.strokes)).trim().to_string()
}
