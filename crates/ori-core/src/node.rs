use std::{any::Any, fmt::Debug, time::Instant};

use glam::Vec2;
use ori_graphics::{Frame, Rect, Renderer};
use uuid::Uuid;

use crate::{
    AnyView, BoxConstraints, Context, Cursor, DrawContext, EmptyView, Event, EventContext,
    EventSink, Guard, ImageCache, IntoView, LayoutContext, Lock, Lockable, Margin, OwnedSignal,
    PointerEvent, RequestRedrawEvent, Shared, Style, StyleCache, StyleSelector, StyleSelectors,
    StyleStates, StyleTransition, Stylesheet, TransitionStates, View,
};

/// A node identifier. This uses a UUID to ensure that nodes are unique.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    uuid: Uuid,
}

impl NodeId {
    /// Create a new node identifier, using uuid v4.
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }

    /// Gets the inner uuid.
    pub const fn uuid(self) -> Uuid {
        self.uuid
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// The state of a node, which is used to store information about the node.
///
/// This should almost never be used directly, and instead should be used through the [`Node`]
/// struct.
#[derive(Clone, Debug)]
pub struct NodeState {
    pub id: NodeId,
    pub local_rect: Rect,
    pub global_rect: Rect,
    pub active: bool,
    pub focused: bool,
    pub hovered: bool,
    pub last_draw: Instant,
    pub style: Style,
    pub recreated: OwnedSignal<bool>,
    pub transitions: TransitionStates,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            id: NodeId::new(),
            local_rect: Rect::ZERO,
            global_rect: Rect::ZERO,
            active: false,
            focused: false,
            hovered: false,
            last_draw: Instant::now(),
            style: Style::default(),
            recreated: OwnedSignal::new(true),
            transitions: TransitionStates::new(),
        }
    }
}

impl NodeState {
    /// Create a new node state with the given style.
    pub fn new(style: Style) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    /// Propagate the node state up to the parent.
    ///
    /// This is called before events are propagated.
    pub fn propagate_up(&mut self, parent: &NodeState) {
        self.global_rect = self.local_rect.translate(parent.global_rect.min);
    }

    /// Propagate the node state down to the child.
    ///
    /// This is called after events are propagated.
    pub fn propagate_down(&mut self, _child: &NodeState) {}

    /// Returns the [`StyleStates´].
    pub fn style_states(&self) -> StyleStates {
        let mut states = StyleStates::new();

        if self.active {
            states.push("active");
        }

        if self.focused {
            states.push("focus");
        }

        if self.hovered {
            states.push("hover");
        }

        states
    }

    /// Returns the [`StyleSelector`].
    pub fn selector(&self) -> StyleSelector {
        StyleSelector {
            element: self.style.element.map(Into::into),
            classes: self.style.classes.clone(),
            states: self.style_states(),
        }
    }

    /// Returns the time in seconds since the last draw.
    pub fn delta(&self) -> f32 {
        self.last_draw.elapsed().as_secs_f32()
    }

    /// Transition a value.
    ///
    /// If the value is an [`f32`], or a [`Color`], then it will be transitioned.
    pub fn transition<T: 'static>(
        &mut self,
        name: &str,
        mut value: T,
        transition: Option<StyleTransition>,
    ) -> T {
        (self.transitions).transition_any(name, &mut value, transition);
        value
    }

    /// Update the transitions.
    pub fn update_transitions(&mut self) -> bool {
        self.transitions.update(self.delta())
    }

    /// Updates `self.last_draw` to the current time.
    fn draw(&mut self) {
        self.last_draw = Instant::now();
    }
}

struct NodeInner<T: View> {
    view_state: Lock<T::State>,
    node_state: Lock<NodeState>,
    view: T,
}

impl<T: View> NodeInner<T> {
    fn view_state(&self) -> Guard<T::State> {
        self.view_state.lock_mut()
    }

    fn node_state(&self) -> Guard<NodeState> {
        self.node_state.lock_mut()
    }
}

impl<T: View> Debug for NodeInner<T>
where
    T: Debug,
    T::State: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeInner")
            .field("view_state", &self.view_state)
            .field("node_state", &self.node_state)
            .field("view", &self.view.type_id())
            .finish()
    }
}

impl<T: IntoView> From<T> for Node<T::View> {
    fn from(into_view: T) -> Self {
        let view = into_view.into_view();
        let view_state = View::build(&view);
        let node_state = NodeState::new(View::style(&view));

        Self {
            inner: Shared::new(NodeInner {
                view_state: Lock::new(view_state),
                node_state: Lock::new(node_state),
                view,
            }),
        }
    }
}

pub trait IntoNode<T: View, U: ?Sized> {
    fn into_node(self) -> Node<T>;
}

impl<T: IntoView> IntoNode<T::View, T> for T {
    fn into_node(self) -> Node<T::View> {
        Node::from(self.into_view())
    }
}

impl<T: IntoView> IntoNode<Box<dyn AnyView>, dyn AnyView> for T {
    fn into_node(self) -> Node<Box<dyn AnyView>> {
        Node::from(Box::new(self.into_view()) as Box<dyn AnyView>)
    }
}

