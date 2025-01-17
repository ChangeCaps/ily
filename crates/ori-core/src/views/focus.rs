use crate::{
    context::{BuildCx, DrawCx, EventCx, LayoutCx, RebuildCx},
    event::Event,
    layout::{Size, Space},
    view::View,
};

/// Create a new [`Focus`].
pub fn focus<T, U, V: View<U>>(
    content: V,
    focus: impl FnMut(&mut T, &mut Lens<U>) + 'static,
) -> Focus<T, U, V> {
    Focus::new(content, focus)
}

/// A lens used by [`Focus`].
pub type Lens<'a, T> = dyn FnMut(&mut T) + 'a;

/// A view that focuses on a part of the data.
///
/// This is useful when using components that require specific data.
pub struct Focus<T, U, V> {
    content: V,
    #[allow(clippy::type_complexity)]
    focus: Box<dyn FnMut(&mut T, &mut Lens<U>)>,
}

impl<T, U, V> Focus<T, U, V> {
    /// Create a new [`Focus`].
    pub fn new(content: V, focus: impl FnMut(&mut T, &mut Lens<U>) + 'static) -> Self {
        Self {
            content,
            focus: Box::new(focus),
        }
    }
}

impl<T, U, V: View<U>> View<T> for Focus<T, U, V> {
    type State = V::State;

    fn build(&mut self, cx: &mut BuildCx, data: &mut T) -> Self::State {
        let mut state = None;

        (self.focus)(data, &mut |data| {
            state = Some(self.content.build(cx, data));
        });

        state.expect("focus did not call the lens")
    }

    fn rebuild(&mut self, state: &mut Self::State, cx: &mut RebuildCx, data: &mut T, old: &Self) {
        (self.focus)(data, &mut |data| {
            self.content.rebuild(state, cx, data, &old.content);
        });
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        cx: &mut EventCx,
        data: &mut T,
        event: &Event,
    ) -> bool {
        let mut handled = false;

        (self.focus)(data, &mut |data| {
            handled = self.content.event(state, cx, data, event);
        });

        handled
    }

    fn layout(
        &mut self,
        state: &mut Self::State,
        cx: &mut LayoutCx,
        data: &mut T,
        space: Space,
    ) -> Size {
        let mut size = space.min;

        (self.focus)(data, &mut |data| {
            size = self.content.layout(state, cx, data, space);
        });

        size
    }

    fn draw(&mut self, state: &mut Self::State, cx: &mut DrawCx, data: &mut T) {
        (self.focus)(data, &mut |data| {
            self.content.draw(state, cx, data);
        });
    }
}
