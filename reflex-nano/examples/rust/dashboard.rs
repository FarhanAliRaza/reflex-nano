use reflex_nano::dashboard_application;

#[tokio::main]
async fn main() -> Result<(), String> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3001);
    dashboard_application()?
        .build()?
        .serve("127.0.0.1", port)
        .await
}
