use std::default::Default;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::mpsc::channel;

use nix::poll::{poll, PollFd, PollFlags};
use os_pipe::pipe;

mod app;
mod buffer;
mod cmd;
mod color;
mod config;
mod doublemempool;
mod draw;
mod widget;
mod widgets;

use app::{App, OutputMode};
use cmd::Cmd;

enum Mode {
    Start,
    PrintConfig(bool),
}

fn main() {
    let socket_path = match env::var("XDG_RUNTIME_DIR") {
        Ok(dir) => dir + "/wlgreet",
        Err(_) => "/tmp/wlgreet".to_string(),
    };
    let config_home = match env::var("XDG_CONFIG_HOME") {
        Ok(dir) => dir + "/wlgreet",
        Err(_) => match env::var("HOME") {
            Ok(home) => home + "/.config/wlgreet",
            Err(_) => panic!("unable to find user folder"),
        },
    };

    let (is_yaml, config): (bool, config::Config) =
        match File::open(config_home.clone() + "/config.yaml") {
            Ok(f) => {
                let reader = BufReader::new(f);
                (true, serde_yaml::from_reader(reader).unwrap())
            }
            Err(_) => match File::open(config_home + "/config.json") {
                Ok(f) => {
                    let reader = BufReader::new(f);
                    (false, serde_json::from_reader(reader).unwrap())
                }
                Err(_) => (true, Default::default()),
            },
        };

    let scale = config.scale;

    let args: Vec<String> = env::args().collect();
    let mode = match args.len() {
        1 => Mode::Start,
        2 => match args[1].as_str() {
            "start" => Mode::Start,
            "print-config" => Mode::PrintConfig(!is_yaml),
            "print-config-json" => Mode::PrintConfig(true),
            "print-config-yaml" => Mode::PrintConfig(false),
            s => {
                eprintln!("unsupported sub-command {}", s);
                std::process::exit(1);
            }
        },
        v => {
            eprintln!("expected 0 or 1 arguments, got {}", v);
            std::process::exit(1);
        }
    };

    match mode {
        Mode::Start => (),
        Mode::PrintConfig(json) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&config).unwrap());
            } else {
                println!("{}", serde_yaml::to_string(&config).unwrap());
            }
            std::process::exit(0);
        }
    }

    let output_mode = match config.output_mode {
        config::OutputMode::All => OutputMode::All,
        config::OutputMode::Active => OutputMode::Active,
    };

    let background = config.background;

    let (tx_draw, rx_draw) = channel();
    let tx_draw_mod = tx_draw.clone();
    let (mod_tx, mod_rx) = channel();
    std::thread::spawn(move || {
        // Print, write to a file, or send to an HTTP server.
        match config.widget.construct(tx_draw_mod) {
            Some(w) => mod_tx.send(w).unwrap(),
            None => panic!("no widget configured"),
        }
    });

    let mut app = App::new(tx_draw, output_mode, background, scale);
    let widget = mod_rx.recv().unwrap();
    app.set_widget(widget).unwrap();

    let (mut rx_pipe, mut tx_pipe) = pipe().unwrap();

    let worker_queue = app.cmd_queue();
    let _ = std::thread::Builder::new()
        .name("cmd_proxy".to_string())
        .spawn(move || loop {
            let cmd = rx_draw.recv().unwrap();
            worker_queue.lock().unwrap().push_back(cmd);
            tx_pipe.write_all(&[0x1]).unwrap();
        });

    let mut fds = [
        PollFd::new(app.event_queue().get_connection_fd(), PollFlags::POLLIN),
        PollFd::new(rx_pipe.as_raw_fd(), PollFlags::POLLIN),
    ];

    app.cmd_queue().lock().unwrap().push_back(Cmd::Draw);

    let q = app.cmd_queue();
    loop {
        let cmd = q.lock().unwrap().pop_front();
        match cmd {
            Some(cmd) => match cmd {
                Cmd::Draw => {
                    app.redraw(false).expect("Failed to draw");
                    app.flush_display();
                }
                Cmd::ForceDraw => {
                    app.redraw(true).expect("Failed to draw");
                    app.flush_display();
                }
                Cmd::MouseClick { btn, pos } => {
                    app.get_widget().mouse_click(btn, pos);
                    q.lock().unwrap().push_back(Cmd::Draw);
                }
                Cmd::MouseScroll { scroll, pos } => {
                    app.get_widget().mouse_scroll(scroll, pos);
                    q.lock().unwrap().push_back(Cmd::Draw);
                }
                Cmd::Keyboard {
                    key,
                    key_state,
                    modifiers_state,
                    interpreted,
                } => {
                    app.get_widget()
                        .keyboard_input(key, modifiers_state, key_state, interpreted);
                    q.lock().unwrap().push_back(Cmd::Draw);
                }
                Cmd::Exit => {
                    let _ = std::fs::remove_file(socket_path);
                    return;
                }
            },
            None => {
                app.flush_display();

                poll(&mut fds, -1).unwrap();

                if fds[0].revents().unwrap().contains(PollFlags::POLLIN) {
                    if let Some(guard) = app.event_queue().prepare_read() {
                        if let Err(e) = guard.read_events() {
                            if e.kind() != ::std::io::ErrorKind::WouldBlock {
                                eprintln!(
                                    "Error while trying to read from the wayland socket: {:?}",
                                    e
                                );
                            }
                        }
                    }

                    app.event_queue()
                        .dispatch_pending()
                        .expect("Failed to dispatch all messages.");
                }

                if fds[1].revents().unwrap().contains(PollFlags::POLLIN) {
                    let mut v = [0x00];
                    rx_pipe.read_exact(&mut v).unwrap();
                }
            }
        }
    }
}
