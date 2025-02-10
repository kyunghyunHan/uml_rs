use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};

#[component]
fn Favorites() -> impl IntoView {
    view! {
        <div class="w-full max-w-2xl mx-auto bg-white">
            <h1 class="text-xl font-bold p-4">"즐겨찾기"</h1>
        </div>
    }
}
