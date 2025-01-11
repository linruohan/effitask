#![warn(warnings)]

use adw::prelude::*;
use relm4::gtk;
mod agenda;
mod application;
mod date;
mod done;
mod edit;
mod flag;
mod inbox;
mod logger;
mod search;
mod tasks;
mod widgets;

fn main() {
    human_panic::setup_panic!();

    envir::init();

    if std::env::args().nth(1).as_deref() == Some("usage") {
        usage(&std::env::args().next().unwrap());

        std::process::exit(0);
    }

    let config = todo_txt::Config::from_env();
    // 设置主题
    gtk::init().expect("Failed to initialize GTK.");
    let settings = gtk::Settings::default().expect("Failed to get default settings.");
    settings.set_property("gtk-theme-name", &"Adwaita:dark");

    let app = relm4::RelmApp::new("txt.todo.effitask").with_args(Vec::new());
    initialize_resources();

    app.run::<application::Model>(config);
}

fn usage(program: &str) {
    let path = std::path::Path::new(&program);

    println!("    {}", path.file_name().unwrap().to_str().unwrap());
    println!("      Launch focus graphical interface");
}

fn initialize_resources() {
    gtk::gio::resources_register_include!("gresource").unwrap();

    let display = gtk::gdk::Display::default().unwrap();
    let theme = gtk::IconTheme::for_display(&display);
    theme.add_resource_path("/txt/todo/effitask");
}
