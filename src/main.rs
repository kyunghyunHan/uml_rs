use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};
mod pages;
use pages::{main::__StoryGallery, users::__Favorites};

#[component]
pub fn App() -> impl IntoView {
    view! {
      <Router>
      <div class="w-full max-w-2xl mx-auto bg-white">

        <nav>
        <div class="flex justify-between items-center p-4 border-b">
        <h1 class="text-xl font-bold">"나의 냉장고"</h1>
        <div class="flex items-center space-x-4">
            <button class="p-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/>
                </svg>
            </button>
         </div>
        </div>
        </nav>
        <main>
        <Routes fallback=|| "Not found.">
        <Route path=path!("/") view=__StoryGallery/>
        <Route path=path!("/users") view=__Favorites/>
      </Routes>
        </main>
        </div>
      </Router>
    }
}
fn main() {
    mount_to_body(|| view! { <App/> })
}
