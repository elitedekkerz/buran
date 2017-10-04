extern crate gtk;
extern crate gdk;
extern crate glib;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::cell::RefCell;

use gtk::prelude::*;
use gtk::{
    Builder,
};

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK+.");
        return;
    }

    let glade_src = include_str!("gui.glade");

    let builder = Builder::new_from_string(glade_src);

    let window: gtk::Window = builder.get_object("main_window").unwrap();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let battlelog: gtk::TextView = builder.get_object("battlelog").unwrap();
    battlelog.get_buffer().unwrap().create_mark("end", &battlelog.get_buffer().unwrap().get_end_iter(), false);
    
    let (tx,rx) = mpsc::channel();
    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((battlelog, rx))
    });

    thread::spawn(move|| {
        loop {
            thread::sleep(Duration::from_millis(1000));

            tx.send("> greetings from thread".to_owned()).expect("Couldn't send text through channel");

            glib::idle_add(receive);
        }
    });

    window.show_all();
    gtk::main();
}

fn receive() -> glib::Continue {
    GLOBAL.with(move |global| {
        if let Some((ref view, ref rx)) = *global.borrow() {
            if let Ok(text) = rx.try_recv() {
                {
                    let buf = view.get_buffer().unwrap();
                    buf.insert(&mut buf.get_end_iter(), "\n");
                    buf.insert(&mut buf.get_end_iter(), &text);
                }
                view.scroll_mark_onscreen(&mut view.get_buffer().unwrap().get_mark("end").unwrap());
            }
        }
    });
    glib::Continue(false)
}

thread_local!(
    static GLOBAL: RefCell<Option<(gtk::TextView, mpsc::Receiver<String>)>> = RefCell::new(None)
);
