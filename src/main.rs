use leptos::mount::mount_to_body;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
mod data;
use data::Profile;

#[component]
fn StoryGallery() -> impl IntoView {
    let (selected_profile, set_selected_profile) = create_signal::<Option<Profile>>(None);
    let profiles = data::data();

    view! {
        // 헤더 추가
        <div class="w-full max-w-2xl mx-auto bg-white">
            <div class="flex justify-between items-center p-4 border-b">
                <h1 class="text-xl font-bold">나의 냉장고</h1>
                <div class="flex items-center space-x-4">
                    <button class="p-2">use data::Profile;
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                        </svg>
                    </button>
                    <button class="p-2">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        </svg>
                    </button>
                </div>
            </div>

            // 기존 컨텐츠를 그리드로 변경
            <div class="grid grid-cols-2 gap-4 p-4">
                {profiles.into_iter().map(|profile| {
                    let profile_clone = profile.clone();
                    let set_selected = set_selected_profile.clone();
                    view! {
                        <button
                            on:click=move |_| set_selected.set(Some(profile_clone.clone()))
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
                            <span class="text-xs text-gray-500">"25.02.10 추가"</span>
                        </button>
                    }
                }).collect_view()}
            </div>

            // 냉장고 파먹기 버튼 추가
            <div class="fixed bottom-0 left-0 right-0 p-4 bg-white border-t">
                <button class="w-full py-3 bg-red-500 text-white rounded-lg text-center">
                    "냉장고 파먹기"
                </button>
            </div>

            // 하단 네비게이션 바 추가
            <div class="fixed bottom-0 left-0 right-0 bg-white border-t flex justify-around p-4">
                <button class="flex flex-col items-center text-blue-500">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
                    </svg>
                    <span class="text-xs mt-1">"나의 냉장고"</span>
                </button>
                <button class="flex flex-col items-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                    </svg>
                    <span class="text-xs mt-1">"모든 레시피"</span>
                </button>
                <button class="flex flex-col items-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                    </svg>
                    <span class="text-xs mt-1">"즐겨찾기"</span>
                </button>
            </div>
        </div>
    }
}

fn main() {
    mount_to_body(|| view! { <StoryGallery/> })
}
