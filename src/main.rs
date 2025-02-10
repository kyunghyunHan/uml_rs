use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
enum Category {
    Vegetable, // 채소
    Meat,      // 고기
    Seafood,   // 해산물
    Dairy,     // 계란/유제품
    Fruit,     // 과일
    Grain,     // 곡물
    Sauce,     // 양념/소스
    Drink,     // 음료
    Other,     // 기타
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Profile {
    id: i32,
    username: String,
    avatar: String,
    categories: Vec<Category>,
    posts: Vec<String>,
    added_date: String,
}

#[component]
fn StoryGallery() -> impl IntoView {
    let (selected_profile, set_selected_profile) = create_signal::<Option<Profile>>(None);
    let (selected_categories, set_selected_categories) = create_signal::<Vec<Category>>(vec![]);

    let profiles = vec![
        Profile {
            id: 1,
            username: "시금치".to_string(),
            avatar: "/img/시금치.jpg".to_string(),
            categories: vec![Category::Vegetable],
            posts: vec!["/api/placeholder/300/400".to_string()],
            added_date: "25.02.10".to_string(),
        },
        Profile {
            id: 2,
            username: "닭가슴살".to_string(),
            avatar: "/img/닭가슴살.jpg".to_string(),
            categories: vec![Category::Meat],
            posts: vec!["/api/placeholder/300/400".to_string()],
            added_date: "25.02.10".to_string(),
        },
        Profile {
            id: 3,
            username: "연어".to_string(),
            avatar: "/img/연어.jpg".to_string(),
            categories: vec![Category::Seafood, Category::Meat],
            posts: vec!["/api/placeholder/300/400".to_string()],
            added_date: "25.02.10".to_string(),
        },
    ];

    let filtered_profiles = move || {
        let categories = selected_categories.get();
        if categories.is_empty() {
            profiles.clone()
        } else {
            profiles
                .clone()
                .into_iter()
                .filter(|profile| {
                    profile
                        .categories
                        .iter()
                        .any(|cat| categories.contains(cat))
                })
                .collect()
        }
    };

    let toggle_category = move |category: Category| {
        let mut current = selected_categories.get();
        if let Some(pos) = current.iter().position(|x| x == &category) {
            current.remove(pos);
        } else {
            current.push(category);
        }
        set_selected_categories.set(current); // 수정된 부분
    };

    view! {
        <div class="w-full max-w-2xl mx-auto bg-white">
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

            <div class="overflow-x-auto">
                <div class="flex space-x-4 p-4 min-w-max">
                    {[
                        (Category::Vegetable, "채소", "bg-green-100", "/img/category/vegetables.png"),
                        (Category::Meat, "고기", "bg-red-100", "/img/category/meat.png"),
                        (Category::Seafood, "해산물", "bg-blue-100", "/img/category/seafood.png"),
                        (Category::Dairy, "계란/유제품", "bg-yellow-100", "/img/category/eggs.png"),
                        (Category::Fruit, "과일", "bg-purple-100", "/img/category/fruits.png"),
                        (Category::Grain, "곡물", "bg-indigo-100", "/img/category/grains.png"),
                        (Category::Sauce, "양념/소스", "bg-pink-100", "/img/category/sauce.png"),
                        (Category::Drink, "음료", "bg-gray-100", "/img/category/drinks.png"),
                        (Category::Other, "기타", "bg-orange-100", "/img/category/others.png"),
                    ].into_iter().map(|(category, label, bg_color, icon)| {
                        let category_clone = category.clone();
                        let is_selected = move || selected_categories.get().contains(&category);
                        view! {
                            <button
                                class=move || format!("flex flex-col items-center {}", if is_selected() { "opacity-100" } else { "opacity-50" })
                                on:click=move |_| toggle_category(category_clone.clone())
                            >
                                <div class=format!("w-16 h-16 rounded-full {} flex items-center justify-center mb-1", bg_color)>
                                    <img
                                        src=icon
                                        alt=label
                                        class="w-8 h-8"
                                    />
                                </div>
                                <span class="text-xs">{label}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="grid grid-cols-2 gap-4 p-4">
                {move || filtered_profiles().into_iter().map(|profile| {
                    let profile_clone = profile.clone();
                    view! {
                        <button
                            class="bg-white rounded-lg shadow p-4 flex flex-col items-center"
                        >
                            <div class="w-16 h-16 mb-2">
                                <img
                                    src={profile.avatar}
                                    alt={profile.username.clone()}
                                    class="w-full h-full object-contain"
                                />
                            </div>
                            <span class="text-sm font-medium text-gray-900">{profile.username}</span>
                            <span class="text-xs text-gray-500">{profile.added_date}</span>
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="fixed bottom-16 left-0 right-0 p-4 bg-white border-t">
                <button class="w-full py-3 bg-red-500 text-white rounded-lg text-center">
                    "냉장고 파먹기"
                </button>
            </div>

            <div class="fixed bottom-0 left-0 right-0 bg-white border-t flex justify-around p-4">
                <button class="flex flex-col items-center text-blue-500">
                    <span class="text-xs mt-1">"나의 냉장고"</span>
                </button>
                <button class="flex flex-col items-center">
                    <span class="text-xs mt-1">"모든 레시피"</span>
                </button>
                <button class="flex flex-col items-center">
                    <span class="text-xs mt-1">"즐겨찾기"</span>
                </button>
            </div>
        </div>
    }
}

#[component]
fn Favorites() -> impl IntoView {
    view! {
        <div class="w-full max-w-2xl mx-auto bg-white">
            <h1 class="text-xl font-bold p-4">"즐겨찾기"</h1>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
      <Router>
        <nav>
        </nav>
        <main>
        <Routes fallback=|| "Not found.">
        <Route path=path!("/") view=StoryGallery/>
        <Route path=path!("/users") view=Favorites/>
        <Route path=path!("/users/:id") view=Favorites/>
        <Route path=path!("/*any") view=|| view! { <h1>"Not Found"</h1> }/>
      </Routes>
        </main>
      </Router>
    }
}
fn main() {
    mount_to_body(|| view! { <App/> })
}
