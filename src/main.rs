use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};
mod pages;
use leptos::prelude::create_action;
use pages::{
    main::{__StoryGallery, Category},
    users::__Favorites,
};
#[derive(Clone)]
enum CategoryAction {
    Add(Category),
    Remove(Category),
    Clear,
}

// 전역 상태를 위한 컨텍스트 타입 정의
#[derive(Clone)]
struct CategoryState {
    selected: RwSignal<Vec<Category>>,
    action: Action<CategoryAction, ()>,
}
#[component]
pub fn App() -> impl IntoView {
    let selected_categories = create_rw_signal(vec![]);

    // Action 생성 - input을 참조로 받고 내부에서 clone
    let category_action = Action::new(move |input: &CategoryAction| {
        let action = input.clone(); // clone the input for the async block
        let selected = selected_categories; // capture the signal

        async move {
            let mut current = selected.get();
            match action {
                CategoryAction::Add(category) => {
                    if !current.contains(&category) {
                        current.push(category);
                    }
                }
                CategoryAction::Remove(category) => {
                    if let Some(pos) = current.iter().position(|x| x == &category) {
                        current.remove(pos);
                    }
                }
                CategoryAction::Clear => current.clear(),
            }
            selected.set(current);
        }
    });
    // 상태와 액션을 context로 제공
    let state = CategoryState {
        selected: selected_categories,
        action: category_action,
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
      </Routes>
        </main>
        </div>
      </Router>
    }
}
fn main() {
    mount_to_body(|| view! { <App/> })
}
