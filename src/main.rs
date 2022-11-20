pub mod run;

use bevy::prelude::App;

fn main() {
    #[allow(unused_mut)]
    let mut app = App::new();

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        app.add_plugin(bevy_web_resizer::Plugin);
    }

    run::run(app).unwrap();
}
