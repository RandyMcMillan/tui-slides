use std::io::Read;
use std::path::Path;

use block::Position;
use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{block::Title, *},
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, Image, Resize, StatefulImage};
use syntect::{
    easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::LinesWithEndings,
};
use syntect_tui::into_span;
use tokio::sync::mpsc::UnboundedSender;
use tui_big_text::{BigText, PixelSize};

use super::{Component, Frame};
use crate::{
    action::Action,
    enums::{ContentJson, ReturnSlideWidget, SlideContentType, SlideJson, SlidesJson},
    layout::{get_slides_layout, CONTENT_HEIGHT, CONTENT_WIDTH},
    slide_builder::{
        get_slide_content_string, make_slide_block, make_slide_content, make_slide_image,
    },
};

pub struct Slides {
    action_tx: Option<UnboundedSender<Action>>,
    json_slides: String,
    slides: Option<SlidesJson>,
    slide_index: usize,
    slide_count: usize,
    picker: Picker,
    images: Vec<Box<dyn StatefulProtocol>>,
}

impl Default for Slides {
    fn default() -> Self {
        Self::new()
    }
}

impl Slides {
    pub fn new() -> Self {
        Self {
            action_tx: None,
            json_slides: String::from(""),
            slides: None,
            slide_index: 0,
            slide_count: 0,
            picker: Picker::from_termios().unwrap(),
            images: vec![],
        }
    }

    fn get_json_slides(&mut self) {
        let error_string = format!(
            "file: '{}' failed to open slides json file",
            self.json_slides
        );
        let mut f = std::fs::File::open(self.json_slides.clone()).expect(&error_string);
        let mut f_content = String::new();
        f.read_to_string(&mut f_content)
            .expect("Failed to read json slides file");
        let slides: SlidesJson = serde_json::from_str(&f_content).unwrap();

        self.slides = Some(slides);
        if let Some(slides) = &self.slides {
            self.slide_count = slides.slides.len();
        }
    }

    fn get_slide(&self) -> SlideJson {
        if let Some(slides) = &self.slides {
            return slides.slides[self.slide_index].clone();
        }
        SlideJson {
            title: None,
            content: vec![],
        }
    }

    fn get_slide_rect(&self, rect: Rect, item_rect: Option<Rect>) -> Rect {
        let mut slide_rect = Rect::new(rect.x, rect.y, rect.width, rect.height);
        if let Some(slides) = &self.slides {
            if let Some(s_content_rect) = item_rect {
                slide_rect.x += s_content_rect.x;
                slide_rect.y += s_content_rect.y;
                slide_rect.width = s_content_rect.width;
                slide_rect.height = s_content_rect.height;
            }
        }
        slide_rect
    }

    fn store_images(&mut self) {
        self.images.clear();

        let f_path = Path::new(&self.json_slides);
        let img_path = f_path.parent().unwrap();
        let slide = self.get_slide();

        for item in slide.content {
            if item.type_ == SlideContentType::Image {
                let d_img = make_slide_image(item, self.json_slides.clone());
                if let ReturnSlideWidget::Image(dyn_img) = d_img {
                    let img_static = self.picker.new_resize_protocol(dyn_img);
                    self.images.push(img_static);
                }
            }
        }
    }

    fn next_slide(&mut self) {
        let mut s_index = self.slide_index + 1;
        s_index %= self.slide_count;
        self.slide_index = s_index;

        self.store_images();
    }

    fn previous_slide(&mut self) {
        let mut s_index = self.slide_index;
        if self.slide_index.checked_sub(1).is_none() {
            s_index = self.slide_count - 1;
        } else {
            s_index -= 1;
        }
        self.slide_index = s_index;

        self.store_images();
    }

    fn make_title<'a>(slide: &SlideJson) -> BigText<'a> {
        let mut title_text = "__title__".to_string();
        if let Some(title) = &slide.title {
            title_text = title.to_string();
        }

        let big_title = BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .lines(vec![title_text.green().into()])
            .alignment(Alignment::Center)
            .build();
        big_title.unwrap()
    }

