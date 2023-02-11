use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn main() -> Result<(), JsValue> {
    log("Hello, world!");

    let window = web_sys::window().expect_throw("no global window");
    let document = window.document().expect_throw("no window document");
    let body = document.body().expect_throw("no document body");

    let p = document.create_element("p")?;
    p.set_text_content(Some("Hello, world!"));

    body.append_child(&p)?;

    Ok(())
}
