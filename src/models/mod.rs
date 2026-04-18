// Data structures (Database entities, API requests/responses)

pub struct User {
    pub id: String,
    pub email: String,
}

pub struct EmailRecord {
    pub id: String,
    pub user_id: String,
    pub subject: String,
    pub status: String,
}
