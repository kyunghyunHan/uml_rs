use leptos::prelude::*;
use leptos_router::components::{A, Outlet, ParentRoute, Route, Router, Routes};
use serde::{Deserialize, Serialize};
use crate::AppState;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Category {
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
pub struct Profile {
    pub id: i32,
    pub username: String,
    pub avatar: String,
    pub categories: Vec<Category>,
    pub posts: Vec<String>,
    pub added_date: String,
}

#[component]
pub fn StoryGallery() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState should be provided");

    let selected_items = state.selected_items;
    let selected_categories = state.selected_categories;
    let select_profile_action = state.select_profile_action;
    let toggle_category_action = state.toggle_category_action;
    let (selected_profile, set_selected_profile) = create_signal::<Option<Profile>>(None);
    let (selected_categories, set_selected_categories) = create_signal::<Vec<Category>>(vec![]);
    let (selected_items, set_selected_items) = create_signal::<Vec<Profile>>(vec![]);
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
    let select_profile_action = Action::new(move |profile: &Profile| {
        let profile = profile.clone();
        let items = selected_items;
        let set_items = set_selected_items;

        async move {
            let mut current = items.get();
            if let Some(pos) = current.iter().position(|x| x.id == profile.id) {
                current.remove(pos);
            } else {
                current.push(profile);
            }
            set_items.set(current);
        }
    });
    let toggle_action = Action::new(move |category: &Category| {
        let category = category.clone();
        let selected = selected_categories;
        let set_selected = set_selected_categories;

        async move {
            let mut current = selected.get();
            if let Some(pos) = current.iter().position(|x| x == &category) {
                current.remove(pos);
            } else {
                current.push(category);
            }
            set_selected.set(current);
        }
    });
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
    // let toggle_category = move |category: Category| {
    //     let mut current = selected_categories.get();
    //     if let Some(pos) = current.iter().position(|x| x == &category) {
    //         current.remove(pos);
    //     } else {
    //         current.push(category);
    //     }
    //     set_selected_categories.set(current); // 수정된 부분
    // };

    view! {
        <div >
        <div class="p-4">
        {move || {
            let items = selected_items.get();
            view! {
                <div>
                    {items.into_iter().map(|item| {
                        view! {
                            <div class="bg-blue-100 rounded-full px-3 py-1 text-sm">
                                {item.username}
                            </div>
                        }
                    }).collect_view()}
                </div>
            }
        }}
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
                        let toggle_action = toggle_action.clone();

                        view! {
                            <button
                                class=move || format!("flex flex-col items-center {}", if is_selected() { "opacity-100" } else { "opacity-50" })
                                on:click=move |_| {
                                    toggle_action.dispatch(category_clone.clone());
                                }
                            >
                                <span class="text-xs">{label}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="grid grid-cols-2 gap-4 p-4">
            {move || filtered_profiles().into_iter().map(|profile| {
                let profile_clone = profile.clone();
                let is_selected = move || selected_items.get().iter().any(|p| p.id == profile.id);
                let select_action = select_profile_action.clone();

                view! {
                    <button
                        class=move || {
                            let base = "bg-white rounded-lg shadow p-4 flex flex-col items-center";
                            if is_selected() {
                                format!("{} border-2 border-red-500", base)
                            } else {
                                base.to_string()
                            }
                        }
                        on:click=move |_| {
                            select_action.dispatch(profile_clone.clone());
                        }
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
          
        </div>
    }
}
