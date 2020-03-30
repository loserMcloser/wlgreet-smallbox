use std::io::{Read, Write};
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

use app::App;
use cmd::Cmd;

fn main() {
    let config = config::read_config();

    let (tx_draw, rx_draw) = channel();
    let mut app = App::new(tx_draw, config.clone());
    app.set_widget(widgets::login::Login::new(config.command))
        .unwrap();

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
