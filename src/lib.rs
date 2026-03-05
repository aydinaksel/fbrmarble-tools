pub mod app;

#[cfg(feature = "ssr")]
pub mod database;

#[cfg(feature = "ssr")]
pub mod pdf;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;
    use leptos::prelude::*;

    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
