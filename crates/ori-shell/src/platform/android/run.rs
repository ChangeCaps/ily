use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, Keycode, MotionAction, MotionEvent},
    AndroidApp, AndroidAppWaker, InputStatus, MainEvent, PollEvent,
};
use ori_app::{App, AppBuilder, AppRequest, UiBuilder};
use ori_core::{
    clipboard::Clipboard,
    command::CommandWaker,
    event::{Key, PointerButton, PointerId},
    layout::{Point, Size},
    text::Fonts,
    window::{Window, WindowId, WindowUpdate},
};
use ori_skia::{SkiaFonts, SkiaRenderer};
use tracing::warn;

use crate::platform::egl::{EglContext, EglNativeDisplay, EglSurface};

use super::{
    clipboard::AndroidClipboard,
    keyboard::{ImeEvent, ImeState},
    AndroidError, ANDROID_APP,
};

/// Run the app on Android.
pub fn run<T>(app: AppBuilder<T>, data: &mut T) -> Result<(), AndroidError> {
    let android = ANDROID_APP.get().ok_or(AndroidError::NotInitialized)?;

    let waker = CommandWaker::new({
        let waker = android.create_waker();

        move || {
            waker.wake();
        }
    });

    let egl_context = EglContext::new(EglNativeDisplay::Android).unwrap();

    let fonts = Box::new(SkiaFonts::new(Some("Roboto")));

    let mut app = app.build(waker, fonts);

    app.add_context(Clipboard::new(Box::new(AndroidClipboard {
        app: android.clone(),
    })));

    let mut state = AppState {
        running: true,
        app,
        android: android.clone(),
        waker: android.create_waker(),
        egl_context,
        ime_state: ImeState::default(),
        window: None,
        combining: None,
    };

    let mut init = false;

    while state.running {
        android.poll_events(None, |event| {
            match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(event) => match event {
                    MainEvent::ConfigChanged { .. } => {}
                    MainEvent::ContentRectChanged { .. } => {}
                    MainEvent::Destroy => {
                        state.running = false;
                    }
                    MainEvent::GainedFocus => {}
                    MainEvent::InitWindow { .. } => {
                        if !init {
                            state.app.init(data);
                            init = true;
                        } else {
                            recreate_window(&mut state);
                        }
                    }
                    MainEvent::InputAvailable => {
                        request_redraw(&mut state);
                    }
                    MainEvent::InsetsChanged { .. } => {}
                    MainEvent::LostFocus => {}
                    MainEvent::LowMemory => {}
                    MainEvent::Pause => {}
                    MainEvent::RedrawNeeded { .. } => {
                        request_redraw(&mut state);
                    }
                    MainEvent::Resume { .. } => {}
                    MainEvent::SaveState { .. } => {}
                    MainEvent::Start => {}
                    MainEvent::Stop => {}
                    MainEvent::TerminateWindow { .. } => {}
                    MainEvent::WindowResized { .. } => {
                        window_resized(&mut state, data);
                        request_redraw(&mut state);
                    }
                    _ => {}
                },
                _ => {}
            }

            if init {
                state.app.handle_commands(data);
                handle_requests(&mut state, data);

                handle_input_events(&mut state, &android, data);
                handle_ime_events(&mut state, data);

                render_window(&mut state, data);
                handle_requests(&mut state, data);

                state.app.idle(data);
                handle_requests(&mut state, data);

                if matches!(
                    state.window,
                    Some(WindowState {
                        needs_redraw: true,
                        ..
                    })
                ) {
                    state.waker.wake();
                }
            }
        });
    }

    Ok(())
}

struct AppState<T> {
    running: bool,
    app: App<T>,
    android: AndroidApp,
    waker: AndroidAppWaker,
    egl_context: EglContext,
    ime_state: ImeState,
    window: Option<WindowState>,
    combining: Option<char>,
}

struct WindowState {
    id: WindowId,
    physical_width: u32,
    physical_height: u32,
    scale_factor: f32,
    needs_redraw: bool,
    egl_surface: EglSurface,
    renderer: SkiaRenderer,
}

fn handle_input_events<T>(state: &mut AppState<T>, android: &AndroidApp, data: &mut T) {
    let mut inputs = android.input_events_iter().unwrap();

    loop {
        if !inputs.next(|event| input_event(state, data, event)) {
            break;
        }

        handle_requests(state, data);
    }
}

fn handle_ime_events<T>(state: &mut AppState<T>, data: &mut T) {
    while let Some(event) = state.ime_state.next_event() {
        if let Some(ref window) = state.window {
            match event {
                ImeEvent::CommitText(commit) => {
                    (state.app).keyboard_key(
                        data,
                        window.id,
                        Key::Unidentified,
                        None,
                        Some(commit),
                        true,
                    );
                }
                ImeEvent::DeleteSurroundingText(_before, _after) => {
                    (state.app).keyboard_key(data, window.id, Key::Backspace, None, None, true);
                }
            }
        }

        handle_requests(state, data);
    }
}