impl<T: View> IntoNode<T, Node<T>> for Node<T> {
    fn into_node(self) -> Node<T> {
        self
    }
}

/// A node in the ui tree.
///
/// A node is a wrapper around a view, and contains the state of the view.
#[derive(Clone)]
pub struct Node<T: View = Box<dyn AnyView>> {
    inner: Shared<NodeInner<T>>,
}

impl Node {
    pub fn empty() -> Self {
        Self::new(EmptyView)
    }
}

impl<T: View> Node<T> {
    /// Create a new node with the given [`View`].
    pub fn new<U: ?Sized>(view: impl IntoNode<T, U>) -> Self {
        view.into_node()
    }

    /// Returns a [`Guard`] to the [`AnyViewState`].
    ///
    /// Be careful when using this, as it can cause deadlocks.
    pub fn view_state(&self) -> Guard<T::State> {
        self.inner.view_state.lock_mut()
    }

    /// Returns a [`Guard`] to the [`NodeState`].
    ///
    /// Be careful when using this, as it can cause deadlocks.
    pub fn node_state(&self) -> Guard<NodeState> {
        self.inner.node_state.lock_mut()
    }

    /// Returns a reference to the [`View`].
    pub fn view(&self) -> &T {
        &self.inner.view
    }

    /// Sets the offset of the node, relative to the parent.
    pub fn set_offset(&self, offset: Vec2) {
        let mut node_state = self.node_state();

        let size = node_state.local_rect.size();
        node_state.local_rect = Rect::min_size(offset, size);
    }

    /// Returns the [`StyleStates`].
    pub fn style_states(&self) -> StyleStates {
        self.node_state().style_states()
    }

    /// Gets the local [`Rect`] of the node.
    pub fn local_rect(&self) -> Rect {
        self.node_state().local_rect
    }

    /// Gets the global [`Rect`] of the node.
    pub fn global_rect(&self) -> Rect {
        self.node_state().global_rect
    }

    /// Gets the size of the node.
    pub fn size(&self) -> Vec2 {
        self.local_rect().size()
    }
}

impl<T: View> Node<T> {
    /// Returns true if the node should be redrawn.
    fn handle_pointer_event(
        node_state: &mut NodeState,
        event: &PointerEvent,
        is_handled: bool,
    ) -> bool {
        let is_over = node_state.global_rect.contains(event.position) && !event.left && !is_handled;
        if is_over != node_state.hovered && event.is_motion() {
            node_state.hovered = is_over;
            true
        } else {
            false
        }
    }

    /// Update the cursor.
    fn update_cursor(cx: &mut impl Context) {
        let Some(cursor) = cx.style("cursor") else {
            return;
        };

        if cx.hovered() || cx.active() {
            cx.set_cursor(cursor);
        }
    }

    fn event_inner(inner: &NodeInner<T>, cx: &mut EventContext, event: &Event) {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();
        node_state.propagate_up(cx.state);

        if let Some(pointer_event) = event.get::<PointerEvent>() {
            if Self::handle_pointer_event(node_state, pointer_event, event.is_handled()) {
                cx.request_redraw();
            }
        }

        {
            let selector = node_state.selector();
            let selectors = cx.selectors.clone().with(selector);
            let mut cx = EventContext {
                state: node_state,
                renderer: cx.renderer,
                selectors: &selectors,
                selectors_hash: selectors.hash(),
                style: cx.style,
                style_cache: cx.style_cache,
                event_sink: cx.event_sink,
                image_cache: cx.image_cache,
                cursor: cx.cursor,
            };

            (inner.view).event(&mut inner.view_state(), &mut cx, event);
            Self::update_cursor(&mut cx);
        }

        cx.state.propagate_down(&node_state);
    }

    /// Handle an event.
    pub fn event(&self, cx: &mut EventContext, event: &Event) {
        Self::event_inner(&self.inner, cx, event);
    }

    fn layout_inner(inner: &NodeInner<T>, cx: &mut LayoutContext, bc: BoxConstraints) -> Vec2 {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();
        node_state.propagate_up(cx.state);

        let size = {
            let selector = node_state.selector();
            let selectors = cx.selectors.clone().with(selector);
            let mut cx = LayoutContext {
                state: node_state,
                renderer: cx.renderer,
                selectors: &selectors,
                selectors_hash: selectors.hash(),
                style: cx.style,
                style_cache: cx.style_cache,
                event_sink: cx.event_sink,
                image_cache: cx.image_cache,
                cursor: cx.cursor,
            };

            let margin = Margin::from_style(&mut cx, bc);
            cx.state.global_rect.translate(margin.top_left());

            let bc = bc.with_margin(margin);
            let size = inner.view.layout(&mut inner.view_state(), &mut cx, bc);
            let size = size + margin.size();

            Self::update_cursor(&mut cx);

            size
        };

        node_state.local_rect = Rect::min_size(node_state.local_rect.min, size);
        node_state.global_rect = Rect::min_size(node_state.global_rect.min, size);

        cx.state.propagate_down(&node_state);

        size
    }

