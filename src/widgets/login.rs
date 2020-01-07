use crate::color::Color;
use crate::draw::{draw_box, Font, DEJAVUSANS_MONO};
use crate::widget::{DrawContext, DrawReport, KeyState, ModifiersState, Widget};

use std::io;
use std::io::{Write, Read};
use std::collections::HashMap;
use std::env;
use std::os::unix::net::UnixStream;

use smithay_client_toolkit::keyboard::keysyms;

use greet_proto::{Request, Response, Header};

pub trait Scrambler {
    fn scramble(&mut self);
}

impl<T: Default> Scrambler for Vec<T> {
    fn scramble(&mut self) {
        let cap = self.capacity();
        self.truncate(0);
        for _ in 0..cap {
            self.push(Default::default())
        }
        self.truncate(0);
    }
}

impl Scrambler for String {
    fn scramble(&mut self) {
        let cap = self.capacity();
        self.truncate(0);
        for _ in 0..cap {
            self.push(Default::default())
        }
        self.truncate(0);
    }
}

enum Mode {
    EditingUsername,
    EditingPassword,
}

pub struct Login {
    username: String,
    password: String,
    mode: Mode,
    border: Color,
    headline_font: Font,
    prompt_font: Font,
    dirty: bool,
    reset_border: bool,
}

impl Login {
    pub fn new(
    ) -> Box<Login> {
        Box::new(Login {
            username: String::new(),
            password: String::new(),
            mode: Mode::EditingUsername,
            headline_font: Font::new(&DEJAVUSANS_MONO, 72.0),
            prompt_font: Font::new(&DEJAVUSANS_MONO, 32.0),
            border: Color::new(1.0, 1.0, 1.0, 1.0),
            dirty: false,
            reset_border: false,
        })
    }
}

fn login(username: &str, password: &str, command: Vec<String>, env: HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::Login{
        username: username.to_string(),
        password: password.to_string(),
        command,
        env,
    };

    let mut stream = UnixStream::connect(env::var("GREETD_SOCK")?)?;

    // Write request
    let mut req = request.to_bytes()?;
    let header = Header::new(req.len() as u32);
    stream.write_all(&header.to_bytes()?)?;
    stream.write_all(&req)?;

    // Wipe password
    req.scramble();
    match request {
        Request::Login { mut password, .. } => password.scramble(),
        _ => (),
    }

    // Read response
    let mut header_buf = vec![0; Header::len()];
    stream.read_exact(&mut header_buf)?;
    let header = Header::from_slice(&header_buf)?;

    let mut resp_buf = vec![0; header.len as usize];
    stream.read_exact(&mut resp_buf)?;
    let resp = Response::from_slice(&resp_buf)?;

    match resp {
        Response::Success => Ok(()),
        Response::Failure(err) => Err(std::io::Error::new(io::ErrorKind::Other, format!("login error: {:?}", err)).into())
    }
}

impl Widget for Login {
    fn size(&self) -> (u32, u32) {
        (1024, 128)
    }

    fn draw(
        &mut self,
        ctx: &mut DrawContext,
        _pos: (u32, u32),
    ) -> Result<DrawReport, ::std::io::Error> {
        let (width, height) = self.size();
        if !self.dirty && !ctx.force {
            return Ok(DrawReport::empty(width, height));
        }
        self.dirty = false;
        let mut buf = ctx.buf.subdimensions((0, 0, width, height))?;
        buf.memset(&ctx.bg);
        draw_box(&mut buf, &self.border, (width, height))?;

        self.headline_font.auto_draw_text(
            &mut buf.offset((32, 24))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            "Login",
        )?;

        self.prompt_font.auto_draw_text(
            &mut buf.offset((256, 24))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            "username:",
        )?;

        self.prompt_font.auto_draw_text(
            &mut buf.offset((256, 64))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            "password:",
        )?;

        self.prompt_font.auto_draw_text(
            &mut buf.subdimensions((416, 24, width - 416 - 32, 64))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            &format!("{}", self.username)
        )?;

        let mut stars = "".to_string();
        for _ in 0..self.password.len() {
            stars += "*";
        }

        self.prompt_font.auto_draw_text(
            &mut buf.subdimensions((416, 64, width - 416 - 32, 64))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            &stars
        )?;

        if self.reset_border {
            self.border = Color::new(1.0, 1.0, 1.0, 1.0);
            self.reset_border = false;
        }

        Ok(DrawReport {
            width: width,
            height: height,
            damage: vec![buf.get_signed_bounds()],
            full_damage: false,
        })
    }

    fn keyboard_input(
        &mut self,
        key: u32,
        _: ModifiersState,
        _: KeyState,
        interpreted: Option<String>,
    ) {
        match key {
            keysyms::XKB_KEY_BackSpace => {
                match self.mode {
                    Mode::EditingUsername => self.username.truncate(self.username.len().saturating_sub(1)),
                    Mode::EditingPassword => self.password.truncate(self.password.len().saturating_sub(1)),
                };
                self.dirty = true;
            }
            keysyms::XKB_KEY_Return => match self.mode {
                Mode::EditingUsername => {
                    if self.username.len() > 0 {
                        self.mode = Mode::EditingPassword
                    }
                }
                Mode::EditingPassword => {
                    if self.password.len() > 0 {
                        let mut env = HashMap::new();
                        env.insert("XDG_SESSION_TYPE".to_string(), "wayland".to_string());
                        env.insert("XDG_SESSION_DESKTOP".to_string(), "sway".to_string());
                        let res = login(&self.username,
                            &self.password,
                            vec!["sway".to_string()],
                            env
                        );
                        self.username.scramble();
                        self.password.scramble();
                        match res {
                            Ok(_) => {
                                std::process::exit(0)
                            },
                            Err(_) => {
                                self.border = Color::new(0.75, 0.25, 0.25, 1.0);
                                self.mode = Mode::EditingUsername;
                                self.reset_border = true;
                            }
                        }
                    }
                }
            }
            _ => match interpreted {
                Some(v) => {
                    match self.mode {
                        Mode::EditingUsername => self.username += &v,
                        Mode::EditingPassword => self.password += &v,
                    }
                    self.dirty = true;
                }
                None => {}
            },
        }
    }
    fn mouse_click(&mut self, _: u32, _: (u32, u32)) {}
    fn mouse_scroll(&mut self, _: (f64, f64), _: (u32, u32)) {}
}
