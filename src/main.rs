use leptos::mount::mount_to_body;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use serde::{Deserialize, Serialize};
mod routes;
use routes::{main::pages::__StoryGallery, users::pages::__Favorites};

#[component]
pub fn App() -> impl IntoView {
    view! {
      <Router>
        <nav>
        </nav>
        <main>
        <Routes fallback=|| "Not found.">
        <Route path=path!("/") view=__StoryGallery/>
        <Route path=path!("/users") view=__Favorites/>
        // <Route path=path!("/users/:id") view=Favorites/>
        // <Route path=path!("/*any") view=|| view! { <h1>"Not Found"</h1> }/>
      </Routes>
        </main>
      </Router>
    }
}
fn main() {
    mount_to_body(|| view! { <App/> })
}
