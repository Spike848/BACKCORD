//! Stream Proof: while on, messages hide on this screen until one is
//! shown with a click (Local Blur), captures leave the window out on
//! Windows (Blackout Stream), and it comes on by itself while this account
//! goes live (Auto Stream Proof). A button on the chat bar or the header
//! turns it on and off; Extra Features say which and what it does.

use iced::Task;

use crate::app::{App, Message};

#[derive(Clone, Debug)]
pub enum Event {
    Toggle,
}

/// Whether it's on, and whether this account was going live as the core
/// last said, to follow its start and end.
#[derive(Default)]
pub struct StreamProof {
    pub active: bool,
    streaming: bool,
}

/// Where its button is, as Extra Features' choice lists them.
pub const LOCATIONS: [&str; 3] = ["Chat bar", "Header bar", "Disabled"];

/// What `revealed` holds for a message shown though Stream Proof hides it.
pub fn reveal_key(item: &str) -> String {
    format!("{item}/stream")
}

impl App {
    pub fn stream_proof_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Toggle => {
                let on = !self.stream_proof.active;
                self.set_stream_proof(on)
            }
        }
    }

    /// Whether messages hide now: it's on, and Local Blur with it.
    pub fn stream_hidden(&self) -> bool {
        let prefs = &self.layout.prefs;
        self.stream_proof.active
            && prefs.stream_proof.unwrap_or(true)
            && prefs.stream_proof_blur.unwrap_or(true)
    }

    fn set_stream_proof(&mut self, on: bool) -> Task<Message> {
        if self.stream_proof.active == on {
            return Task::none();
        }
        self.stream_proof.active = on;
        // Shown messages hide again next time.
        if !on {
            self.timeline
                .revealed
                .retain(|key| !key.ends_with("/stream"));
        }
        self.refresh_stream_proof()
    }

    /// Applies what Extra Features say now: its rows, and the window to
    /// captures; off while the feature is.
    pub fn refresh_stream_proof(&mut self) -> Task<Message> {
        let prefs = &self.layout.prefs;
        if !prefs.stream_proof.unwrap_or(true) && self.stream_proof.active {
            return self.set_stream_proof(false);
        }
        let hidden = self.stream_proof.active && prefs.stream_proof_blackout.unwrap_or(false);
        self.measure(&crate::app::Messages::default(), true);
        self.dirty = true;
        let capture = match self.shell.window {
            Some(id) => iced::window::run(id, move |window| {
                crate::desktop::desktop().hide_from_capture(window, hidden);
            })
            .discard(),
            None => Task::none(),
        };
        Task::batch([capture, self.ask()])
    }

    /// Auto Stream Proof: on as this account goes live, off as it stops.
    pub fn follow_stream(&mut self) -> Task<Message> {
        let streaming = self.client.state().voice.sharing.is_some();
        if streaming == self.stream_proof.streaming {
            return Task::none();
        }
        self.stream_proof.streaming = streaming;
        let prefs = &self.layout.prefs;
        if !prefs.stream_proof.unwrap_or(true) || !prefs.stream_proof_auto.unwrap_or(true) {
            return Task::none();
        }
        self.set_stream_proof(streaming)
    }
}

/// How far a blurred piece is drawn off its place, at most, in pixels.
const BLUR_REACH: f32 = 10.0;

/// The offsets a blurred piece is drawn at: three rings round its place,
/// eight each and turned from one another, so its strokes run into one
/// smear wider than a letter.
fn blur_offsets() -> impl Iterator<Item = iced::Vector> {
    (0..24).map(|step| {
        let ring = (step / 8) as f32;
        let radius = BLUR_REACH * (ring + 1.0) / 3.0;
        let angle = (step % 8) as f32 * std::f32::consts::FRAC_PI_4
            + ring * std::f32::consts::FRAC_PI_4 / 3.0;
        iced::Vector::new(radius * angle.cos(), radius * angle.sin())
    })
}

/// `content` blurred: laid out once and drawn at each of [`blur_offsets`],
/// so what's drawn in a faint ink runs into a smear nobody reads.
pub fn blurred<'a>(content: impl Into<iced::Element<'a, Message>>) -> iced::Element<'a, Message> {
    crate::widgets::Wrap::new(content, Blur).into()
}

struct Blur;

impl crate::widgets::Hooks<Message> for Blur {
    type State = crate::widgets::Stateless;

    fn draw(
        &self,
        _state: &crate::widgets::Stateless,
        renderer: &mut crate::widgets::Renderer,
        _at: crate::widgets::At,
        child: &mut dyn FnMut(&mut crate::widgets::Renderer),
    ) {
        use iced::advanced::Renderer as _;
        for offset in blur_offsets() {
            renderer.with_translation(offset, |renderer| child(renderer));
        }
    }
}
