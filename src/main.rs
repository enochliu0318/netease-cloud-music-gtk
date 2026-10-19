mod application;
mod audio;
mod config;
mod gui;
mod model;
mod ncmapi;
mod path;
mod utils;
mod window;

use self::application::NeteaseCloudMusicGtk4Application;
use self::gui::NowPlayingPage;
use self::window::NeteaseCloudMusicGtk4Window;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use env_logger::Env;
use once_cell::sync::Lazy;

const APP_ID: &str = "com.gitee.gmg137.NeteaseCloudMusicGtk4";
const APP_NAME: &str = "NetEase Cloud Music Gtk4";
const MPRIS_NAME: &str = "NeteaseCloudMusicGtk4";

pub static MAINCONTEXT: Lazy<glib::MainContext> = Lazy::new(glib::MainContext::default);

fn main() {
    // Initialize log
    env_logger::Builder::from_env(Env::default().default_filter_or("off")).init();

    // Initialize gstreamer
    gstreamer::init().expect("Error initializing gstreamer");

    // Initialize paths
    path::init().expect("Unable to create paths.");
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources (prefer build dir for dev, fall back to install prefix)
    let build_dir_res = concat!(env!("CARGO_MANIFEST_DIR"), "/_build/data/netease-cloud-music-gtk4.gresource");
    let install_res = PKGDATADIR.to_owned() + "/netease-cloud-music-gtk4.gresource";
    let resources = if std::path::Path::new(build_dir_res).exists() {
        gio::Resource::load(build_dir_res).expect("Could not load resources from build dir")
    } else {
        gio::Resource::load(&install_res).expect("Could not load resources from install dir")
    };
    gio::resources_register(&resources);

    // 确保 NowPlayingPage 类型已注册到 GType 系统，
    // 以便 window.ui 中的 <object class="NowPlayingPage"> 能被 GTK Builder 实例化
    let _ = NowPlayingPage::static_type();

    glib::set_application_name(&gettextrs::gettext(APP_NAME));

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = NeteaseCloudMusicGtk4Application::new(APP_ID, &gio::ApplicationFlags::NON_UNIQUE);

    let _guard = MAINCONTEXT.acquire().unwrap();

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    std::process::exit(app.run().into());
}