    /// Layout the node.
    pub fn layout(&self, cx: &mut LayoutContext, bc: BoxConstraints) -> Vec2 {
        Self::layout_inner(&self.inner, cx, bc)
    }

    fn draw_inner(inner: &NodeInner<T>, cx: &mut DrawContext) {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();
        node_state.propagate_up(cx.state);

        {
            let selector = node_state.selector();
            let selectors = cx.selectors.clone().with(selector);
            let mut cx = DrawContext {
                state: node_state,
                frame: cx.frame,
                renderer: cx.renderer,
                selectors: &selectors,
                selectors_hash: selectors.hash(),
                style: cx.style,
                style_cache: cx.style_cache,
                event_sink: cx.event_sink,
                image_cache: cx.image_cache,
                cursor: cx.cursor,
            };

            inner.view.draw(&mut inner.view_state(), &mut cx);

            if cx.state.update_transitions() {
                cx.request_redraw();
            }

            cx.state.draw();

            Self::update_cursor(&mut cx);
        }

        cx.state.propagate_down(&node_state);
    }

    /// Draw the node.
    pub fn draw(&self, cx: &mut DrawContext) {
        Self::draw_inner(&self.inner, cx);
    }
}

impl<T: View> Node<T> {
    fn event_root_inner(
        inner: &NodeInner<T>,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        renderer: &dyn Renderer,
        event_sink: &EventSink,
        event: &Event,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();

        if let Some(pointer_event) = event.get::<PointerEvent>() {
            if Self::handle_pointer_event(node_state, pointer_event, event.is_handled()) {
                event_sink.emit(RequestRedrawEvent);
            }
        }

        let selector = node_state.selector();
        let selectors = StyleSelectors::new().with(selector);
        let mut cx = EventContext {
            state: node_state,
            renderer,
            selectors: &selectors,
            selectors_hash: selectors.hash(),
            style,
            style_cache,
            event_sink,
            image_cache,
            cursor: cursor_icon,
        };

        (inner.view).event(&mut inner.view_state(), &mut cx, event);
    }

    /// Handle an event on the root node.
    pub fn event_root(
        &self,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        renderer: &dyn Renderer,
        event_sink: &EventSink,
        event: &Event,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) {
        Self::event_root_inner(
            &self.inner,
            style,
            style_cache,
            renderer,
            event_sink,
            event,
            image_cache,
            cursor_icon,
        );
    }

    fn layout_root_inner(
        inner: &NodeInner<T>,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        renderer: &dyn Renderer,
        window_size: Vec2,
        event_sink: &EventSink,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) -> Vec2 {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();

        let selector = node_state.selector();
        let selectors = StyleSelectors::new().with(selector);
        let mut cx = LayoutContext {
            state: node_state,
            renderer,
            selectors: &selectors,
            selectors_hash: selectors.hash(),
            style,
            style_cache,
            event_sink,
            image_cache,
            cursor: cursor_icon,
        };

        let bc = BoxConstraints::new(Vec2::ZERO, window_size);
        let size = inner.view.layout(&mut inner.view_state(), &mut cx, bc);

        node_state.local_rect = Rect::min_size(node_state.local_rect.min, size);
        node_state.global_rect = Rect::min_size(node_state.global_rect.min, size);

        size
    }

    /// Layout the root node.
    pub fn layout_root(
        &self,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        renderer: &dyn Renderer,
        window_size: Vec2,
        event_sink: &EventSink,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) -> Vec2 {
        Self::layout_root_inner(
            &self.inner,
            style,
            style_cache,
            renderer,
            window_size,
            event_sink,
            image_cache,
            cursor_icon,
        )
    }

    fn draw_root_inner(
        inner: &NodeInner<T>,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        frame: &mut Frame,
        renderer: &dyn Renderer,
        event_sink: &EventSink,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) {
        let node_state = &mut inner.node_state();
        node_state.style = inner.view.style();

        let selector = node_state.selector();
        let selectors = StyleSelectors::new().with(selector);
        let mut cx = DrawContext {
            state: node_state,
            frame,
            renderer,
            selectors: &selectors,
            selectors_hash: selectors.hash(),
            style,
            style_cache,
            event_sink,
            image_cache,
            cursor: cursor_icon,
        };

        inner.view.draw(&mut inner.view_state(), &mut cx);

        cx.state.draw();
    }

    /// Draw the root node.
    pub fn draw_root(
        &self,
        style: &Stylesheet,
        style_cache: &mut StyleCache,
        frame: &mut Frame,
        renderer: &dyn Renderer,
        event_sink: &EventSink,
        image_cache: &mut ImageCache,
        cursor_icon: &mut Cursor,
    ) {
        Self::draw_root_inner(
            &self.inner,
            style,
            style_cache,
            frame,
            renderer,
            event_sink,
            image_cache,
            cursor_icon,
        );
    }
}

impl<T: View> Debug for Node<T>
where
    T: Debug,
    T::State: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("inner", &self.inner).finish()
    }
}