//! Shared test inputs: minimap samples turned into frames, and a fake data source serving
//! made-up worlds in OSMS's shape (ADR 0008) and whatever item searches a test gives it.
#![allow(dead_code)]

pub mod items;
pub mod osms;
pub mod player;
pub mod world;

use minerva_core::osms::{OsmsFiles, SavedFiles};
use minerva_core::osms_items::{ItemFiles, SavedItemFiles};
use minerva_core::{
    MINERVA_VERSION, Minerva, Fetched, Frame, GameWindow, ItemHit, MapDataSource, PanelState,
    ScreenRect, Settings, SourceError, TitleReading,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

pub fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/minimap")
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn load(name: &str) -> image::RgbaImage {
    image::open(samples_dir().join(format!("{name}.png")))
        .unwrap_or_else(|e| panic!("sample {name}: {e}"))
        .to_rgba8()
}

pub fn frame(img: &image::RgbaImage, at_ms: u64) -> Frame {
    Frame { width: img.width(), height: img.height(), rgba: img.as_raw().clone(), at_ms }
}

/// Sizes no sample is at, to scale samples to (ADR 0020): from about the smallest the game
/// draws its UI at (×0.5, Default Ratio at 600 rows) to past the largest (×1.8, at 2160).
pub const OTHER_UI_SIZES: [f32; 4] = [0.55, 0.8, 1.45, 1.9];

/// `img` scaled by `size`, smoothly, as the game draws its UI at another UI Size.
pub fn scaled(img: &image::RgbaImage, size: f32) -> image::RgbaImage {
    let at = |px: u32| (px as f32 * size).round() as u32;
    image::imageops::resize(img, at(img.width()), at(img.height()), image::imageops::FilterType::Triangle)
}

/// The game's pointer drawn over `img`, `rows` tall, its tip at `(x, y)`: white with a
/// black outline, cut off at the image's edges.
pub fn pointer(img: &mut image::RgbaImage, (x, y): (u32, u32), rows: u32) {
    for dy in 0..rows {
        for dx in 0..=dy / 2 {
            let edge = dx == 0 || dx == dy / 2 || dy == rows - 1;
            if x + dx < img.width() && y + dy < img.height() {
                img.put_pixel(x + dx, y + dy, image::Rgba(if edge { [0, 0, 0, 255] } else { [255, 255, 255, 255] }));
            }
        }
    }
}

/// A loading screen: where the title was found is plain black.
pub fn black_frame(at_ms: u64) -> Frame {
    frame(&image::RgbaImage::from_pixel(560, 96, image::Rgba([0, 0, 0, 255])), at_ms)
}

/// `--window` samples are whole game-window screenshots including the Windows title
/// bar (25–28 px, hand-cropped). The client area starts below it.
const TITLE_BAR_PX: u32 = 27;

/// The game's client area in a window screenshot: everything below the title bar.
pub fn client_of(window: &image::RgbaImage) -> image::RgbaImage {
    image::imageops::crop_imm(window, 0, TITLE_BAR_PX, window.width(), window.height() - TITLE_BAR_PX)
        .to_image()
}

/// Capture `region` (physical screen px) from a game client area shown on screen at
/// `game`, the way the shell captures the panel state's `title_capture`.
pub fn capture(client: &image::RgbaImage, game: ScreenRect, region: ScreenRect, at_ms: u64) -> Frame {
    let (x, y) = ((region.x - game.x) as u32, (region.y - game.y) as u32);
    let crop = image::imageops::crop_imm(client, x, y, region.width, region.height).to_image();
    assert_eq!(crop.dimensions(), (region.width, region.height), "region inside the client area");
    frame(&crop, at_ms)
}

/// The frame the shell would capture for a sample: `--window` samples' client area, as the
/// shell captures it while it looks for the title (ADR 0021); other samples (a minimap crop,
/// a capture of where the title was found) are fed whole.
pub fn sample_frame(name: &str, at_ms: u64) -> Frame {
    let img = load(name);
    if name.ends_with("--window") { frame(&client_of(&img), at_ms) } else { frame(&img, at_ms) }
}

/// Two confident reads of a sample: the panel switches to its map.
pub fn arrive(minerva: &mut Minerva, sample: &str, at_ms: u64) -> PanelState {
    minerva.on_frame(sample_frame(sample, at_ms));
    minerva.on_frame(sample_frame(sample, at_ms + 333)).clone()
}

/// A core with EXP tracking on: both regions kept, on a 1920x1200 game window.
pub fn tracking() -> Minerva {
    tracking_with(FakeSource::osms())
}

/// As [`tracking`], with OSMS's file set from `source`.
pub fn tracking_with(source: FakeSource) -> Minerva {
    let mut minerva = Minerva::new(source);
    minerva.restore(Settings {
        setup_done: true,
        ..Settings::default()
    });
    let screen = ScreenRect { x: 0, y: 0, width: 1920, height: 1200 };
    minerva.on_game_window(Some(GameWindow { client: screen, frame: screen, screens: vec![screen] }));
    minerva
}

/// Sunset Sky's and the Construction Site's map IDs: both have a Map Summary in the
/// made-up game world ([`osms::game`]).
pub const SUNSET_SKY_ID: &str = "010003070";
pub const CONSTRUCTION_SITE_ID: &str = "010003080";

/// The made-up game world, with the Construction Site and Sunset Sky sharing the Kerning
/// City sample's title: the Construction Site, listed first, shows first.
pub fn sharing_a_title() -> FakeSource {
    FakeSource::with_world(osms::game().titled("Victoria Road", "Kerning City", &[osms::CONSTRUCTION_SITE, osms::SUNSET_SKY]))
}

/// The Northern Top of the Construction Site's map ID: it has a Map Summary.
pub const NORTHERN_TOP_ID: &str = "010003081";

/// As [`sharing_a_title`], with the Northern Top of the Construction Site sharing the
/// title too: three Candidates in map list order, the Northern Top first (shown first),
/// then the Construction Site, then Sunset Sky.
pub fn three_sharing_a_title() -> FakeSource {
    let ids = [osms::NORTHERN_TOP, osms::CONSTRUCTION_SITE, osms::SUNSET_SKY];
    FakeSource::with_world(osms::game().titled("Victoria Road", "Kerning City", &ids))
}

type PictureAnswer = Result<String, SourceError>;
type SearchAnswer = Result<Vec<ItemHit>, SourceError>;
pub type OsmsAnswer = Poll<Result<OsmsFiles, SourceError>>;
pub type ItemSetAnswer = Poll<Result<ItemFiles, SourceError>>;

/// A data source that answers from memory and counts what it is asked.
pub struct FakeSource {
    /// Answers for successive fetches of OSMS's file set; the last one repeats.
    osms_answers: Vec<OsmsAnswer>,
    /// How many times OSMS's file set was fetched.
    pub osms_requests: Arc<AtomicU32>,
    /// The set an earlier launch saved; `None`: none was.
    saved_set: Option<SavedFiles>,
    /// MeowDB is unreachable and item searches are answered with copies saved at this time
    /// (ms since the Unix epoch); `None`: they are fresh.
    saved_at: Option<u64>,
    /// Answers a picture request for a path under OSMS's data folder. Defaults to
    /// `"picture:<path>"`.
    pictures: Box<dyn FnMut(&str) -> PictureAnswer + Send>,
    /// The path of every picture fetch started, in order.
    pub picture_requests: Arc<Mutex<Vec<String>>>,
    /// How many polls a picture takes: 0 when it's saved (answered at once), otherwise it
    /// arrives on the `n`th poll after its fetch started, as from the network.
    picture_polls: usize,
    /// The picture fetch in flight, its answer, and how many more polls it takes.
    picture_in_flight: Option<(String, PictureAnswer, usize)>,
    /// Answers an item search for a name. Defaults to an answer with no items.
    item_searches: Box<dyn FnMut(&str) -> SearchAnswer + Send>,
    /// The name of every item search started, in order.
    pub item_requests: Arc<Mutex<Vec<String>>>,
    /// How many polls an item search takes, as `picture_polls` for pictures.
    item_polls: usize,
    item_in_flight: Option<(String, SearchAnswer, usize)>,
    /// Answers for successive fetches of OSMS's item files; the last one repeats.
    item_set_answers: Vec<ItemSetAnswer>,
    /// How many times OSMS's item files were fetched.
    pub item_set_requests: Arc<AtomicU32>,
    /// The item files an earlier launch saved; `None`: none were.
    saved_item_set: Option<SavedItemFiles>,
}

impl FakeSource {
    /// OSMS's file set answers these, one ask after another; the last one repeats.
    pub fn answering_osms(answers: Vec<OsmsAnswer>) -> Self {
        Self {
            osms_answers: answers,
            osms_requests: Arc::default(),
            saved_set: None,
            saved_at: None,
            pictures: Box::new(|path| Ok(format!("picture:{path}"))),
            picture_requests: Arc::default(),
            picture_polls: 0,
            picture_in_flight: None,
            item_searches: Box::new(|_| Ok(Vec::new())),
            item_requests: Arc::default(),
            item_polls: 0,
            item_in_flight: None,
            item_set_answers: vec![Poll::Ready(Err(unreachable()))],
            item_set_requests: Arc::default(),
            saved_item_set: None,
        }
    }

    /// OSMS serves this made-up world.
    pub fn with_world(world: osms::World) -> Self {
        Self::answering_osms(vec![fresh(world)])
    }

    /// OSMS serves the game's map list, with made-up figures on the maps the minimap
    /// samples show ([`osms::game`]).
    pub fn osms() -> Self {
        Self::with_world(osms::game())
    }

    /// OSMS can't be reached, and no set is saved (unless one is added with
    /// [`FakeSource::with_saved_set`]).
    pub fn osms_offline() -> Self {
        Self::answering_osms(vec![Poll::Ready(Err(unreachable()))])
    }

    /// A set of `world`'s files was saved at `saved_at_unix_ms` (ms since the Unix epoch),
    /// at an earlier launch of this version.
    pub fn with_saved_set(self, world: osms::World, saved_at_unix_ms: u64) -> Self {
        self.with_saved_files(world.files(), saved_at_unix_ms)
    }

    /// These files were saved as OSMS's set at `saved_at_unix_ms`, at an earlier launch of
    /// this version.
    pub fn with_saved_files(self, files: OsmsFiles, saved_at_unix_ms: u64) -> Self {
        let saved_by = Some(MINERVA_VERSION.to_string());
        Self { saved_set: Some(SavedFiles { files, saved_at_unix_ms, saved_by }), ..self }
    }

    /// The saved set and item files were saved by `version` (`None`: by a Minerva from
    /// before saved files said which version saved them), not this one.
    pub fn saved_by(mut self, version: Option<&str>) -> Self {
        let by = version.map(String::from);
        self.saved_set.iter_mut().for_each(|saved| saved.saved_by.clone_from(&by));
        self.saved_item_set.iter_mut().for_each(|saved| saved.saved_by.clone_from(&by));
        self
    }

    /// MeowDB can't be reached, and every item search is answered with a copy saved at
    /// `saved_at_unix_ms`.
    pub fn serving_saved_copies(self, saved_at_unix_ms: u64) -> Self {
        Self { saved_at: Some(saved_at_unix_ms), ..self }
    }

    fn fetched<T>(&self, answer: Poll<Result<T, SourceError>>) -> Poll<Result<Fetched<T>, SourceError>> {
        answer.map(|result| {
            result.map(|data| match self.saved_at {
                Some(saved_at_unix_ms) => Fetched::SavedCopy { data, saved_at_unix_ms },
                None => Fetched::Fresh(data),
            })
        })
    }

    /// Answer picture requests with `pictures`, by path.
    pub fn with_pictures(mut self, pictures: impl FnMut(&str) -> PictureAnswer + Send + 'static) -> Self {
        self.pictures = Box::new(pictures);
        self
    }

    /// Pictures aren't saved yet, and come over the network: each arrives on the
    /// `polls`th poll after its fetch started.
    pub fn pictures_taking(self, polls: usize) -> Self {
        Self { picture_polls: polls, ..self }
    }

    /// Answer item searches with `searches`, by name.
    pub fn with_item_searches(mut self, searches: impl FnMut(&str) -> SearchAnswer + Send + 'static) -> Self {
        self.item_searches = Box::new(searches);
        self
    }

    /// Item searches come over the network, not from the cache: each arrives on the
    /// `polls`th poll after it started.
    pub fn items_taking(self, polls: usize) -> Self {
        Self { item_polls: polls, ..self }
    }

    /// OSMS serves this made-up item data.
    pub fn with_items(self, items: items::ItemWorld) -> Self {
        self.answering_item_set(vec![Poll::Ready(Ok(items.files()))])
    }

    /// OSMS's item files answer these, one ask after another; the last one repeats.
    pub fn answering_item_set(self, answers: Vec<ItemSetAnswer>) -> Self {
        Self { item_set_answers: answers, ..self }
    }

    /// These item files were saved at `saved_at_unix_ms`, at an earlier launch of this
    /// version.
    pub fn with_saved_items(self, items: items::ItemWorld, saved_at_unix_ms: u64) -> Self {
        let saved_by = Some(MINERVA_VERSION.to_string());
        Self { saved_item_set: Some(SavedItemFiles { files: items.files(), saved_at_unix_ms, saved_by }), ..self }
    }
}

fn unreachable() -> SourceError {
    SourceError::Unreachable("offline".into())
}

/// How many of `requests` (a [`FakeSource`]'s log) were for `id`.
pub fn requests_for(requests: &Mutex<Vec<String>>, id: &str) -> usize {
    requests.lock().unwrap().iter().filter(|r| *r == id).count()
}

/// One fetch at a time: the answer for `id` if its fetch is the one in flight and it has
/// arrived, otherwise a new fetch, logged in `requests`, answered at once or (with
/// `polls` above 0) on the `polls`th poll after.
fn one_at_a_time<T>(
    id: &str,
    in_flight: &mut Option<(String, Result<T, SourceError>, usize)>,
    requests: &Mutex<Vec<String>>,
    polls: usize,
    answer: impl FnOnce(&str) -> Result<T, SourceError>,
) -> Poll<Result<T, SourceError>> {
    if let Some((asked, arrived, left)) = in_flight.take()
        && asked == id
    {
        if left <= 1 {
            return Poll::Ready(arrived);
        }
        *in_flight = Some((asked, arrived, left - 1));
        return Poll::Pending;
    }
    requests.lock().unwrap().push(id.to_string());
    let answer = answer(id);
    if polls > 0 {
        *in_flight = Some((id.to_string(), answer, polls));
        return Poll::Pending;
    }
    Poll::Ready(answer)
}

/// OSMS's file set fetched afresh: `world`'s files.
pub fn fresh(world: osms::World) -> OsmsAnswer {
    Poll::Ready(Ok(world.files()))
}

/// OSMS's file set fetched afresh: these files, as they are.
pub fn fresh_files(files: OsmsFiles) -> OsmsAnswer {
    Poll::Ready(Ok(files))
}

impl MapDataSource for FakeSource {
    fn saved_osms_files(&mut self) -> Option<SavedFiles> {
        self.saved_set.clone()
    }

    fn fetch_osms_files(&mut self) -> Poll<Result<OsmsFiles, SourceError>> {
        let n = self.osms_requests.fetch_add(1, Ordering::SeqCst) as usize;
        self.osms_answers[n.min(self.osms_answers.len() - 1)].clone()
    }

    fn picture(&mut self, path: &str) -> Poll<Result<String, SourceError>> {
        one_at_a_time(path, &mut self.picture_in_flight, &self.picture_requests, self.picture_polls, &mut self.pictures)
    }

    fn item_search(&mut self, name: &str) -> Poll<Result<Fetched<Vec<ItemHit>>, SourceError>> {
        let answer =
            one_at_a_time(name, &mut self.item_in_flight, &self.item_requests, self.item_polls, &mut self.item_searches);
        self.fetched(answer)
    }

    fn saved_item_files(&mut self) -> Option<SavedItemFiles> {
        self.saved_item_set.clone()
    }

    fn fetch_item_files(&mut self) -> Poll<Result<ItemFiles, SourceError>> {
        let n = self.item_set_requests.fetch_add(1, Ordering::SeqCst) as usize;
        self.item_set_answers[n.min(self.item_set_answers.len() - 1)].clone()
    }
}

/// Where a Minimap Title is found on the game's client area `client` in these tests: at
/// client (70,40), 620x48 (ADR 0021), where the Title Region the player marked there once was.
pub fn title_on(client: ScreenRect) -> ScreenRect {
    ScreenRect { x: client.x + 70, y: client.y + 40, width: 620.min(client.width - 70), height: 48 }
}

/// A Minimap Title found at `area` on screen, but unreadable: the map shown stays as it is, and
/// our windows keep clear of it.
pub fn found_title(minerva: &mut Minerva, area: ScreenRect, at_ms: u64) {
    minerva.on_reading(TitleReading { lines: None, found: Some(area), at_ms });
}

/// Where readings made up in these tests found the Minimap Title, on a game window at the
/// screen's top-left: the open minimap's top, at the samples' UI Size.
pub const TITLE_FOUND: ScreenRect = ScreenRect { x: 0, y: 4, width: 720, height: 110 };
