use std::time::Duration;

use glam::Vec2;

use crate::{Affine, Fonts, Glyphs, Mesh, Rect, Size, TextSection, Update, ViewState};

/// A base context that is shared between all other contexts.
pub struct BaseCx<'a> {
    fonts: &'a mut Fonts,
    delta_time: Duration,
}

impl<'a> BaseCx<'a> {
    pub(crate) fn set_delta_time(&mut self, delta_time: Duration) {
        self.delta_time = delta_time;
    }

    /// Create a new base context.
    pub fn new(fonts: &'a mut Fonts) -> Self {
        Self {
            fonts,
            delta_time: Duration::ZERO,
        }
    }
}

/// A context for building the view tree.
pub struct BuildCx<'a, 'b> {
    pub(crate) base: &'a mut BaseCx<'b>,
}

impl<'a, 'b> BuildCx<'a, 'b> {
    pub(crate) fn new(base: &'a mut BaseCx<'b>) -> Self {
        Self { base }
    }

    /// Create a child context.
    pub fn child(&mut self) -> BuildCx<'_, 'b> {
        BuildCx { base: self.base }
    }
}

/// A context for rebuilding the view tree.
pub struct RebuildCx<'a, 'b> {
    pub(crate) base: &'a mut BaseCx<'b>,
    pub(crate) view_state: &'a mut ViewState,
}

impl<'a, 'b> RebuildCx<'a, 'b> {
    pub(crate) fn new(base: &'a mut BaseCx<'b>, view_state: &'a mut ViewState) -> Self {
        Self { base, view_state }
    }

    pub(crate) fn build_cx(&mut self) -> BuildCx<'_, 'b> {
        BuildCx::new(self.base)
    }

    /// Create a child context.
    pub fn child(&mut self) -> RebuildCx<'_, 'b> {
        RebuildCx {
            base: self.base,
            view_state: self.view_state,
        }
    }
}

/// A context for handling events.
pub struct EventCx<'a, 'b> {
    pub(crate) base: &'a mut BaseCx<'b>,
    pub(crate) view_state: &'a mut ViewState,
    pub(crate) transform: Affine,
}

impl<'a, 'b> EventCx<'a, 'b> {
    pub(crate) fn new(base: &'a mut BaseCx<'b>, view_state: &'a mut ViewState) -> Self {
        let transform = view_state.transform;

        Self {
            base,
            view_state,
            transform,
        }
    }

    /// Create a child context.
    pub fn child(&mut self) -> EventCx<'_, 'b> {
        EventCx {
            base: self.base,
            view_state: self.view_state,
            transform: self.transform,
        }
    }

    /// Get the transform of the view.
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// Transform a point from global space to local space.
    pub fn local(&self, point: Vec2) -> Vec2 {
        self.transform.inverse() * point
    }
}

/// A context for laying out the view tree.
pub struct LayoutCx<'a, 'b> {
    pub(crate) base: &'a mut BaseCx<'b>,
    pub(crate) view_state: &'a mut ViewState,
}

impl<'a, 'b> LayoutCx<'a, 'b> {
    pub(crate) fn new(base: &'a mut BaseCx<'b>, view_state: &'a mut ViewState) -> Self {
        Self { base, view_state }
    }

    /// Create a child context.
    pub fn child(&mut self) -> LayoutCx<'_, 'b> {
        LayoutCx {
            base: self.base,
            view_state: self.view_state,
        }
    }
}

/// A context for drawing the view tree.
pub struct DrawCx<'a, 'b> {
    pub(crate) base: &'a mut BaseCx<'b>,
    pub(crate) view_state: &'a mut ViewState,
}

impl<'a, 'b> DrawCx<'a, 'b> {
    pub(crate) fn new(base: &'a mut BaseCx<'b>, view_state: &'a mut ViewState) -> Self {
        Self { base, view_state }
    }

    /// Create a child context.
    pub fn layer(&mut self) -> DrawCx<'_, 'b> {
        DrawCx {
            base: self.base,
            view_state: self.view_state,
        }
    }

    /// Create a mesh for the given glyphs.
    pub fn text_mesh(&mut self, glyphs: &Glyphs, rect: Rect) -> Option<Mesh> {
        self.base.fonts.text_mesh(glyphs, rect)
    }
}

macro_rules! impl_context {
    ($ty:ty { $($impl:item)* }) => {
        impl $ty {
            $($impl)*
        }
    };
    ($ty:ty, $($mode:ty),* { $($impl:item)* }) => {
        impl_context!($ty { $($impl)* });
        impl_context!($($mode),* { $($impl)* });
    };
}

impl_context! {EventCx<'_, '_>, DrawCx<'_, '_> {
    /// Get the size of the view.
    pub fn size(&self) -> Size {
        self.view_state.size
    }

    /// Get the rect of the view in local space.
    pub fn rect(&self) -> Rect {
        Rect::min_size(Vec2::ZERO, self.size())
    }
}}

impl_context! {BuildCx<'_, '_>, RebuildCx<'_, '_>, EventCx<'_, '_>, LayoutCx<'_, '_>, DrawCx<'_, '_> {
    /// Get the fonts.
    pub fn fonts(&mut self) -> &mut Fonts {
        self.base.fonts
    }
}}

impl_context! {RebuildCx<'_, '_>, EventCx<'_, '_>, LayoutCx<'_, '_>, DrawCx<'_, '_> {
    /// Get the delta time in seconds.
    pub fn dt(&self) -> f32 {
        self.base.delta_time.as_secs_f32()
    }

    /// Get whether the view is hot.
    pub fn is_hot(&self) -> bool {
        self.view_state.is_hot()
    }

    /// Set whether the view is hot.
    ///
    /// Returns `true` if the hot state changed.
    pub fn set_hot(&mut self, hot: bool) -> bool {
        let updated = self.is_hot() != hot;
        self.view_state.set_hot(hot);
        updated
    }

    /// Get whether the view is active.
    pub fn is_active(&self) -> bool {
        self.view_state.is_active()
    }

    /// Set whether the view is active.
    ///
    /// Returns `true` if the active state changed.
    pub fn set_active(&mut self, active: bool) -> bool {
        let updated = self.is_active() != active;
        self.view_state.set_active(active);
        updated
    }

    /// Get the flex of the view.
    pub fn flex(&self) -> f32 {
        self.view_state.flex
    }

    /// Set the flex of the view.
    pub fn set_flex(&mut self, flex: f32) {
        self.view_state.set_flex(flex);
    }

    /// Request a rebuild of the view tree.
    pub fn request_rebuild(&mut self) {
        self.view_state.update |= Update::TREE;
    }

    /// Request a layout of the view tree.
    pub fn request_layout(&mut self) {
        self.view_state.update |= Update::DRAW | Update::LAYOUT;
    }

    /// Request a draw of the view tree.
    pub fn request_draw(&mut self) {
        self.view_state.update |= Update::DRAW;
    }

    /// Layout the given [`TextSection`].
    pub fn layout_text(&mut self, text: &TextSection<'_>) -> Option<Glyphs> {
        self.base.fonts.layout_text(text)
    }
}}