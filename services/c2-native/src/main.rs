mod app;
mod ecs;
mod renderer;
mod ui;

fn main() -> anyhow::Result<()> {
    app::run()
}
