use ori_core::{
    command::{CommandProxy, CommandWaker},
    context::Contexts,
    style::{IntoStyles, Styles},
    text::{FontSource, Fonts},
    view::{any, AnyView},
    window::WindowDescriptor,
};

use crate::{App, AppRequest, Delegate, UiBuilder};

/// A builder for an [`App`].
pub struct AppBuilder<T> {
    delegates: Vec<Box<dyn Delegate<T>>>,
    requests: Vec<AppRequest<T>>,
    style: Styles,
    fonts: Fonts,
}

impl<T> Default for AppBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AppBuilder<T> {
    /// Create a new application builder.
    pub fn new() -> Self {
        Self {
            delegates: Vec::new(),
            requests: Vec::new(),
            style: Styles::new(),
            fonts: Fonts::new(),
        }
    }

    /// Add a delegate to the application.
    pub fn delegate(mut self, delegate: impl Delegate<T> + 'static) -> Self {
        self.delegates.push(Box::new(delegate));
        self
    }

    /// Add a style to the application.
    pub fn style(mut self, style: impl IntoStyles) -> Self {
        self.style.set(style);
        self
    }

    /// Add a style builder to the application.
    pub fn build_style<U: 'static>(mut self, builder: impl Fn(&Styles) -> U + 'static) -> Self {
        self.style.builder(builder);
        self
    }

    /// Add a font to the application.
    pub fn font(mut self, font: impl Into<FontSource>) -> Self {
        if let Err(err) = self.fonts.load_font(font) {
            eprintln!("Failed to load font: {}", err);
        }

        self
    }

    /// Add a window to the application.
    pub fn window<V: AnyView<T> + 'static>(
        mut self,
        descriptor: WindowDescriptor,
        mut ui: impl FnMut(&mut T) -> V + 'static,
    ) -> Self {
        let builder: UiBuilder<T> = Box::new(move |data| any(ui(data)));
        (self.requests).push(AppRequest::OpenWindow(descriptor, builder));
        self
    }

    /// Build the application.
    pub fn build(self, waker: CommandWaker) -> App<T> {
        let (proxy, receiver) = CommandProxy::new(waker);

        let mut contexts = Contexts::new();
        contexts.insert(self.fonts);

        App {
            windows: Default::default(),
            modifiers: Default::default(),
            delegates: self.delegates,
            proxy,
            receiver,
            style: self.style,
            requests: self.requests,
            contexts,
        }
    }
}
