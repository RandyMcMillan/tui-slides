use image::DynamicImage;
use ratatui::{layout::Rect, text::Line, widgets::Paragraph};
use serde::{Deserialize, Serialize};
use tui_big_text::BigText;

// #[derive(Debug)]
pub enum ReturnSlideWidget<'a> {
    Paragraph(Paragraph<'a>),
    BigText(BigText<'a>),
    Line(Line<'a>),
    Image(DynamicImage),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum SlideContentType {
    Paragraph,
    BigText,
    Line,
    Image,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ContentJson {
    #[serde(rename = "type")]
    pub type_: SlideContentType,
    pub content: Option<String>,
    pub rect: Option<Rect>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SlideJson {
    pub title: Option<String>,
    pub content: Vec<ContentJson>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BoxSizeJson {
    pub percent_width: u16,
    pub percent_height: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SlidesJson {
    pub box_size: BoxSizeJson,
    pub slides: Vec<SlideJson>,
}
