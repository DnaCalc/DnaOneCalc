use dnaonecalc_host::app::host_mount::{render_shell_html, HostMountTarget};
use dnaonecalc_host::app::OneCalcHostApp;

fn main() {
    let app = OneCalcHostApp::new();
    println!("{}", app.launch_message());
    println!("{}", render_shell_html(HostMountTarget::DesktopTauri, app.state().clone()));
}
