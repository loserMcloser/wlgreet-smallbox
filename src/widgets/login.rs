use crate::color::Color;
use crate::draw::{draw_box, Font, DEJAVUSANS_MONO};
use crate::widget::{DrawContext, DrawReport, KeyState, ModifiersState, Widget};

use std::env;
use std::error::Error;
use std::os::unix::net::UnixStream;

use smithay_client_toolkit::keyboard::keysyms;

use greet_proto::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};

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

pub struct Login {
    question: String,
    answer: String,
    mode: Option<AuthMessageType>,
    error: Option<String>,
    border: Color,
    headline_font: Font,
    prompt_font: Font,
    dirty: bool,
    reset_border: bool,
    stream: UnixStream,
}

impl Login {
    pub fn new() -> Box<Login> {
        let mut l = Login {
            question: String::new(),
            answer: String::new(),
            mode: None,
            error: None,
            headline_font: Font::new(&DEJAVUSANS_MONO, 72.0),
            prompt_font: Font::new(&DEJAVUSANS_MONO, 32.0),
            border: Color::new(1.0, 1.0, 1.0, 1.0),
            dirty: false,
            reset_border: false,
            stream: UnixStream::connect(env::var("GREETD_SOCK").expect("GREETD_SOCK not set"))
                .expect("Unable to connect to greetd"),
        };
        l.reset();
        Box::new(l)
    }

    fn reset(&mut self) {
        self.question = "login:".to_string();
        self.answer = String::new();
    }

    fn communicate(&mut self) -> Result<(), Box<dyn Error>> {
        let req = match self.mode {
            None => Request::CreateSession {
                username: self.answer.to_string(),
            },
            Some(_) => Request::PostAuthMessageResponse {
                response: Some(self.answer.to_string()),
            },
        };
        req.write_to(&mut self.stream)?;

        match Response::read_from(&mut self.stream)? {
            Response::AuthMessage {
                auth_message,
                auth_message_type,
            } => {
                self.question = auth_message;
                self.mode = Some(auth_message_type);
            }
            Response::Success => {
                Request::StartSession {
                    env: vec![],
                    cmd: vec!["sway-run -d 2>/tmp/swaystuff".to_string()],
                }
                .write_to(&mut self.stream)?;

                match Response::read_from(&mut self.stream)? {
                    Response::Success => std::process::exit(0),
                    Response::Error {
                        error_type,
                        description,
                    } => match error_type {
                        ErrorType::AuthError => return Err("Login failed".into()),
                        ErrorType::Error => {
                            eprintln!("err: {}", description);
                            std::process::exit(-1);
                        }
                    },
                    _ => panic!("unexpected message"),
                }
            }
            Response::Error {
                error_type,
                description,
            } => {
                Request::CancelSession.write_to(&mut self.stream)?;
                match error_type {
                    ErrorType::AuthError => return Err("Login failed".into()),
                    ErrorType::Error => {
                        eprintln!("err: {}", description);
                        std::process::exit(-1);
                    }
                }
            }
        }
        Ok(())
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

        let (w, _) = self.prompt_font.auto_draw_text(
            &mut buf.offset((256, 24))?,
            &ctx.bg,
            &Color::new(1.0, 1.0, 1.0, 1.0),
            &self.question,
        )?;

        match self.mode {
            None | Some(AuthMessageType::Visible) => {
                self.prompt_font.auto_draw_text(
                    &mut buf.subdimensions((256 + w + 16, 24, width - 416 - 32, 64))?,
                    &ctx.bg,
                    &Color::new(1.0, 1.0, 1.0, 1.0),
                    &format!("{}", self.answer),
                )?;
            }
            Some(AuthMessageType::Secret) => {
                let mut stars = "".to_string();
                for _ in 0..self.answer.len() {
                    stars += "*";
                }
                self.prompt_font.auto_draw_text(
                    &mut buf.subdimensions((256 + w + 16, 24, width - 416 - 32, 64))?,
                    &ctx.bg,
                    &Color::new(1.0, 1.0, 1.0, 1.0),
                    &stars,
                )?;
            }
            _ => (),
        }

        if let Some(e) = &self.error {
            self.prompt_font.auto_draw_text(
                &mut buf.offset((256, 64))?,
                &ctx.bg,
                &Color::new(1.0, 1.0, 1.0, 1.0),
                e,
            )?;
        }

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
                self.answer.truncate(self.answer.len().saturating_sub(1));
                self.dirty = true;
                return;
            }
            keysyms::XKB_KEY_Return => {
                let res = self.communicate();
                self.dirty = true;
                self.answer.clear();
                self.error = None;
                if let Err(e) = res {
                    self.error = Some(format!("{}", e));
                    self.mode = None;
                    return;
                }
            }
            _ => match interpreted {
                Some(v) => {
                    self.answer += &v;
                    self.dirty = true;
                    return;
                }
                None => {}
            },
        }
    }
    fn mouse_click(&mut self, _: u32, _: (u32, u32)) {}
    fn mouse_scroll(&mut self, _: (f64, f64), _: (u32, u32)) {}
}
