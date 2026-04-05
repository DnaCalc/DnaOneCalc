use dnaonecalc_host::app::OneCalcHostApp;

fn main() {
    let app = OneCalcHostApp::new();
    println!("{}", app.launch_message());
}
