use crate::{
    canvas::Canvas,
    event::Event,
    layout::{Padding, Size, Space},
    rebuild::Rebuild,
    view::{BuildCx, Content, DrawCx, EventCx, LayoutCx, RebuildCx, State, View},
};

/// Create a new [`Pad`] view.
pub fn pad<V>(padding: impl Into<Padding>, content: V) -> Pad<V> {
    Pad::new(padding, content)
}

/// A view that adds padding to its content.
#[derive(Rebuild)]
pub struct Pad<V> {
    /// The content.
    pub content: Content<V>,
    /// The padding.
    #[rebuild(layout)]
    pub padding: Padding,
}

impl<V> Pad<V> {
    /// Create a new [`Pad`] view.
    pub fn new(padding: impl Into<Padding>, content: V) -> Self {
        Self {
            content: Content::new(content),
            padding: padding.into(),
        }
    }
}

impl<T, V: View<T>> View<T> for Pad<V> {
    type State = State<T, V>;

    fn build(&mut self, cx: &mut BuildCx, data: &mut T) -> Self::State {
        self.content.build(cx, data)
    }

    fn rebuild(&mut self, state: &mut Self::State, cx: &mut RebuildCx, data: &mut T, old: &Self) {
        Rebuild::rebuild(self, cx, old);

        self.content.rebuild(state, cx, data, &old.content);
    }

    fn event(&mut self, state: &mut Self::State, cx: &mut EventCx, data: &mut T, event: &Event) {
        self.content.event(state, cx, data, event);
    }

    fn layout(
        &mut self,
        state: &mut Self::State,
        cx: &mut LayoutCx,
        data: &mut T,
        space: Space,
    ) -> Size {
        let content_space = space.shrink(self.padding.size());
        let content_size = self.content.layout(state, cx, data, content_space);

        state.translate(self.padding.offset());

        space.fit(content_size + self.padding.size())
    }

    fn draw(
        &mut self,
        state: &mut Self::State,
        cx: &mut DrawCx,
        data: &mut T,
        canvas: &mut Canvas,
    ) {
        self.content.draw(state, cx, data, canvas);
    }
}