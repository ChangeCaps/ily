use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    BoxedView, BuildCx, Canvas, DrawCx, Event, EventCx, LayoutCx, RebuildCx, Size, Space, Update,
    View, ViewState,
};

pub type AnyPod<T> = Pod<T, BoxedView<T>>;

pub struct PodState<T, V: View<T>> {
    content: V::State,
    view_state: ViewState,
}

impl<T, V: View<T>> Deref for PodState<T, V> {
    type Target = ViewState;

    fn deref(&self) -> &Self::Target {
        &self.view_state
    }
}

impl<T, V: View<T>> DerefMut for PodState<T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view_state
    }
}

pub struct Pod<T, V> {
    content: V,
    marker: PhantomData<fn() -> T>,
}

impl<T, V> Pod<T, V> {
    pub const fn new(content: V) -> Self {
        Self {
            content,
            marker: PhantomData,
        }
    }
}

impl<T, V: View<T>> View<T> for Pod<T, V> {
    type State = PodState<T, V>;

    fn build(&mut self, cx: &mut BuildCx, data: &mut T) -> Self::State {
        PodState {
            content: self.content.build(cx, data),
            view_state: ViewState::default(),
        }
    }

    fn rebuild(&mut self, state: &mut Self::State, cx: &mut RebuildCx, data: &mut T, old: &Self) {
        state.view_state.update.remove(Update::TREE);

        let mut new_cx = cx.child();
        new_cx.view_state = &mut state.view_state;

        (self.content).rebuild(&mut state.content, &mut new_cx, data, &old.content);

        cx.view_state.propagate(&mut state.view_state);
    }

    fn event(&mut self, state: &mut Self::State, cx: &mut EventCx, data: &mut T, event: &Event) {
        let mut new_cx = cx.child();
        new_cx.transform *= state.view_state.transform;
        new_cx.view_state = &mut state.view_state;

        (self.content).event(&mut state.content, &mut new_cx, data, event);

        cx.view_state.propagate(&mut state.view_state);
    }

    fn layout(
        &mut self,
        state: &mut Self::State,
        cx: &mut LayoutCx,
        data: &mut T,
        space: Space,
    ) -> Size {
        state.view_state.update.remove(Update::LAYOUT);

        let mut new_cx = cx.child();
        new_cx.view_state = &mut state.view_state;

        let size = (self.content).layout(&mut state.content, &mut new_cx, data, space);
        state.view_state.size = size;

        cx.view_state.propagate(&mut state.view_state);

        size
    }

    fn draw(
        &mut self,
        state: &mut Self::State,
        cx: &mut DrawCx,
        data: &mut T,
        canvas: &mut Canvas,
    ) {
        state.view_state.update.remove(Update::DRAW);

        // create the canvas layer
        let mut canvas = canvas.layer();
        canvas.transform *= state.view_state.transform;

        // create the draw context
        let mut new_cx = cx.layer();
        new_cx.view_state = &mut state.view_state;

        // draw the content
        (self.content).draw(&mut state.content, &mut new_cx, data, &mut canvas);

        // propagate the view state
        cx.view_state.propagate(&mut state.view_state);
    }
}

impl<T, V: Debug> Debug for Pod<T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pod")
            .field("content", &self.content)
            .finish()
    }
}

impl<T, V: Clone> Clone for Pod<T, V> {
    fn clone(&self) -> Self {
        Self {
            content: self.content.clone(),
            marker: PhantomData,
        }
    }
}

impl<T, V: Copy> Copy for Pod<T, V> {}

impl<T, V: Default> Default for Pod<T, V> {
    fn default() -> Self {
        Self::new(V::default())
    }
}

impl<T, V> Deref for Pod<T, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<T, V> DerefMut for Pod<T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}