fn handle_requests<T>(state: &mut AppState<T>, data: &mut T) {
    for request in state.app.take_requests() {
        handle_request(state, data, request);
    }
}

fn handle_request<T>(state: &mut AppState<T>, data: &mut T, request: AppRequest<T>) {
    match request {
        AppRequest::OpenWindow(window, ui) => create_window(state, data, window, ui),
        AppRequest::CloseWindow(_) => {
            state.running = false;
        }
        AppRequest::DragWindow(_) => {
            warn!("Dragging windows is not supported on Android");
        }
        AppRequest::RequestRedraw(_) => request_redraw(state),
        AppRequest::UpdateWindow(_, update) => match update {
            WindowUpdate::Title(_) => warn!("Window title is not supported on Android"),
            WindowUpdate::Icon(_) => warn!("Window icon is not supported on Android"),
            WindowUpdate::Size(_) => warn!("Window size is not supported on Android"),
            WindowUpdate::Scale(_) => warn!("Window scale is not supported on Android"),
            WindowUpdate::Resizable(_) => warn!("Window resizable is not supported on Android"),
            WindowUpdate::Decorated(_) => warn!("Window decorated is not supported on Android"),
            WindowUpdate::Maximized(_) => warn!("Window maximized is not supported on Android"),
            WindowUpdate::Visible(_) => warn!("Window visible is not supported on Android"),
            WindowUpdate::Color(_) => warn!("Window color is not supported on Android"),
            WindowUpdate::Cursor(_) => warn!("Window cursor is not supported on Android"),
            WindowUpdate::Ime(ime) => match ime {
                Some(ime) => {
                    state.ime_state.show(&state.android).unwrap();
                    state.ime_state.set(&state.android, ime).unwrap();
                }
                None => {
                    state.ime_state.hide(&state.android).unwrap();
                }
            },
        },
        AppRequest::Quit => {
            state.running = false;
        }
    }
}

fn create_window<T>(state: &mut AppState<T>, data: &mut T, mut window: Window, ui: UiBuilder<T>) {
    if state.window.is_some() {
        warn!("Only one window is supported on Android");
        return;
    }

    let native_window = state.android.native_window().unwrap();

    let physical_width = native_window.width() as u32;
    let physical_height = native_window.height() as u32;

    // the scale factor in DPI
    let scale_factor = state.android.config().density().unwrap_or(160) as f32;
    let scale_factor = scale_factor / 160.0;

    window.size = Size::new(physical_width as f32, physical_height as f32) / scale_factor;
    window.scale = scale_factor;

    let native_window_ptr = native_window.ptr().as_ptr();
    let egl_surface = EglSurface::new(&state.egl_context, native_window_ptr as _).unwrap();

    egl_surface.make_current().unwrap();
    egl_surface.swap_interval(1).unwrap();

    let renderer = unsafe { SkiaRenderer::new(|name| state.egl_context.get_proc_address(name)) };

    let window_state = WindowState {
        id: window.id(),
        physical_width,
        physical_height,
        scale_factor,
        needs_redraw: true,
        egl_surface,
        renderer,
    };

    state.window = Some(window_state);
    state.app.add_window(data, ui, window);
}

fn recreate_window<T>(state: &mut AppState<T>) {
    if let Some(window) = state.window.take() {
        let native_window = state.android.native_window().unwrap();

        let physical_width = native_window.width() as u32;
        let physical_height = native_window.height() as u32;

        let scale_factor = state.android.config().density().unwrap_or(160) as f32;
        let scale_factor = scale_factor / 160.0;

        let native_window_ptr = native_window.ptr().as_ptr();
        let egl_surface = EglSurface::new(&state.egl_context, native_window_ptr as _).unwrap();

        egl_surface.make_current().unwrap();
        egl_surface.swap_interval(1).unwrap();

        let renderer = unsafe {
            // SAFETY: The EGL context is current
            SkiaRenderer::new(|name| state.egl_context.get_proc_address(name))
        };

        let window_state = WindowState {
            id: window.id,
            physical_width,
            physical_height,
            scale_factor,
            needs_redraw: true,
            egl_surface,
            renderer,
        };

        state.window = Some(window_state);
    }
}

