use crate::buffer::Buffer;
use crate::color::Color;
use crate::config::Config;
use chrono::{DateTime, Local};
pub use smithay_client_toolkit::seat::keyboard::{KeyState, ModifiersState};

pub struct DrawContext<'a> {
    pub buf: &'a mut Buffer<'a>,
    pub bg: &'a Color,
    pub time: &'a DateTime<Local>,
    pub force: bool,
    pub config: &'a Config,
}

#[derive(Debug)]
pub struct DrawReport {
    pub width: u32,
    pub height: u32,
    pub damage: Vec<(i32, i32, i32, i32)>,
    pub full_damage: bool,
}

impl DrawReport {
    pub fn empty(width: u32, height: u32) -> DrawReport {
        DrawReport {
            width,
            height,
            damage: Vec::new(),
            full_damage: false,
        }
    }
}

pub trait Widget {
    fn size(&self) -> (u32, u32);
    fn draw(
        &mut self,
        ctx: &mut DrawContext,
        pos: (u32, u32),
    ) -> Result<DrawReport, ::std::io::Error>;

    fn keyboard_input(
        &mut self,
        keysym: u32,
        modifier_state: ModifiersState,
        key_state: KeyState,
        interpreted: Option<String>,
    );
    fn mouse_click(&mut self, button: u32, pos: (u32, u32));
    fn mouse_scroll(&mut self, scroll: (f64, f64), pos: (u32, u32));
}
