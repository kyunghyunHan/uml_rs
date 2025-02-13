use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};
mod pages;
use pages::{
    main::{__StoryGallery, Category, Profile},
    recipes::__Recipes,
    users::__Favorites,
};
#[derive(Clone)]
struct AppState {
    selected_items: RwSignal<Vec<Profile>>, // 선택된 프로필들
    selected_category: RwSignal<Option<Category>>, // 현재 선택된 카테고리 (단일 선택)
    select_profile_action: Action<Profile, ()>,
}
#[component]
pub fn App() -> impl IntoView {
    let selected_items = create_rw_signal(vec![]);
    let selected_category = create_rw_signal(None);
    let select_profile_action: Action<Profile, ()> = Action::new(move |profile: &Profile| {
        let profile = profile.clone();
        let items = selected_items;

        async move {
            let mut current = items.get();
            if let Some(pos) = current.iter().position(|x: &Profile| x.id == profile.id) {
                current.remove(pos);
            } else {
                current.push(profile);
            }
            items.set(current);
            ()
        }
    });
    let state = AppState {
        selected_items,
        selected_category,
        select_profile_action,
    };
    provide_context(state);

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
        <Route path=path!("/recipes") view=__Recipes/> // 새로운 라우트 추가
      </Routes>
        </main>
        <div class="fixed bottom-0 left-0 right-0 bg-white border-t flex justify-around p-4">
        <A href="/users">
            <button class="flex flex-col items-center text-blue-500">
                <span class="text-xs mt-1">"나의 냉장고"</span>
            </button>
        </A>
        <button class="flex flex-col items-center">
            <span class="text-xs mt-1">"모든 레시피"</span>
        </button>
        <button class="flex flex-col items-center">
            <span class="text-xs mt-1">"즐겨찾기"</span>
        </button>
    </div>
        </div>
      </Router>
    }
}
fn main() {
    mount_to_body(|| view! { <App/> })
}
