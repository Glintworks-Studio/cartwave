//! Probe: which `shadow` declarations does vizia 0.4 actually render?
use vizia::prelude::*;

const CSS: &str = r#"
:root { background-color: #16181F; }
.box { width: 140px; height: 50px; background-color: #0A0B0E; corner-radius: 6px; }
.a { shadow: 0px 0px 0px 4px red; }
.b { shadow: 2px 3px 6px 0px rgba(0, 0, 0, 0.95) inset; background-color: #444; }
.c { shadow: 2px 3px 6px 0px red inset; background-color: #444; }
.d { shadow: 0px 0px 12px 2px rgba(124, 92, 255, 0.8), 0px 0px 0px 1px red inset; }
.e { shadow: inset 2px 3px 6px red; background-color: #444; }
.f { shadow: 2px 3px 6px red; }
"#;

fn main() -> Result<(), ApplicationError> {
    Application::new(|cx| {
        cx.add_stylesheet(CSS).unwrap();
        VStack::new(cx, |cx| {
            for c in ["a", "b", "c", "d", "e", "f"] {
                HStack::new(cx, |cx| {
                    Label::new(cx, c).width(Pixels(30.0));
                    Element::new(cx).class("box").class(c);
                })
                .height(Auto)
                .gap(Pixels(20.0));
            }
        })
        .padding(Pixels(30.0))
        .gap(Pixels(24.0));
    })
    .title("probe")
    .inner_size((400, 520))
    .run()
}