fn render_window<T>(state: &mut AppState<T>, data: &mut T) {
    if let Some(ref mut window) = state.window {
        if !window.needs_redraw {
            return;
        }

        window.needs_redraw = false;

        if let Some(draw) = state.app.draw_window(data, window.id) {
            window.egl_surface.make_current().unwrap();

            let fonts = state.app.contexts.get_mut::<Box<dyn Fonts>>().unwrap();

            window.renderer.render(
                fonts.downcast_mut().unwrap(),
                &draw.canvas,
                draw.clear_color,
                window.physical_width,
                window.physical_height,
                window.scale_factor,
            );

            window.egl_surface.swap_buffers().unwrap();
        }
    }
}

fn request_redraw<T>(state: &mut AppState<T>) {
    if let Some(ref mut window) = state.window {
        window.needs_redraw = true;
    }
}

fn window_resized<T>(state: &mut AppState<T>, data: &mut T) {
    if let Some(ref mut window) = state.window {
        let native_window = state.android.native_window().unwrap();

        window.physical_width = native_window.width() as u32;
        window.physical_height = native_window.height() as u32;

        state.app.window_resized(
            data,
            window.id,
            (window.physical_width as f32 / window.scale_factor) as u32,
            (window.physical_height as f32 / window.scale_factor) as u32,
        );
    }
}

fn input_event<T>(state: &mut AppState<T>, data: &mut T, event: &InputEvent) -> InputStatus {
    match event {
        InputEvent::MotionEvent(event) => match motion_event(state, data, event) {
            true => InputStatus::Handled,
            false => InputStatus::Unhandled,
        },
        InputEvent::KeyEvent(event) => match key_event(state, data, event) {
            true => InputStatus::Handled,
            false => InputStatus::Unhandled,
        },
        InputEvent::TextEvent(_) => InputStatus::Unhandled,
        _ => InputStatus::Unhandled,
    }
}

fn motion_event<T>(state: &mut AppState<T>, data: &mut T, event: &MotionEvent) -> bool {
    let Some(ref mut window) = state.window else {
        return false;
    };

    let [b0, b1, b2, b3] = event.device_id().to_le_bytes();
    let [b4, b5, b6, b7] = (event.pointer_index() as u32).to_le_bytes();
    let bytes = [b0, b1, b2, b3, b4, b5, b6, b7];
    let pointer_id = PointerId::from_u64(u64::from_le_bytes(bytes));

    let pointer = event.pointer_at_index(event.pointer_index());
    let point = Point::new(pointer.x(), pointer.y()) / window.scale_factor;

    match event.action() {
        MotionAction::Down | MotionAction::Up => {
            let pressed = matches!(event.action(), MotionAction::Down);

            let mut handled = false;

            if pressed {
                handled |= state.app.pointer_moved(data, window.id, pointer_id, point);
            }

            handled |= state.app.pointer_button(
                data,
                window.id,
                pointer_id,
                PointerButton::Primary,
                pressed,
            );

            if !pressed {
                handled |= state.app.pointer_left(data, window.id, pointer_id);
            }

            handled
        }
        MotionAction::Move => state.app.pointer_moved(data, window.id, pointer_id, point),
        _ => false,
    }
}

fn key_event<T>(state: &mut AppState<T>, data: &mut T, event: &KeyEvent) -> bool {
    let Some(ref mut window) = state.window else {
        return false;
    };

    let window_id = window.id;
    let pressed = matches!(event.action(), KeyAction::Down);

    let keychar = get_key_event_keychar(state, event);
    let logical = to_logical(keychar, event.key_code());
    let text = logical.as_char().map(String::from);

    (state.app).keyboard_key(data, window_id, logical, None, text, pressed)
}

fn get_key_event_keychar<T>(state: &mut AppState<T>, event: &KeyEvent) -> Option<KeyMapChar> {
    let device_id = event.device_id();

    let Ok(keymap) = state.android.device_key_character_map(device_id) else {
        warn!("Failed to get key character map");
        return None;
    };

    let keycode = event.key_code();
    let Ok(keymapchar) = keymap.get(keycode, event.meta_state()) else {
        warn!("Failed to get key code");
        return None;
    };

    match keymapchar {
        KeyMapChar::Unicode(unicode) => {
            if event.action() == KeyAction::Down {
                return Some(KeyMapChar::Unicode(unicode));
            }

            let combined = match state.combining {
                Some(accent) => match keymap.get_dead_char(accent, unicode) {
                    Ok(Some(key)) => Some(key),
                    Ok(None) => None,
                    Err(err) => {
                        warn!("Failed to get dead char: {:?}", err);
                        None
                    }
                },
                None => Some(unicode),
            };

            state.combining = None;
            combined.map(KeyMapChar::Unicode)
        }
        KeyMapChar::CombiningAccent(accent) => {
            if event.action() == KeyAction::Down {
                state.combining = Some(accent);
            }

            Some(KeyMapChar::CombiningAccent(accent))
        }
        KeyMapChar::None => None,
    }
}

