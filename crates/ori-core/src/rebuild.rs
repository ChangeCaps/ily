pub use ori_macro::Rebuild;

use crate::RebuildCx;

/// A convenience trait for rebuilding a [`View`](crate::View).
///
/// When derived this will detect changes in the fields of the struct and
/// request a rebuild, layout or draw when necessary. This is done by
/// specifying the `#[rebuild(...)]` attribute on the fields of the struct.
/// Valid values are `tree`, `layout` and `draw`.
///
/// # Example
/// ``` ignore
/// #[derive(Rebuild)]
/// struct MyView {
///     #[rebuild(layout)]
///     size: f32,
///     #[rebuild(draw)]
///     color: Color,
/// }
/// ```
pub trait Rebuild {
    /// Rebuild the view.
    fn rebuild(&self, cx: &mut RebuildCx, old: &Self);
}
