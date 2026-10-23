//! Title recogniser (ADR 0002): ocrs, recognition model only. We supply the line boxes from
//! our own mask and line split, so ocrs' detection model is not shipped. The title is found
//! first, wherever it is in the capture (ADR 0021, `minimap_title`).

use crate::Frame;
use crate::minimap_title;
use crate::panel::{ScreenPoint, ScreenRect};
use crate::prep::{Numeral, PAD, Prepared, Rect};
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

/// An ink's key in what was read: its width and its pixels.
type InkKey = (u32, Vec<u8>);

fn ink_key(ink: &GrayImage) -> InkKey {
    (ink.width(), ink.as_raw().clone())
}

/// What inks (black on white) read as, remembered by the ink, up to `most` of them: past
/// that it forgets them all at once. A frame read again unchanged costs no recognition, the
/// costly part of reading it.
struct Remembered<V> {
    read: HashMap<InkKey, V>,
    most: usize,
}

impl<V> Remembered<V> {
    fn new(most: usize) -> Self {
        Self { read: HashMap::new(), most }
    }

    fn get(&self, key: &InkKey) -> Option<&V> {
        self.read.get(key)
    }

    /// Remember what inks `read` as, forgetting everything first if they don't all fit.
    fn remember(&mut self, read: impl ExactSizeIterator<Item = (InkKey, V)>) {
        if self.read.len() + read.len() > self.most {
            self.read.clear();
        }
        self.read.extend(read);
    }
}

/// What lines of ink read as, remembered by their ink ([`Remembered`]).
pub(crate) struct RememberedLines {
    read: Remembered<Option<String>>,
}

impl Default for RememberedLines {
    fn default() -> Self {
        Self { read: Remembered::new(REMEMBERED) }
    }
}

impl RememberedLines {
    /// What each of `inks` (black on white) reads as, enlarged `upscale` times for ocrs:
    /// those not read before, all at once.
    pub fn read(&mut self, recogniser: &Recogniser, inks: &[GrayImage], upscale: u32) -> Vec<Option<String>> {
        let keys: Vec<InkKey> = inks.iter().map(ink_key).collect();
        let unknown: Vec<usize> = (0..inks.len()).filter(|&i| self.read.get(&keys[i]).is_none()).collect();
        if !unknown.is_empty() {
            let read = recogniser.read_stacked(&unknown.iter().map(|&i| &inks[i]).collect::<Vec<_>>(), upscale);
            self.read.remember(unknown.iter().map(|&i| keys[i].clone()).zip(read));
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

/// A capture for the Minimap Title (ADR 0021): of the game's whole client area while the title
/// is looked for, or of where it was last found.
#[derive(Debug, Clone)]
pub struct TitleCapture {
    pub frame: Frame,
    /// Where the frame's top-left pixel is on screen.
    pub origin: ScreenPoint,
    /// Where our own windows were on screen as it was captured (the panel state's
    /// `our_windows`): the capture holds them, and no title is looked for in them.
    pub ours: Vec<ScreenRect>,
}

/// What one title capture reads as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleReading {
    /// `None` when the title is unreadable (covered, say), or not found.
    pub lines: Option<TitleLines>,
    /// Where on screen the title was found: the minimap window's top, as wide and as tall as
    /// either layout is for the longest names. `None`: no title found (a loading screen, the
    /// minimap closed).
    pub found: Option<ScreenRect>,
    /// When the frame was captured, as in [`Frame::at_ms`].
    pub at_ms: u64,
}

/// How many titles' readings [`TitleReader`] keeps before it forgets them all at once.
const REMEMBERED_TITLES: usize = 64;

/// Reads title captures with the bundled recogniser (ADR 0002). Reading takes tens of
/// milliseconds, so the shell keeps one on its own thread and hands the core each
/// [`TitleReading`] (`Minerva::on_title_read`). It remembers what each title's ink read as:
/// standing in one map, the title doesn't change, and costs no recognition.
pub struct TitleReader {
    recogniser: Recogniser,
    read: Remembered<Option<TitleLines>>,
}

impl TitleReader {
    pub fn bundled() -> Self {
        Self { recogniser: Recogniser::bundled(), read: Remembered::new(REMEMBERED_TITLES) }
    }

    pub fn read(&mut self, capture: &TitleCapture) -> TitleReading {
        let found = frame_to_rgb(&capture.frame).and_then(|mut img| {
            blank(&mut img, capture.origin, &capture.ours);
            minimap_title::find(&img)
        });
        let Some(found) = found else { return TitleReading { lines: None, found: None, at_ms: capture.frame.at_ms } };
        let key = ink_key(&found.prepared.canvas);
        let lines = match self.read.get(&key) {
            Some(lines) => lines.clone(),
            None => {
                let lines = self.recogniser.read_prepared(&found.prepared);
                self.read.remember([(key, lines.clone())].into_iter());
                lines
            }
        };
        let (x, y, width, height) = found.area;
        let found = ScreenRect { x: capture.origin.x + x, y: capture.origin.y + y, width, height };
        TitleReading { lines, found: Some(found), at_ms: capture.frame.at_ms }
    }
}

/// `img`, captured with its top-left pixel at `origin` on screen, painted black wherever one of
/// `ours` (our windows, on screen) covers it.
fn blank(img: &mut RgbImage, origin: ScreenPoint, ours: &[ScreenRect]) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    for r in ours {
        let (x0, y0) = ((r.x as i64 - origin.x as i64).clamp(0, w), (r.y as i64 - origin.y as i64).clamp(0, h));
        let x1 = (r.x as i64 + r.width as i64 - origin.x as i64).clamp(0, w);
        let y1 = (r.y as i64 + r.height as i64 - origin.y as i64).clamp(0, h);
        for y in y0..y1 {
            for x in x0..x1 {
                img.put_pixel(x as u32, y as u32, image::Rgb([0, 0, 0]));
            }
        }
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
    pub(crate) fn read_stacked(&self, inks: &[&GrayImage], upscale: u32) -> Vec<Option<String>> {
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

    /// Read the street line and the map line, of either layout: the one line "Street : Map"
    /// comes split at its colon. `None` means unreadable.
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