fn to_logical(keychar: Option<KeyMapChar>, keycode: Keycode) -> Key {
    use Keycode::*;

    match keychar {
        Some(KeyMapChar::Unicode(unicode)) if !unicode.is_control() => Key::Character(unicode),
        Some(KeyMapChar::CombiningAccent(_)) => Key::Dead,
        _ => match keycode {
            Keycode0 => Key::Character('0'),
            Keycode1 => Key::Character('1'),
            Keycode2 => Key::Character('2'),
            Keycode3 => Key::Character('3'),
            Keycode4 => Key::Character('4'),
            Keycode5 => Key::Character('5'),
            Keycode6 => Key::Character('6'),
            Keycode7 => Key::Character('7'),
            Keycode8 => Key::Character('8'),
            Keycode9 => Key::Character('9'),
            A => Key::Character('a'),
            B => Key::Character('b'),
            C => Key::Character('c'),
            D => Key::Character('d'),
            E => Key::Character('e'),
            F => Key::Character('f'),
            G => Key::Character('g'),
            H => Key::Character('h'),
            I => Key::Character('i'),
            J => Key::Character('j'),
            K => Key::Character('k'),
            L => Key::Character('l'),
            M => Key::Character('m'),
            N => Key::Character('n'),
            O => Key::Character('o'),
            P => Key::Character('p'),
            Q => Key::Character('q'),
            R => Key::Character('r'),
            S => Key::Character('s'),
            T => Key::Character('t'),
            U => Key::Character('u'),
            V => Key::Character('v'),
            W => Key::Character('w'),
            X => Key::Character('x'),
            Y => Key::Character('y'),
            Z => Key::Character('z'),
            Comma => Key::Character(','),
            Period => Key::Character('.'),
            Grave => Key::Character('`'),
            Minus => Key::Character('-'),
            Equals => Key::Character('='),
            LeftBracket => Key::Character('['),
            RightBracket => Key::Character(']'),
            Backslash => Key::Character('\\'),
            Semicolon => Key::Character(';'),
            Apostrophe => Key::Character('\''),
            Slash => Key::Character('/'),
            At => Key::Character('@'),
            Plus => Key::Character('+'),

            DpadUp => Key::Up,
            DpadDown => Key::Down,
            DpadLeft => Key::Left,
            DpadRight => Key::Right,
            DpadCenter => Key::Enter,

            Clear => Key::Clear,

            AltLeft => Key::Alt,
            AltRight => Key::Alt,
            ShiftLeft => Key::Shift,
            ShiftRight => Key::Shift,
            Tab => Key::Tab,
            Space => Key::Space,
            Sym => Key::Symbol,
            Enter => Key::Enter,
            Del => Key::Backspace,

            Num => Key::Alt,

            PageUp => Key::PageUp,
            PageDown => Key::PageDown,

            Escape => Key::Escape,
            ForwardDel => Key::Delete,
            CtrlLeft => Key::Control,
            CtrlRight => Key::Control,
            CapsLock => Key::CapsLock,
            ScrollLock => Key::ScrollLock,
            MetaLeft => Key::Meta,
            MetaRight => Key::Meta,
            Function => Key::Fn,
            Sysrq => Key::PrintScreen,
            Break => Key::Pause,
            MoveHome => Key::Home,
            MoveEnd => Key::End,
            Insert => Key::Insert,

            F1 => Key::F1,
            F2 => Key::F2,
            F3 => Key::F3,
            F4 => Key::F4,
            F5 => Key::F5,
            F6 => Key::F6,
            F7 => Key::F7,
            F8 => Key::F8,
            F9 => Key::F9,
            F10 => Key::F10,
            F11 => Key::F11,
            F12 => Key::F12,

            NumLock => Key::NumLock,
            Numpad0 => Key::Character('0'),
            Numpad1 => Key::Character('1'),
            Numpad2 => Key::Character('2'),
            Numpad3 => Key::Character('3'),
            Numpad4 => Key::Character('4'),
            Numpad5 => Key::Character('5'),
            Numpad6 => Key::Character('6'),
            Numpad7 => Key::Character('7'),
            Numpad8 => Key::Character('8'),
            Numpad9 => Key::Character('9'),
            NumpadDivide => Key::Character('/'),
            NumpadMultiply => Key::Character('*'),
            NumpadSubtract => Key::Character('-'),
            NumpadAdd => Key::Character('+'),
            NumpadDot => Key::Character('.'),
            NumpadComma => Key::Character(','),
            NumpadEnter => Key::Enter,
            NumpadEquals => Key::Character('='),
            NumpadLeftParen => Key::Character('('),
            NumpadRightParen => Key::Character(')'),

            _ => Key::Unidentified,
        },
    }
}
