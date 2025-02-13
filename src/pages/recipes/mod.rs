use crate::AppState;
use leptos::prelude::*;
use crate::pages::main::Profile;

#[derive(Clone, Debug)]
pub struct Recipe {
    id: i32,
    title: String,
    description: String,
    ingredients: Vec<Profile>,
    cooking_time: String,
    difficulty: String,
    image: String,
}

#[component]
pub fn Recipes() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState should be provided");
    let selected_items = state.selected_items;

    let recipes = vec![
        Recipe {
            id: 1,
            title: "시금치 무침".to_string(),
            description: "간단하게 만드는 시금치 무침".to_string(),
            ingredients: vec![], // 임시로 비워둠
            cooking_time: "10분".to_string(),
            difficulty: "쉬움".to_string(),
            image: "/img/recipes/spinach.jpg".to_string(),
        },
    ];

    let filtered_recipes = move || {
        let items = selected_items.get();
        if items.is_empty() {
            vec![]
        } else {
            recipes.clone()
        }
    };

    view! {
        <div class="p-4">
            <h2 class="text-xl font-bold mb-4">
                {move || format!("{}개 재료로 만들 수 있는 레시피", selected_items.get().len())}
            </h2>
            <div class="grid gap-4">
                {move || filtered_recipes().into_iter().map(|recipe| {
                    view! {
                        <div class="bg-white rounded-lg shadow p-4">
                            <h3 class="text-lg font-bold">{recipe.title}</h3>
                            <p class="text-gray-600">{recipe.description}</p>
                            <div class="mt-2 flex items-center space-x-4 text-sm text-gray-500">
                                <span>{recipe.cooking_time}</span>
                                <span>{recipe.difficulty}</span>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}