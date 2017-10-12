#[macro_use] extern crate lazy_static;
extern crate gtk;
extern crate gdk;
extern crate glib;
//extern crate crossbeam;
extern crate regex;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::cell::RefCell;

use gtk::prelude::*;
use gtk::{
    Builder,
};

mod client;
mod servercomms;

pub enum UiMessage {
    Log(String),
    ConnectionFailed,
    Connected(String),
}

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

    let cmdline: gtk::Entry = builder.get_object("cmdline").unwrap();
    
    let (ui_tx, ui_rx) = mpsc::channel();
    let (client_tx, client_rx) = mpsc::channel();
    
    {
        let client_tx_ = client_tx.clone();
        cmdline.connect_activate(move |this| {
            client_tx_.send(client::GuiClientMessage::Command(this.get_buffer().get_text())).unwrap();
            this.get_buffer().set_text("");
        });
    }

    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((battlelog, cmdline, ui_rx))
    });

    thread::spawn(move|| {
        glib::timeout_add(50,receive);
        client::client(client_rx, ui_tx);
        println!("thread existed!");
    });

    window.show_all();
    gtk::main();
}

fn receive() -> glib::Continue {
    GLOBAL.with(move |global| {
        if let Some((ref view, ref cmdline, ref ui_rx)) = *global.borrow() {
            while let Ok(message) = ui_rx.try_recv() {
                {
                    match message {
                        UiMessage::Log(text) => {
                            let buf = view.get_buffer().unwrap();
                            buf.insert(&mut buf.get_end_iter(), &text);
                            view.scroll_mark_onscreen(
                                &mut view.get_buffer().unwrap().get_mark("end").unwrap());
                        },
                        _ => ()
                    }
                }
            }
            glib::Continue(true)
        } else {
            glib::Continue(true)
        }
    })
}

thread_local!(
    static GLOBAL: RefCell<Option<(gtk::TextView, gtk::Entry, mpsc::Receiver<UiMessage>)>> = RefCell::new(None)
);
