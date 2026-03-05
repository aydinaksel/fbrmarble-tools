use leptos::ev::Event;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes, A},
    path,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options/>
                <MetaTags/>
            </head>
            <body class="bg-gray-50 text-gray-900 min-h-screen">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="FBR Marble Tools"/>
        <Nav/>
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/") view=Dashboard/>
                <Route path=path!("/crate-labels") view=CrateLabelsPage/>
            </Routes>
        </Router>
    }
}

#[component]
fn Nav() -> impl IntoView {
    view! {
        <nav class="border-b border-gray-200 bg-white">
            <div class="mx-auto max-w-5xl px-6 flex items-center gap-6 h-14">
                <A href="/" attr:class="font-semibold text-gray-900 text-sm tracking-tight">
                    "FBR Marble"
                </A>
                <div class="flex items-center gap-4 text-sm text-gray-500">
                    <A href="/crate-labels" attr:class="hover:text-gray-900">"Crate Labels"</A>
                </div>
            </div>
        </nav>
    }
}

#[component]
fn Dashboard() -> impl IntoView {
    view! {
        <main class="mx-auto max-w-5xl px-6 py-12">
            <h1 class="text-2xl font-bold tracking-tight mb-1">"Internal Tools"</h1>
            <p class="text-sm text-gray-500 mb-10">"FBR Marble operations"</p>

            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                <ToolCard
                    href="/crate-labels"
                    title="Crate Label Generator"
                    description="Generate PDF crate labels from a purchase order number."
                />
            </div>
        </main>
    }
}

#[component]
fn ToolCard(href: &'static str, title: &'static str, description: &'static str) -> impl IntoView {
    view! {
        <A href=href attr:class="block rounded-lg border border-gray-200 bg-white p-5 hover:border-gray-300 hover:shadow-sm transition-all">
            <h2 class="font-semibold text-gray-900 text-sm mb-1">{title}</h2>
            <p class="text-xs text-gray-500 leading-relaxed">{description}</p>
        </A>
    }
}

#[component]
fn CrateLabelsPage() -> impl IntoView {
    let po_number = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);

    let handle_submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let po = po_number.get();
        if po.is_empty() {
            return;
        }

        loading.set(true);
        error_message.set(None);

        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async move {
                match fetch_and_open_pdf(po).await {
                    Ok(()) => {}
                    Err(message) => error_message.set(Some(message)),
                }
                loading.set(false);
            });
        }
    };

    view! {
        <main class="mx-auto max-w-lg px-6 py-12">
            <h1 class="text-xl font-bold tracking-tight mb-1">"Crate Label Generator"</h1>
            <p class="text-sm text-gray-500 mb-8">
                "Generates a PDF with one label per crate for each line on the purchase order."
            </p>

            <form on:submit=handle_submit class="space-y-4">
                <div>
                    <label
                        for="po-number-input"
                        class="block text-sm font-medium text-gray-700 mb-1"
                    >
                        "Purchase Order Number"
                    </label>
                    <input
                        id="po-number-input"
                        type="text"
                        inputmode="numeric"
                        pattern="[0-9]+"
                        maxlength="11"
                        autocomplete="off"
                        required
                        placeholder="e.g. 983892"
                        class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm shadow-sm placeholder:text-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
                        prop:value=po_number
                        on:input=move |event: Event| {
                            po_number.set(event_target_value(&event));
                        }
                    />
                </div>

                <button
                    type="submit"
                    disabled=loading
                    class="w-full rounded-md bg-blue-600 px-4 py-2 text-sm font-semibold text-white shadow-sm hover:bg-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-600 focus:ring-offset-2 disabled:opacity-60 disabled:cursor-not-allowed"
                >
                    {move || if loading.get() { "Generating\u{2026}" } else { "Generate PDF" }}
                </button>
            </form>

            {move || {
                error_message.get().map(|message| view! {
                    <div
                        role="alert"
                        class="mt-4 rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700"
                    >
                        <strong>"Error: "</strong>
                        {message}
                    </div>
                })
            }}
        </main>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="mx-auto max-w-lg px-6 py-12 text-center">
            <h1 class="text-2xl font-bold">"404 — Page not found"</h1>
        </main>
    }
}

#[cfg(feature = "hydrate")]
async fn fetch_and_open_pdf(po_number: String) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::window;

    let win = window().ok_or_else(|| "No window object".to_string())?;
    let url = format!("/api/crate-labels/{}", po_number);

    let response_value = JsFuture::from(win.fetch_with_str(&url))
        .await
        .map_err(|error| format!("Fetch failed: {:?}", error))?;
    let response: web_sys::Response = response_value.dyn_into().unwrap();

    if !response.ok() {
        let text_value = JsFuture::from(
            response.text().map_err(|error| format!("{:?}", error))?,
        )
        .await
        .map_err(|error| format!("{:?}", error))?;
        let text = text_value
            .as_string()
            .unwrap_or_else(|| format!("HTTP {}", response.status()));
        return Err(text.trim().to_string());
    }

    let blob_value = JsFuture::from(
        response.blob().map_err(|error| format!("{:?}", error))?,
    )
    .await
    .map_err(|error| format!("{:?}", error))?;
    let blob: web_sys::Blob = blob_value.dyn_into().unwrap();
    let object_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|error| format!("{:?}", error))?;
    win.open_with_url_and_target(&object_url, "_blank")
        .map_err(|error| format!("{:?}", error))?;

    Ok(())
}
