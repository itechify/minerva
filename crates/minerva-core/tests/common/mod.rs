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
    ExpBar, ExpBarPlace, ExpBarReading, Fetched, Frame, GameWindow, ItemHit, ItemTooltip, ItemTooltipReading, MINERVA_VERSION,
    MapDataSource, Minerva, OwlLine, OwlMood, PanelState, Request, ScreenPoint, ScreenRect, Settings, Size, SourceError,
    TitleCapture, TitleLines, TitleReading,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

/// The folder of one kind of sample under `samples/`: "minimap", "exp-bar", "status-bar",
/// "item-tooltip" or "player-shop".
pub fn samples_in(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples").join(kind)
}

pub fn samples_dir() -> PathBuf {
    samples_in("minimap")
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
    minerva.restore(set_up());
    minerva.on_game_window(Some(window_at_origin(1920, 1200)));
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

/// OSMS or MeowDB can't be reached.
pub fn unreachable() -> SourceError {
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

// Time, in the ms the core counts in.
pub const S: u64 = 1000;
pub const MIN: u64 = 60 * S;
pub const HOUR: u64 = 60 * MIN;
pub const DAY: u64 = 24 * HOUR;

/// The wall clock at capture time 0 for the Map Leaderboards' tests: 9 Oct 2026, 12:00 UTC.
pub const WALL: u64 = 1_791_547_200_000;

/// When a saved copy was made: 2026-10-01 00:00 UTC, in ms since the Unix epoch.
pub const SAVED_AT: u64 = 1_790_812_800_000;

// The window samples the tests arrive on, named as `sample_frame` loads them.
pub const SUNSET_SAMPLE: &str = "victoria-road__sunset-sky--window";
pub const KERNING_SAMPLE: &str = "victoria-road__kerning-city--window";
pub const CONSTRUCTION_SAMPLE: &str = "victoria-road__construction-site-north-of-kerning-city--window";

/// A made-up map whose data the tests give, on a street of its own.
pub const FIELD: u64 = 100_000_000;

/// `img`'s pixels as a frame at 1,000 ms, their colours times `brightness` (as the game
/// draws a dimmed screen).
pub fn dimmed(img: &image::RgbaImage, brightness: f32) -> Frame {
    let mut rgba = img.as_raw().clone();
    for (i, channel) in rgba.iter_mut().enumerate() {
        if i % 4 != 3 {
            *channel = (*channel as f32 * brightness).round() as u8;
        }
    }
    Frame { width: img.width(), height: img.height(), rgba, at_ms: 1_000 }
}

/// The game's client area (physical px) in the tests' usual spot: 1920x1200 at (812,110).
pub const GAME: ScreenRect = ScreenRect { x: 812, y: 110, width: 1920, height: 1200 };
/// The tests' usual screen: one 3440x1440 monitor.
pub const SCREEN: ScreenRect = ScreenRect { x: 0, y: 0, width: 3440, height: 1440 };

/// The game window's frame as the shell measures it around `client`: 1 px each side and 31
/// above.
pub fn framed(client: ScreenRect) -> ScreenRect {
    ScreenRect { x: client.x - 1, y: client.y - 31, width: client.width + 2, height: client.height + 32 }
}

/// The game window as the shell reports it, with this client area, framed as measured, on
/// [`SCREEN`].
pub fn window_on_wide_screen(client: ScreenRect) -> GameWindow {
    GameWindow { client, frame: framed(client), screens: vec![SCREEN] }
}

/// The game window with this client area, framed as measured, on [`SCREEN`], as the shell
/// reports it found.
pub fn game(client: ScreenRect) -> Option<GameWindow> {
    Some(window_on_wide_screen(client))
}

pub fn rect(x: i32, y: i32, width: u32, height: u32) -> ScreenRect {
    ScreenRect { x, y, width, height }
}

/// The game window at the screen's origin, `width` by `height`, with no frame around it, on
/// a screen of its size.
pub fn window_at_origin(width: u32, height: u32) -> GameWindow {
    let screen = ScreenRect { x: 0, y: 0, width, height };
    GameWindow { client: screen, frame: screen, screens: vec![screen] }
}

/// The client area of a window sample shown at (812,110).
pub fn game_of(sample: &str) -> ScreenRect {
    let client = client_of(&load(sample));
    ScreenRect { x: 812, y: 110, width: client.width(), height: client.height() }
}

/// The capture the core asks for, taken of `sample`'s client area shown at [`game_of`] it.
pub fn title_capture(minerva: &Minerva, sample: &str, at_ms: u64) -> TitleCapture {
    let client = client_of(&load(sample));
    let region = minerva.panel_state().title_capture.expect("a title capture with the game in front");
    let ours = minerva.panel_state().our_windows.clone();
    TitleCapture { frame: capture(&client, game_of(sample), region, at_ms), origin: ScreenPoint { x: region.x, y: region.y }, ours }
}

/// Settings with setup finished.
pub fn set_up() -> Settings {
    Settings { setup_done: true, ..Settings::default() }
}

/// What the shell does on restart: the settings saved as JSON, then loaded back.
pub fn settings_after_restart(minerva: &Minerva) -> Settings {
    let json = serde_json::to_string(&minerva.settings()).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// The Minimap Title read as `street` and `map` at `at_ms`, found where readings made up in
/// these tests find it ([`TITLE_FOUND`]).
pub fn read_as(street: &str, map: &str, at_ms: u64) -> TitleReading {
    TitleReading { lines: Some(TitleLines { street: street.into(), map: map.into() }), found: Some(TITLE_FOUND), at_ms }
}

/// Two confident readings of a title the minimap samples don't show, over `world`.
pub fn on_title(world: osms::World, street: &str, name: &str) -> PanelState {
    let mut minerva = Minerva::new(FakeSource::with_world(world));
    minerva.on_reading(read_as(street, name, 0));
    minerva.on_reading(read_as(street, name, 333)).clone()
}

/// The percentage the game shows for `exp` on a level of `total` EXP, rounded to hundredths.
pub fn percent(exp: u64, total: u64) -> u32 {
    ((exp * 20_000 + total) / (2 * total)) as u32
}

/// The EXP bar read as `exp [percent_hundredths / 100 %]` at `at_ms`.
pub fn read(minerva: &mut Minerva, at_ms: u64, exp: u64, percent_hundredths: u32) -> PanelState {
    let bar = Some(ExpBar { exp, percent_hundredths });
    minerva.on_exp_bar_reading(ExpBarReading { bar, at_ms, place: None }).clone()
}

/// A frame of the EXP bar at `at_ms` whose figures don't read whole.
pub fn unread(minerva: &mut Minerva, at_ms: u64) -> PanelState {
    minerva.on_exp_bar_reading(ExpBarReading { bar: None, at_ms, place: None }).clone()
}

/// The EXP figures read as unreadable where they were found, at `area`.
pub fn found_exp_figures(minerva: &mut Minerva, area: ScreenRect) {
    minerva.on_exp_bar_reading(ExpBarReading { bar: None, at_ms: 0, place: Some(ExpBarPlace { area, remembered: false }) });
}

/// Whether two rectangles on screen overlap.
pub fn overlaps(a: ScreenRect, b: ScreenRect) -> bool {
    a.x < b.x + b.width as i32 && b.x < a.x + a.width as i32 && a.y < b.y + b.height as i32 && b.y < a.y + a.height as i32
}

/// The Personal Best the Best row shows.
pub fn best_row(state: &PanelState) -> Option<u64> {
    state.exp_tracking.personal_best
}

/// The shown map's ID.
pub fn shown_id(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.id.as_str())
}

/// The shown map's name.
pub fn shown_map_name(state: &PanelState) -> Option<&str> {
    state.map.as_ref().map(|m| m.map_name.as_str())
}

/// Where the pointer is when the player presses an Item Tooltip hotkey.
pub const POINTER: ScreenPoint = ScreenPoint { x: 1000, y: 580 };
/// Where the name line of the Item Tooltip read is on screen.
pub const NAME: ScreenRect = ScreenRect { x: 1000, y: 600, width: 200, height: 19 };

/// The Item Tooltip at [`POINTER`] read as `name`, its name line at [`NAME`].
pub fn found(name: &str, at_ms: u64) -> Option<ItemTooltipReading> {
    Some(ItemTooltipReading { pointer: POINTER, tooltip: ItemTooltip::Found { area: NAME, name: Some(name.into()) }, at_ms })
}

/// Everything the shell has been asked to do since it last asked.
pub fn requests(minerva: &mut Minerva) -> Vec<Request> {
    std::iter::from_fn(|| minerva.take_request()).collect()
}

/// OSMS's file set can't be reached.
pub fn osms_unreachable() -> OsmsAnswer {
    Poll::Ready(Err(unreachable()))
}

/// A set that doesn't read.
pub fn osms_changed_format() -> OsmsAnswer {
    fresh_files(OsmsFiles { maps: "<html>".into(), ..osms::game().files() })
}

/// OSMS's item files can't be reached.
pub fn items_unreachable() -> ItemSetAnswer {
    Poll::Ready(Err(unreachable()))
}

/// The Owl Bubble's size (ADR 0025).
pub const OWL: Size = Size { width: 420, height: 56 };

/// Where the Owl Bubble shows, if it does.
pub fn owl_at(state: &PanelState) -> Option<ScreenRect> {
    state.owl_bubble.as_ref().map(|owl| owl.placement)
}

/// What the owl says, if anything.
pub fn owl_line(state: &PanelState) -> Option<OwlLine> {
    state.owl_bubble.as_ref().map(|owl| owl.line.clone())
}

/// How the owl says it, if it says anything.
pub fn owl_mood(state: &PanelState) -> Option<OwlMood> {
    state.owl_bubble.as_ref().map(|owl| owl.mood)
}

/// The Owl Bubble shows, and takes clicks.
pub fn owl_clickable(state: &PanelState) -> bool {
    state.owl_bubble.as_ref().is_some_and(|owl| owl.clickable)
}

/// How long the owl's line lasts, if the owl says anything: `None` inside while it stays.
pub fn owl_lasts_ms(state: &PanelState) -> Option<Option<u64>> {
    state.owl_bubble.as_ref().map(|owl| owl.lasts_ms)
}

/// Where made-up Pickup Log readings find the log.
pub const LOG: ScreenRect = ScreenRect { x: 1500, y: 800, width: 408, height: 160 };

/// A Pickup Log reading, as the capture thread hands it to the core.
pub fn pickup_reading(at_ms: u64, seen: minerva_core::PickupLogSeen, lines: Vec<minerva_core::PickupLine>) -> minerva_core::PickupLogReading {
    minerva_core::PickupLogReading { at_ms, seen, rows: vec![], shift: 0, arrived: lines, chose: false, guessed: false, overflowed: false }
}

/// A clean reading of lines that arrived at `at_ms`.
pub fn clean_pickup_reading(at_ms: u64, lines: Vec<minerva_core::PickupLine>) -> minerva_core::PickupLogReading {
    pickup_reading(at_ms, minerva_core::PickupLogSeen::Clear { area: LOG }, lines)
}

pub fn picked_up(minerva: &mut Minerva, at_ms: u64, lines: Vec<minerva_core::PickupLine>) -> PanelState {
    minerva.on_pickup_log_reading(clean_pickup_reading(at_ms, lines)).clone()
}

/// Start the usual made-up Session by changing the EXP bar from 160 to 266.
pub fn start_session(minerva: &mut Minerva, baseline_ms: u64, started_ms: u64) -> PanelState {
    read(minerva, baseline_ms, 160, 381);
    read(minerva, started_ms, 266, 633)
}

/// The usual Magician Status Bar, with a name if it read.
pub fn status_bar_reading(name: Option<&str>, at_ms: u64) -> minerva_core::StatusBarReading {
    let area = ScreenRect { x: 334, y: 1145, width: 331, height: 55 };
    minerva_core::StatusBarReading {
        bar: Some(minerva_core::StatusBar { area, level: Some(13), job: Some("Magician".into()), name: name.map(String::from), exp_figures: None }),
        at_ms, near_miss: None,
    }
}

/// Timestamped PNG frames, oldest first; truth files are left out.
pub fn pngs_in(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut frames: Vec<_> = std::fs::read_dir(path).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    frames.sort();
    frames
}
