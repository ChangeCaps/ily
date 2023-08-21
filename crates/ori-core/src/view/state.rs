use glam::Vec2;

use crate::{
    canvas::Canvas,
    layout::{Affine, Size},
    window::Cursor,
};

bitflags::bitflags! {
    /// Flags that indicate what needs to be updated.
    #[must_use]
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct Update: u8 {
        /// The view needs to be laid out.
        const LAYOUT = 1 << 0;
        /// The view needs to be drawn.
        const DRAW = 1 << 1;
    }
}

/// State associated with a [`View`](super::View).
#[derive(Clone, Debug)]
pub struct ViewState {
    pub(crate) hot: bool,
    pub(crate) focused: bool,
    pub(crate) active: bool,
    pub(crate) has_active: bool,
    pub(crate) update: Update,
    /* layout */
    pub(crate) flex: f32,
    pub(crate) size: Size,
    pub(crate) transform: Affine,
    pub(crate) depth: f32,
    /* cursor */
    pub(crate) cursor: Cursor,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            hot: false,
            focused: false,
            active: false,
            has_active: false,
            update: Update::LAYOUT | Update::DRAW,
            flex: 0.0,
            size: Size::ZERO,
            transform: Affine::IDENTITY,
            depth: 0.0,
            cursor: Cursor::default(),
        }
    }
}

impl ViewState {
    pub(crate) fn prepare(&mut self) {
        self.has_active = self.active;
    }

    pub(crate) fn prepare_layout(&mut self) {
        self.prepare();
        self.layed_out();
    }

    pub(crate) fn prepare_draw(&mut self, canvas: &mut Canvas) {
        self.prepare();
        self.drawn();
        self.depth = canvas.depth;
    }

    pub(crate) fn propagate(&mut self, child: &mut Self) {
        self.has_active |= child.has_active;
        self.update |= child.update;
    }
}

impl ViewState {
    /// Get whether the view is hot.
    pub fn is_hot(&self) -> bool {
        self.hot
    }

    /// Set whether the view is hot.
    pub fn set_hot(&mut self, hot: bool) {
        self.hot = hot;
    }

    /// Get whether the view is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Set whether the view is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Get whether the view is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set whether the view is active.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.has_active = active;
    }

    /// Get whether the view has an active child.
    pub fn has_active(&self) -> bool {
        self.has_active
    }

    /// Get the flex of the view.
    pub fn flex(&self) -> f32 {
        self.flex
    }

    /// Set the flex of the view.
    pub fn set_flex(&mut self, flex: f32) {
        self.flex = flex;
    }

    /// Get the size of the view.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Get the transform of the view.
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// Set the transform of the view.
    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    /// Translate the transform of the view.
    pub fn translate(&mut self, translation: Vec2) {
        self.transform = Affine::translate(translation);
    }

    /// Get the depth of the view.
    pub fn depth(&self) -> f32 {
        self.depth
    }

    /// Set the depth of the view.
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }

    /// Request a layout of the view tree.
    pub fn request_layout(&mut self) {
        self.update |= Update::LAYOUT | Update::DRAW;
    }

    /// Request a draw of the view tree.
    pub fn request_draw(&mut self) {
        self.update |= Update::DRAW;
    }

    /// Get whether the view needs to be laid out.
    pub fn needs_layout(&self) -> bool {
        self.update.contains(Update::LAYOUT)
    }

    /// Get whether the view needs to be drawn.
    pub fn needs_draw(&self) -> bool {
        self.update.contains(Update::DRAW)
    }

    /// Mark the view as laid out.
    ///
    /// This will remove the [`Update::LAYOUT`] flag.
    pub fn layed_out(&mut self) {
        self.update.remove(Update::LAYOUT);
    }

    /// Mark the view as drawn.
    ///
    /// This will remove the [`Update::DRAW`] flag.
    pub fn drawn(&mut self) {
        self.update.remove(Update::DRAW);
    }

    /// Get the [`Update`] of the view.
    pub fn update(&self) -> Update {
        self.update
    }
}
