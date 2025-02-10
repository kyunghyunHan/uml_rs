use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: i32,
    pub username: String,
    pub avatar: String,
    pub posts: Vec<String>,
}
pub fn data()->Vec<Profile>{
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
    profiles
}