    fn make_block(title: Option<Line>) -> Block {
        let s_content = ContentJson {
            type_: SlideContentType::Block,
            color: Some("#FFDDDD".to_string()),
            ..Default::default()
        };
        let block = make_slide_block(s_content);
        if let ReturnSlideWidget::Block(mut b) = block {
            if let Some(t) = title {
                b = b.title(Title::from(t));
            }
            return b;
        }

        // -- default
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(100, 100, 100)));
        if let Some(t) = title {
            block = block.title(Title::from(t));
        }
        block
    }

    fn make_content_block(&self) -> Block {
        let s_index = self.slide_index + 1;
        let title = Line::from(vec![
            "|".yellow(),
            s_index.to_string().green(),
            "/".yellow(),
            self.slide_count.to_string().green(),
            "|".yellow(),
        ]);
        Self::make_block(None)
            .title_bottom(title)
            .title_alignment(Alignment::Right)
            .border_type(BorderType::Rounded)
    }

    fn make_slide_items<'a>(
        slide: &SlideJson,
        json_slides: String,
    ) -> Vec<(
        ReturnSlideWidget<'a>,
        String,
        Option<Rect>,
        Option<Vec<u64>>,
    )> {
        let mut slide_items = vec![];
        for item in &slide.content {
            slide_items.push((
                make_slide_content(item.clone(), json_slides.clone()),
                get_slide_content_string(&item),
                item.rect,
                item.data.clone(),
            ));
        }
        slide_items
    }
}

impl Component for Slides {
    fn init(&mut self, area: Rect, json_slides: String) -> Result<()> {
        self.json_slides = json_slides;
        self.picker.guess_protocol();
        self.get_json_slides();
        self.store_images();
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Next => {
                self.next_slide();
            }
            Action::Previous => {
                self.previous_slide();
            }
            Action::Reload => {
                self.get_json_slides();
                self.store_images();
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let mut box_width = CONTENT_WIDTH;
        let mut box_height = CONTENT_HEIGHT;
        if let Some(slides) = &self.slides {
            box_width = slides.box_size.width;
            box_height = slides.box_size.height;
        }

        let rect = get_slides_layout(area, box_width, box_height);
        let title_rect = Rect::new(
            rect.content.x,
            rect.content.y + 2,
            rect.content.width,
            rect.content.height,
        );

        let slide = self.get_slide();

        let slide_items = Self::make_slide_items(&slide, self.json_slides.clone());
        let title = Self::make_title(&slide);
        let block = self.make_content_block();

        f.render_widget(title, title_rect);
        f.render_widget(block, rect.content);

        // -- render slide widgets
        let mut img_index = 0;
        for (slide, c, r, d) in slide_items {
            let slide_rect = self.get_slide_rect(rect.content, r);

            let mut data = vec![];
            if let Some(d1) = d {
                data = d1;
            }

            match slide {
                ReturnSlideWidget::Paragraph(s) => {
                    f.render_widget(s, slide_rect);
                }
                ReturnSlideWidget::Line(s) => {
                    f.render_widget(s, slide_rect);
                }
                ReturnSlideWidget::BigText(s) => {
                    f.render_widget(s, slide_rect);
                }
                ReturnSlideWidget::Image(s) => {
                    // -- block | borders
                    let block = Self::make_block(None)
                        .style(Style::default().bg(Color::Black))
                        .border_style(Style::default().fg(Color::Rgb(100, 100, 100)));
                    let mut b_rect = slide_rect;
                    b_rect.x -= 1;
                    b_rect.width += 2;
                    b_rect.y -= 1;
                    f.render_widget(block, b_rect);

                    // -- image
                    let mut img_static = self.images[img_index].clone();
                    let img = StatefulImage::new(None).resize(Resize::Fit(None));
                    f.render_stateful_widget(img, slide_rect, &mut img_static);
                    img_index += 1;
                }
                ReturnSlideWidget::Block(s) => {
                    f.render_widget(s, slide_rect);
                }
                ReturnSlideWidget::Sparkline(mut s) => {
                    s = s.data(&data);
                    f.render_widget(s, slide_rect);
                }
                ReturnSlideWidget::CodeHighlight(mut l) => {
                    let ps = SyntaxSet::load_defaults_newlines();
                    let ts = ThemeSet::load_defaults();
                    let syntax = ps.find_syntax_by_extension("rs").unwrap();
                    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);

                    let mut lines: Vec<Line> = vec![];
                    let c_lines: Vec<&str> = c.split('\n').collect();

                    for c_line in c_lines {
                        for line in LinesWithEndings::from(c_line) {
                            let l_spans: Vec<Span> = h
                                .highlight_line(line, &ps)
                                .unwrap()
                                .into_iter()
                                .filter_map(|seg| into_span(seg).ok())
                                .collect();
                            let line = Line::from(l_spans);
                            lines.push(line);
                        }
                    }
                    l = Paragraph::new(lines);

                    f.render_widget(l, slide_rect);
                }
            }
        }
        Ok(())
    }
}
