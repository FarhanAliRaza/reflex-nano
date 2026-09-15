//! Run a native demo, benchmark, or exported application without Python installed.
use reflex_nano::{benchmark_application, dashboard_application, Application};

#[tokio::main]
async fn main() -> Result<(), String> {
    let mut args: Vec<_> = std::env::args().collect();
    let mut renderer = std::env::var("NANO_RENDERER").ok();
    if let Some(index) = args.iter().position(|v| v == "--renderer") {
        if index + 1 >= args.len() {
            return Err("Missing renderer name".into());
        }
        renderer = Some(args.remove(index + 1));
        args.remove(index);
    }
    let mut definition = match args.get(1).map(String::as_str) {
        Some("--benchmark") => benchmark_application(
            std::env::var("NANO_BENCH_ROWS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
        )?,
        Some("--manifest") => Application::from_json(
            &std::fs::read_to_string(args.get(2).ok_or("Missing manifest path")?)
                .map_err(|e| e.to_string())?,
        )?,
        None => dashboard_application()?,
        _ => return Err("Usage: runner [--benchmark | --manifest app.json]".into()),
    };
    if let Some(renderer) = renderer {
        definition.renderer = renderer.parse()?;
    }
    let port = std::env::var("NANO_BENCH_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    definition.build()?.serve("127.0.0.1", port).await
}
