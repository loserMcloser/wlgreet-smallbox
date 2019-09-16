use crate::cmd::Cmd;
use crate::color::Color;
use crate::widget;
use crate::widgets;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::sync::mpsc::Sender;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Widget {
    Margin {
        margins: (u32, u32, u32, u32),
        widget: Box<Widget>,
    },
    Fixed {
        width: u32,
        height: u32,
        widget: Box<Widget>,
    },
    HorizontalLayout(Vec<Box<Widget>>),
    VerticalLayout(Vec<Box<Widget>>),
    Login,
}

impl Widget {
    pub fn construct(self, tx: Sender<Cmd>) -> Option<Box<dyn widget::Widget + Send>> {
        match self {
            Widget::Margin { margins, widget } => match widget.construct(tx.clone()) {
                Some(w) => Some(widget::Margin::new(margins, w)),
                None => None,
            },
            Widget::Fixed {
                width,
                height,
                widget,
            } => match widget.construct(tx.clone()) {
                Some(w) => Some(widget::Fixed::new((width, height), w)),
                None => None,
            },
            Widget::HorizontalLayout(widgets) => Some(widget::HorizontalLayout::new(
                widgets
                    .into_iter()
                    .map(|x| x.construct(tx.clone()))
                    .filter(|x| x.is_some())
                    .map(|x| x.unwrap())
                    .collect(),
            )),
            Widget::VerticalLayout(widgets) => Some(widget::VerticalLayout::new(
                widgets
                    .into_iter()
                    .map(|x| x.construct(tx.clone()))
                    .filter(|x| x.is_some())
                    .map(|x| x.unwrap())
                    .collect(),
            )),
            Widget::Login => Some(widgets::login::Login::new()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    All,
    Active,
}

impl Default for OutputMode {
    fn default() -> Self {
        OutputMode::Active
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub output_mode: OutputMode,
    pub scale: u32,
    pub background: Color,
    pub widget: Widget,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            widget: Widget::Login,
            output_mode: Default::default(),
            scale: 1,
            background: Color::new(0.0, 0.0, 0.0, 0.9),
        }
    }
}
