use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{layout, mouse, renderer, Layout, Widget};
use iced::alignment::Vertical;
use iced::{Element, Length, Pixels, Rectangle, Renderer as IcedRenderer, Size, Theme};

const MAX_FONT_SIZE: f32 = 22.0;
const MIN_FONT_SIZE: f32 = 8.0;
const MAX_LINES: f32 = 2.0;

type RendererParagraph = <IcedRenderer as text::Renderer>::Paragraph;

pub fn name_box<'a, Message: 'a>(name_text: impl Into<String>, width: f32, height: f32, wrap_width: f32) -> Element<'a, Message> {
    Element::new(NameBox { content: name_text.into(), width, height, wrap_width })
}

struct NameBox {
    content: String,
    width: f32,
    height: f32,
    wrap_width: f32,
}

impl<Message> Widget<Message, Theme, IcedRenderer> for NameBox {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<RendererParagraph>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(RendererParagraph::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(self.height))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &IcedRenderer, limits: &layout::Limits) -> layout::Node {
        layout::sized(limits, Length::Fixed(self.width), Length::Fixed(self.height), |limits| {
            let bounds = limits.max();
            let font = renderer.default_font();

            let mut size = MAX_FONT_SIZE;
            let paragraph = loop {
                let candidate = RendererParagraph::with_text(text::Text {
                    content: self.content.as_str(),
                    bounds: Size::new(self.wrap_width, f32::INFINITY),
                    size: Pixels(size),
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: Vertical::Center,
                    shaping: text::Shaping::Advanced,
                    wrapping: text::Wrapping::Word,
                });

                let line_height = text::LineHeight::default().to_absolute(Pixels(size)).0;
                let lines = (candidate.min_bounds().height / line_height).round();
                let fits = lines <= MAX_LINES;

                if fits || size <= MIN_FONT_SIZE {
                    break candidate;
                }

                size -= 0.5;
            };

            *tree.state.downcast_mut::<RendererParagraph>() = paragraph;

            bounds
        })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let paragraph = tree.state.downcast_ref::<RendererParagraph>();

        widget::text::draw(renderer, style, layout.bounds(), paragraph, widget::text::Style { color: None }, viewport);
    }
}
