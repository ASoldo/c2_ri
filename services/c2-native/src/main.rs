mod app;
mod ecs;
mod renderer;
mod tiles;
mod ui;

fn main() -> anyhow::Result<()> {
    app::run()
}
