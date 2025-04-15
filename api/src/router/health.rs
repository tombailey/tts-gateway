use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, get};
use serde_json::json;

#[get("/health")]
pub async fn get_health() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(json!({ "status": "pass" }))
}
