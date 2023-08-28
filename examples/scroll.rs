use ori::prelude::*;

#[derive(Default)]
struct Data {}

fn square(index: usize) -> impl View<Data> {
    size(
        100.0,
        button(text("Click me"))
            .color(style(Palette::SECONDARY))
            .on_press(move |_, _| {
                info!("clicked {}", index);
            }),
    )
}

fn app(_data: &mut Data) -> impl View<Data> {
    let scroll = height(
        400.0,
        vscroll(vstack![
            square(0),
            square(1),
            square(2),
            square(3),
            square(4),
            square(5),
            square(6),
            square(7),
            square(8)
        ]),
    );

    let button = button(text("hello"))
        .on_press(|_, _| {
            info!("hello");
        })
        .fancy(4.0);

    center(overlay![scroll, pad(em(0.5), bottom_right(button))])
}

fn main() {
    App::new(app, Data::default())
        .title("Scroll (examples/scroll.rs)")
        .run()
}
