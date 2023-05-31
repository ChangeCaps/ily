use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut, Range},
};

use glam::Vec2;
use ori_graphics::{
    cosmic_text::FontSystem, Frame, ImageHandle, ImageSource, Quad, Rect, Renderer, TextSection,
    WeakImageHandle,
};
use ori_reactive::EventSink;
use ori_style::{
    FromStyleAttribute, Length, StyleAttribute, StyleCache, StyleCacheHash, StyleSpec, StyleTree,
    Stylesheet,
};

use crate::{AvailableSpace, ElementState, Margin, Padding, RequestRedrawEvent, Window};

/// A cache for images.
///
/// This is used to avoid loading the same image multiple times.
#[derive(Clone, Debug, Default)]
pub struct ImageCache {
    images: HashMap<ImageSource, WeakImageHandle>,
}

impl ImageCache {
    /// Creates a new image cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of images in the cache.
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Returns `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Gets an image from the cache.
    pub fn get(&self, source: &ImageSource) -> Option<ImageHandle> {
        self.images.get(source)?.upgrade()
    }

    /// Inserts an image into the cache.
    pub fn insert(&mut self, source: ImageSource, handle: ImageHandle) {
        self.images.insert(source, handle.downgrade());
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.images.clear();
    }

    /// Removes all images that are no longer in use.
    pub fn clean(&mut self) {
        self.images.retain(|_, v| v.is_alive());
    }
}

/// A context for [`View::event`](crate::View::event).
#[allow(missing_docs)]
pub struct EventContext<'a> {
    pub state: &'a mut ElementState,
    pub renderer: &'a dyn Renderer,
    pub window: &'a mut Window,
    pub font_system: &'a mut FontSystem,
    pub stylesheet: &'a Stylesheet,
    pub style_tree: &'a mut StyleTree,
    pub style_cache: &'a mut StyleCache,
    pub event_sink: &'a EventSink,
    pub image_cache: &'a mut ImageCache,
}

/// A context for [`View::layout`](crate::View::layout).
#[allow(missing_docs)]
pub struct LayoutContext<'a> {
    pub state: &'a mut ElementState,
    pub renderer: &'a dyn Renderer,
    pub window: &'a mut Window,
    pub font_system: &'a mut FontSystem,
    pub stylesheet: &'a Stylesheet,
    pub style_tree: &'a mut StyleTree,
    pub style_cache: &'a mut StyleCache,
    pub event_sink: &'a EventSink,
    pub image_cache: &'a mut ImageCache,
    pub parent_space: AvailableSpace,
    pub space: AvailableSpace,
}

impl<'a> LayoutContext<'a> {
    /// Gets the available space, constrained by the element's style.
    pub fn style_constraints(&mut self, space: AvailableSpace) -> AvailableSpace {
        let parent_space = self.parent_space;
        let min_width = self.style_range_group(&["min-width", "width"], parent_space.x_axis());
        let max_width = self.style_range_group(&["max-width", "width"], parent_space.x_axis());

        let min_height = self.style_range_group(&["min-height", "height"], parent_space.y_axis());
        let max_height = self.style_range_group(&["max-height", "height"], parent_space.y_axis());

        let min_size = space.constrain(Vec2::new(min_width, min_height));
        let max_size = space.constrain(Vec2::new(max_width, max_height));

        AvailableSpace::new(min_size, max_size)
    }

    /// Calls `f`, temporarily changing the available space.
    pub fn with_space<T>(&mut self, space: AvailableSpace, f: impl FnOnce(&mut Self) -> T) -> T {
        let tmp = self.space;
        self.space = space;
        let result = f(self);
        self.space = tmp;
        result
    }

    /// Measure the bounds of a text section.
    pub fn measure_text(&mut self, text: &TextSection) -> Rect {
        text.measure(self.font_system)
    }
}

/// A layer for drawing, see [`DrawContext::layer`](DrawContext::layer).
pub struct DrawLayer<'a, 'b> {
    draw_context: &'b mut DrawContext<'a>,
    z_index: f32,
    clip: Option<Rect>,
}

impl<'a, 'b> DrawLayer<'a, 'b> {
    /// Set the z-index of the layer.
    pub fn z_index(mut self, depth: f32) -> Self {
        self.z_index = depth;
        self
    }

    /// Set the clipping rectangle for the layer.
    pub fn clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip.round());
        self
    }

    /// Draw the layer.
    pub fn draw(self, f: impl FnOnce(&mut DrawContext)) {
        let layer = self
            .draw_context
            .frame
            .layer()
            .z_index(self.z_index)
            .clip(self.clip);

        layer.draw(|frame| {
            let mut child = DrawContext {
                state: self.draw_context.state,
                frame,
                renderer: self.draw_context.renderer,
                window: self.draw_context.window,
                font_system: self.draw_context.font_system,
                stylesheet: self.draw_context.stylesheet,
                style_tree: self.draw_context.style_tree,
                style_cache: self.draw_context.style_cache,
                event_sink: self.draw_context.event_sink,
                image_cache: self.draw_context.image_cache,
            };

            f(&mut child);
        });
    }
}

/// A context for [`View::draw`](crate::View::draw).
#[allow(missing_docs)]
pub struct DrawContext<'a> {
    pub state: &'a mut ElementState,
    pub frame: &'a mut Frame,
    pub renderer: &'a dyn Renderer,
    pub window: &'a mut Window,
    pub font_system: &'a mut FontSystem,
    pub stylesheet: &'a Stylesheet,
    pub style_tree: &'a mut StyleTree,
    pub style_cache: &'a mut StyleCache,
    pub event_sink: &'a EventSink,
    pub image_cache: &'a mut ImageCache,
}

impl<'a> DrawContext<'a> {
    /// Returns the frame.
    pub fn frame(&mut self) -> &mut Frame {
        self.frame
    }

    /// Returns a new layer, see [`Frame::layer`].
    pub fn layer<'b>(&'b mut self) -> DrawLayer<'a, 'b> {
        DrawLayer {
            draw_context: self,
            z_index: 1.0,
            clip: None,
        }
    }

    /// Runs the given callback on a new layer offset by the given amount.
    ///
    /// `offset` should almost always be `1.0`.
    pub fn draw_layer(&mut self, f: impl FnOnce(&mut DrawContext)) {
        self.layer().draw(f);
    }

    /// Draws the quad at the current layout rect.
    ///
    /// This will use the following style attributes:
    /// - `background-color`: The background color of the quad.
    /// - `border-radius`: The border radius of the quad overwritten by the more specific
    /// attributes.
    /// - `border-top-left-radius`: The top left border radius of the quad.
    /// - `border-top-right-radius`: The top right border radius of the quad.
    /// - `border-bottom-right-radius`: The bottom right border radius of the quad.
    /// - `border-bottom-left-radius`: The bottom left border radius of the quad.
    /// - `border-width`: The border width of the quad.
    pub fn draw_quad(&mut self) {
        let range = 0.0..self.rect().max.min_element() / 2.0;

        let tl = "border-top-left-radius";
        let tr = "border-top-right-radius";
        let br = "border-bottom-right-radius";
        let bl = "border-bottom-left-radius";

        let tl = self.style_range_group(&[tl, "border-radius"], range.clone());
        let tr = self.style_range_group(&[tr, "border-radius"], range.clone());
        let br = self.style_range_group(&[br, "border-radius"], range.clone());
        let bl = self.style_range_group(&[bl, "border-radius"], range.clone());

        let quad = Quad {
            rect: self.rect(),
            background: self.style("background-color"),
            border_radius: [tl, tr, br, bl],
            border_width: self.style_range("border-width", range),
            border_color: self.style("border-color"),
        };

        self.draw(quad);
    }
}

impl<'a> Deref for DrawContext<'a> {
    type Target = Frame;

    fn deref(&self) -> &Self::Target {
        self.frame
    }
}

impl<'a> DerefMut for DrawContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.frame
    }
}

/// A context that is passed to [`View`](crate::view::View) methods.
///
/// See [`EventContext`], [`DrawContext`] and [`LayoutContext`] for more information.
pub trait Context {
    /// Returns the [`Stylesheet`] of the application.
    fn stylesheet(&self) -> &Stylesheet;

    /// Returns the [`StyleCache`] of the application.
    fn style_cache(&self) -> &StyleCache;

    /// Returns the [`StyleCache`] of the application.
    fn style_cache_mut(&mut self) -> &mut StyleCache;

    /// Returns the [`ElementState`] of the current element.
    fn state(&self) -> &ElementState;

    /// Returns the [`ElementState`] of the current element.
    fn state_mut(&mut self) -> &mut ElementState;

    /// Returns the [`Renderer`] of the application.
    fn renderer(&self) -> &dyn Renderer;

    /// Returns the [`Window`] of the application.
    fn window(&self) -> &Window;

    /// Returns the [`Window`] of the application.
    fn window_mut(&mut self) -> &mut Window;

    /// Returns the [`FontSystem`] of the application.
    fn font_system(&self) -> &FontSystem;

    /// Returns the [`FontSystem`] of the application.
    fn font_system_mut(&mut self) -> &mut FontSystem;

    /// Returns the [`StyleTree`] of the current element.
    fn style_tree(&self) -> &StyleTree;

    /// Returns the [`StyleTree`] of the current element.
    fn style_tree_mut(&mut self) -> &mut StyleTree;

    /// Returns the [`EventSink`] of the application.
    fn event_sink(&self) -> &EventSink;

    /// Returns the [`ImageCache`] of the application.
    fn image_cache(&self) -> &ImageCache;

    /// Returns the [`ImageCache`] of the application.
    fn image_cache_mut(&mut self) -> &mut ImageCache;

    /// Gets the [`StyleAttribute`] for the given `key`.
    fn get_style_attribute(&mut self, key: &str) -> Option<StyleAttribute> {
        self.get_style_attribute_specificity(key)
            .map(|(attribute, _)| attribute)
    }

    /// Gets the [`StyleAttribute`] and [`StyleSpec`] for the given `key`.
    fn get_style_attribute_specificity(
        &mut self,
        key: &str,
    ) -> Option<(StyleAttribute, StyleSpec)> {
        // get inline style attribute
        if let Some(attribute) = self.state().style.get_attribute(key) {
            return Some((attribute.clone(), StyleSpec::INLINE));
        }

        let hash = StyleCacheHash::new(self.style_tree());

        // try to get cached attribute
        if let Some(result) = self.style_cache().get(hash, key) {
            return result;
        }

        let stylesheet = self.stylesheet();

        // get attribute from stylesheet
        match stylesheet.get_attribute_specificity(self.style_tree(), key) {
            Some((attribute, specificity)) => {
                // cache result
                (self.style_cache_mut()).insert(hash, attribute.clone(), specificity);
                Some((attribute, specificity))
            }
            None => {
                // cache result
                self.style_cache_mut().insert_none(hash, key);
                None
            }
        }
    }

    /// Gets the value of a style attribute for the given `key`.
    fn get_style_specificity<T: FromStyleAttribute + 'static>(
        &mut self,
        key: &str,
    ) -> Option<(T, StyleSpec)> {
        let (attribute, specificity) = self.get_style_attribute_specificity(key)?;
        let value = T::from_attribute(attribute.value().clone())?;
        let transition = attribute.transition();

        Some((
            self.state_mut().transition(key, value, transition),
            specificity,
        ))
    }

    /// Gets the value of a style attribute for the given `key`.
    ///
    /// This will also transition the value if the attribute has a transition.
    fn get_style<T: FromStyleAttribute + 'static>(&mut self, key: &str) -> Option<T> {
        self.get_style_specificity(key).map(|(value, _)| value)
    }

    /// Gets the value of a style attribute for the given `key`, if there is no value, returns `T::default()`.
    ///
    /// This will also transition the value if the attribute has a transition.
    #[track_caller]
    fn style<T: FromStyleAttribute + Default + 'static>(&mut self, key: &str) -> T {
        self.get_style(key).unwrap_or_default()
    }

    /// Takes a `primary_key` and a `secondary_key` and returns the value of the attribute with the highest specificity.
    /// If both attributes have the same specificity, the `primary_key` will be used.
    ///
    /// This will also transition the value if the attribute has a transition.
    fn style_group<T: FromStyleAttribute + Default + 'static>(&mut self, keys: &[&str]) -> T {
        let mut specificity = None;
        let mut result = None;

        for key in keys {
            if let Some((v, s)) = self.get_style_specificity(key) {
                if specificity.is_none() || s > specificity.unwrap() {
                    specificity = Some(s);
                    result = Some(v);
                }
            }
        }

        result.unwrap_or_default()
    }

    /// Gets the value of a style attribute in pixels for the given `key`.
    /// `range` is the range from 0% to 100% of the desired value.
    ///
    /// This will also transition the value if the attribute has a transition.
    fn get_style_range(&mut self, key: &str, range: Range<f32>) -> Option<f32> {
        let attribute = self.get_style_attribute(key)?;
        let value = Length::from_attribute(attribute.value().clone())?;
        let transition = attribute.transition();

        let scale = self.window().scale;
        let width = self.window().size.x as f32;
        let height = self.window().size.y as f32;
        let pixels = value.pixels(range, scale, width, height);

        Some((self.state_mut()).transition(key, pixels, transition))
    }

    /// Gets the value of a style attribute in pixels and [`StyleSpec`] for the given `key`.
    fn get_style_range_specificity(
        &mut self,
        key: &str,
        range: Range<f32>,
    ) -> Option<(f32, StyleSpec)> {
        let (attribute, specificity) = self.get_style_attribute_specificity(key)?;
        let value = Length::from_attribute(attribute.value().clone())?;
        let transition = attribute.transition();

        let scale = self.window().scale;
        let width = self.window().size.x as f32;
        let height = self.window().size.y as f32;
        let pixels = value.pixels(range, scale, width, height);

        Some((
            (self.state_mut()).transition(key, pixels, transition),
            specificity,
        ))
    }

    /// Gets the value of a style attribute in pixels for the given `key`, if there is no value, returns `0.0`.
    /// `range` is the range from 0% to 100% of the desired value.
    ///
    /// This will also transition the value if the attribute has a transition.
    #[track_caller]
    fn style_range(&mut self, key: &str, range: Range<f32>) -> f32 {
        self.get_style_range(key, range).unwrap_or_default()
    }

    /// Takes a `primary_key` and a `secondary_key` and returns the value of the attribute with the highest specificity in pixels.
    /// If both attributes have the same specificity, the `primary_key` will be used.
    /// `range` is the range from 0% to 100% of the desired value.
    ///
    /// This will also transition the value if the attribute has a transition.
    fn style_range_group(&mut self, keys: &[&str], range: Range<f32>) -> f32 {
        let mut specificity = None;
        let mut result = None;

        for key in keys {
            if let Some((v, s)) = self.get_style_range_specificity(key, range.clone()) {
                if specificity.is_none() || s > specificity.unwrap() {
                    specificity = Some(s);
                    result = Some(v);
                }
            }
        }

        result.unwrap_or_default()
    }

    /// Tries to downcast the `renderer` to the given type.
    fn downcast_renderer<T: Renderer>(&self) -> Option<&T> {
        self.renderer().downcast_ref()
    }

    /// Loads an image from the given `source` and returns a handle to it.
    fn load_image(&mut self, source: ImageSource) -> ImageHandle {
        if let Some(handle) = self.image_cache().get(&source) {
            return handle;
        }

        let data = source.clone().load();
        let image = self.renderer().create_image(&data);
        self.image_cache_mut().insert(source, image.clone());
        image
    }

    /// Returns `true` if the element is active.
    fn active(&self) -> bool {
        self.state().active
    }

    /// Returns `true` if the element is hovered.
    fn hovered(&self) -> bool {
        self.state().hovered
    }

    /// Returns `true` if the element is focused.
    fn focused(&self) -> bool {
        self.state().focused
    }

    /// Focuses the element, this will also request a redraw.
    fn focus(&mut self) {
        if self.focused() {
            return;
        }

        self.state_mut().focused = true;
        self.request_redraw();
    }

    /// Unfocuses the element, this will also request a redraw.
    fn unfocus(&mut self) {
        if !self.focused() {
            return;
        }

        self.state_mut().focused = false;
        self.request_redraw();
    }

    /// Hovers the element, this will also request a redraw.
    fn hover(&mut self) {
        if self.hovered() {
            return;
        }

        self.state_mut().hovered = true;
        self.request_redraw();
    }

    /// Unhovers the element, this will also request a redraw.
    fn unhover(&mut self) {
        if !self.hovered() {
            return;
        }

        self.state_mut().hovered = false;
        self.request_redraw();
    }

    /// Activates the element, this will also request a redraw.
    fn activate(&mut self) {
        if self.active() {
            return;
        }

        self.state_mut().active = true;
        self.request_redraw();
    }

    /// Deactivates the element, this will also request a redraw.
    fn deactivate(&mut self) {
        if !self.active() {
            return;
        }

        self.state_mut().active = false;
        self.request_redraw();
    }

    /// Returns the local rect of the element.
    fn local_rect(&self) -> Rect {
        self.state().local_rect
    }

    /// Returns the global rect of the element.
    fn rect(&self) -> Rect {
        self.state().global_rect
    }

    /// Returns the margin of the element.
    fn margin(&self) -> Margin {
        self.state().margin
    }

    /// Returns the padding of the element.
    fn padding(&self) -> Padding {
        self.state().padding
    }

    /// Returns the size of the element.
    fn size(&self) -> Vec2 {
        self.state().local_rect.size()
    }

    /// Requests a redraw.
    ///
    /// This is a shortcut for `self.event_sink().send(RequestRedrawEvent)`.
    #[track_caller]
    fn request_redraw(&mut self) {
        tracing::trace!("request redraw");
        self.send_event(RequestRedrawEvent);
    }

    /// Requests a layout.
    ///
    /// This is a shortcut for `self.state_mut().needs_layout = true`.
    #[track_caller]
    fn request_layout(&mut self) {
        tracing::trace!("request layout");
        self.state_mut().needs_layout = true;
    }

    /// Sends an event to the event sink.
    fn send_event(&self, event: impl Any + Send + Sync) {
        self.event_sink().emit(event);
    }

    /// Returns the time in seconds since the last frame.
    fn delta_time(&self) -> f32 {
        self.state().delta_time()
    }
}

macro_rules! context {
    ($name:ident) => {
        impl<'a> Context for $name<'a> {
            fn stylesheet(&self) -> &Stylesheet {
                self.stylesheet
            }

            fn style_cache(&self) -> &StyleCache {
                self.style_cache
            }

            fn style_cache_mut(&mut self) -> &mut StyleCache {
                self.style_cache
            }

            fn state(&self) -> &ElementState {
                self.state
            }

            fn state_mut(&mut self) -> &mut ElementState {
                self.state
            }

            fn renderer(&self) -> &dyn Renderer {
                self.renderer
            }

            fn window(&self) -> &Window {
                self.window
            }

            fn window_mut(&mut self) -> &mut Window {
                self.window
            }

            fn font_system(&self) -> &FontSystem {
                self.font_system
            }

            fn font_system_mut(&mut self) -> &mut FontSystem {
                self.font_system
            }

            fn style_tree(&self) -> &StyleTree {
                self.style_tree
            }

            fn style_tree_mut(&mut self) -> &mut StyleTree {
                self.style_tree
            }

            fn event_sink(&self) -> &EventSink {
                &self.event_sink
            }

            fn image_cache(&self) -> &ImageCache {
                &self.image_cache
            }

            fn image_cache_mut(&mut self) -> &mut ImageCache {
                &mut self.image_cache
            }
        }
    };
}

context!(EventContext);
context!(LayoutContext);
context!(DrawContext);
