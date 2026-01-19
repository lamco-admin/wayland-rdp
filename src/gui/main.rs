//! lamco-rdp-server-gui entry point
//!
//! GUI binary for configuring the lamco-rdp-server.

use iced::Size;

use lamco_rdp_server::gui::app::ConfigGuiApp;

fn main() -> iced::Result {
    // Run the application with window configuration
    iced::application(ConfigGuiApp::new, ConfigGuiApp::update, ConfigGuiApp::view)
        .title("lamco-rdp-server Configuration")
        .window_size(Size::new(1200.0, 800.0))
        .centered()
        .antialiasing(true)
        .subscription(ConfigGuiApp::subscription)
        .run()
}
