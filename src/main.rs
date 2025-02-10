use leptos::mount::mount_to_body;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Profile {
    id: i32,
    username: String,
    avatar: String,
    posts: Vec<String>,
}

#[component]
fn StoryGallery() -> impl IntoView {
    let (selected_profile, set_selected_profile) = create_signal::<Option<Profile>>(None);

    let profiles = vec![
        Profile {
            id: 1,
            username: "시금치".to_string(),
            avatar: "/img/시금치.jpg".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 2,
            username: "양배추".to_string(),
            avatar: "/img/양배추.jpg".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 3,
            username: "닭가슴살".to_string(),
            avatar: "/img/닭가슴살.jpeg".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 4,
            username: "올리브유".to_string(),
            avatar: "/img/올리브유.webp".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 5,
            username: "양상추".to_string(),
            avatar: "/img/양상추.png".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 6,
            username: "계란".to_string(),
            avatar: "/img/계란.jpeg".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 7,
            username: "두부".to_string(),
            avatar: "/img/두부.jpg".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 8,
            username: "잡곡".to_string(),
            avatar: "/img/잡곡.webp".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
        Profile {
            id: 9,
            username: "렌틸콩".to_string(),
            avatar: "/img/렌틸콩.webp".to_string(),
            posts: vec![
                "/api/placeholder/300/400".to_string(),
                "/api/placeholder/300/400".to_string(),
            ],
        },
    ];

    view! {
        <div class="flex flex-col w-full max-w-2xl mx-auto">
            <div class="flex space-x-4 p-4 overflow-x-auto">
                {profiles.into_iter().map(|profile| {
                    let profile_clone = profile.clone();
                    let set_selected = set_selected_profile.clone();
                    view! {
                        <button
                            on:click=move |_| set_selected.set(Some(profile_clone.clone()))
                            class="flex flex-col items-center"
                        >
                            // <div class="w-16 h-16 rounded-full p-0.5 bg-gradient-to-tr from-yellow-400 via-red-500 to-purple-500">
                            //     // <div class="w-full h-full rounded-full p-0.5 bg-black">
                            //     //     <img
                            //     //         src={profile.avatar}
                            //     //         alt={profile.username.clone()}
                            //     //         class="w-full h-full rounded-full object-cover"
                            //     //     />
                            //     // </div>
                            // </div>
                            <span class="text-xs mt-1 text-gray-200">{profile.username}</span>
                        </button>
                    }
                }).collect_view()}
            </div>

            {move || {
                let empty_view = view! {
                    <div class="grid grid-cols-3 gap-1 p-4">
                        {Vec::<View<_>>::new()}
                    </div>
                };

                selected_profile.get().map(|profile| {
                    let profile = profile.clone();
                    let posts_view = profile.posts.into_iter().enumerate().map(|(index, post)| {
                        view! {
                            <div class="aspect-square">
                                <img
                                    src={post}
                                    alt={format!("Post {} by {}", index + 1, profile.username)}
                                    class="w-full h-full object-cover"
                                />
                            </div>
                        }
                    }).collect_view();

                    view! {
                        <div class="grid grid-cols-3 gap-1 p-4">
                            {posts_view}
                        </div>
                    }
                }).unwrap_or(empty_view)
            }}
        </div>
    }
}

fn main() {
    mount_to_body(|| view! { <StoryGallery/> })
}
