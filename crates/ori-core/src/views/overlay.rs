use crate::{
    canvas::Canvas,
    event::Event,
    layout::{Size, Space},
    view::{BuildCx, ContentSeq, DrawCx, EventCx, LayoutCx, RebuildCx, SeqState, View, ViewSeq},
};

pub use crate::overlay;

/// Create a new overlay view.
#[macro_export]
macro_rules! overlay {
    (for $data:ident in $content:expr) => {
        $crate::views::overlay(
            <::std::vec::Vec<_> as ::std::iter::FromIterator<_>>::from_iter($iter)
        )
    };
    ($($child:expr),* $(,)?) => {
        $crate::views::overlay(($($child,)*))
    };
}

/// Create a new overlay view.
pub fn overlay<V>(content: V) -> Overlay<V> {
    Overlay::new(content)
}

/// A view that overlays its content on top of each other.
pub struct Overlay<V> {
    /// The content to overlay.
    pub content: ContentSeq<V>,
}

impl<V> Overlay<V> {
    /// Create a new overlay view.
    pub fn new(content: V) -> Self {
        Self {
            content: ContentSeq::new(content),
        }
    }
}

impl<T, V: ViewSeq<T>> View<T> for Overlay<V> {
    type State = SeqState<T, V>;

    fn build(&mut self, cx: &mut BuildCx, data: &mut T) -> Self::State {
        self.content.build(cx, data)
    }

    fn rebuild(&mut self, state: &mut Self::State, cx: &mut RebuildCx, data: &mut T, old: &Self) {
        (self.content).rebuild(state, &mut cx.build_cx(), data, &old.content);

        for i in 0..self.content.len() {
            self.content.rebuild_nth(i, state, cx, data, &old.content);
        }
    }

    fn event(&mut self, state: &mut Self::State, cx: &mut EventCx, data: &mut T, event: &Event) {
        for i in (0..self.content.len()).rev() {
            self.content.event_nth(i, state, cx, data, event);
        }
    }

    fn layout(
        &mut self,
        state: &mut Self::State,
        cx: &mut LayoutCx,
        data: &mut T,
        space: Space,
    ) -> Size {
        let mut size = self.content.layout_nth(0, state, cx, data, space);

        for i in 1..self.content.len() {
            let content_space = Space::new(space.min, size);
            let content_size = self.content.layout_nth(i, state, cx, data, content_space);
            size = size.max(content_size);
        }

        space.fit(size)
    }

    fn draw(
        &mut self,
        state: &mut Self::State,
        cx: &mut DrawCx,
        data: &mut T,
        canvas: &mut Canvas,
    ) {
        for i in 0..self.content.len() {
            let mut layer = canvas.layer();
            layer.depth += i as f32 * 1000.0;

            self.content.draw_nth(i, state, cx, data, &mut layer);
        }
    }
}