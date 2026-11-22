//! How long the window's work takes, written to the log every
//! [`EVERY`] it works: redraws a second, and building the view, laying it
//! out, stepping its animations and drawing it, on average and at worst,
//! with the moving pictures' frames that weren't decoded in time. For
//! finding what makes a frame late; it changes nothing on screen.

use std::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};

use iced::{Element, advanced::Shell};

use crate::widgets::{At, Hooks, Inner, Renderer, Stateless, Wrap};

/// How often a summary is written while the window works.
const EVERY: Duration = Duration::from_secs(2);

/// Frames of a moving picture asked for before its thread had them.
pub static NOT_READY: AtomicU32 = AtomicU32::new(0);

/// One kind of work's count, total and worst.
#[derive(Default, Clone, Copy)]
struct Phase {
    count: u32,
    total: Duration,
    worst: Duration,
}

impl Phase {
    fn add(&mut self, took: Duration) {
        self.count += 1;
        self.total += took;
        self.worst = self.worst.max(took);
    }

    fn said(&self) -> String {
        if self.count == 0 {
            return "-".into();
        }
        format!(
            "{}x avg {:.1}ms max {:.1}ms",
            self.count,
            self.total.as_secs_f64() * 1000.0 / f64::from(self.count),
            self.worst.as_secs_f64() * 1000.0
        )
    }
}

struct Stats {
    since: Instant,
    view: Phase,
    layout: Phase,
    update: Phase,
    draw: Phase,
}

thread_local! {
    static STATS: RefCell<Stats> = RefCell::new(Stats {
        since: Instant::now(),
        view: Phase::default(),
        layout: Phase::default(),
        update: Phase::default(),
        draw: Phase::default(),
    });
}

fn note(phase: impl FnOnce(&mut Stats) -> &mut Phase, took: Duration) {
    STATS.with(|stats| {
        let mut stats = stats.borrow_mut();
        phase(&mut stats).add(took);
        let elapsed = stats.since.elapsed();
        if elapsed < EVERY {
            return;
        }
        let seconds = elapsed.as_secs_f64();
        tracing::debug!(
            "perf: {:.0} redraws/s | view {} | layout {} | update {} | draw {} | frames late {}",
            f64::from(stats.draw.count) / seconds,
            stats.view.said(),
            stats.layout.said(),
            stats.update.said(),
            stats.draw.said(),
            NOT_READY.swap(0, Ordering::Relaxed),
        );
        *stats = Stats {
            since: Instant::now(),
            view: Phase::default(),
            layout: Phase::default(),
            update: Phase::default(),
            draw: Phase::default(),
        };
    });
}

/// Building the view took `took`.
pub fn viewed(took: Duration) {
    note(|stats| &mut stats.view, took);
}

/// The window's whole view, its layout, redraws' updates and drawing
/// timed.
pub fn timed<'a, M: 'a>(root: impl Into<Element<'a, M>>) -> Element<'a, M> {
    Wrap::new(root, Timed).into()
}

struct Timed;

impl<M> Hooks<M> for Timed {
    type State = Stateless;

    fn layout(
        &mut self,
        limits: &iced::advanced::layout::Limits,
        child: &mut dyn FnMut(&iced::advanced::layout::Limits) -> iced::advanced::layout::Node,
    ) -> iced::advanced::layout::Node {
        let start = Instant::now();
        let node = child(limits);
        note(|stats| &mut stats.layout, start.elapsed());
        node
    }

    fn update(
        &mut self,
        _state: &mut Stateless,
        event: &iced::Event,
        _at: At,
        shell: &mut Shell<'_, M>,
        child: &mut Inner<'_, '_, M>,
    ) {
        if matches!(
            event,
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            let start = Instant::now();
            child.update(shell);
            note(|stats| &mut stats.update, start.elapsed());
        } else {
            child.update(shell);
        }
    }

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        _at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let start = Instant::now();
        child(renderer);
        note(|stats| &mut stats.draw, start.elapsed());
    }
}
