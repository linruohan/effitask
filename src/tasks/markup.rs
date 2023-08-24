pub trait Markup {
    fn markup(&self) -> Option<String>;
}

impl Markup for todo_txt::task::Note {
    fn markup(&self) -> Option<String> {
        let Some(content) = self.content() else {
            return None;
        };

        let parser = pulldown_cmark::Parser::new(&content);

        let mut markup = String::from("<markup>");

        let headers = [
            "xx-large", "x-large", "large", "medium", "small", "x-small", "xx-small",
        ];

        for event in parser {
            use std::fmt::Write;

            use pulldown_cmark::Event;
            use pulldown_cmark::Tag;

            match event {
                Event::Start(Tag::Heading(level, ..)) => {
                    write!(markup, "<span font_size='{}'><u>", headers[level as usize]).ok();
                }
                Event::End(Tag::Heading(..)) => markup.push_str("</u></span>\n\n"),

                Event::Start(Tag::Paragraph) => markup.push_str("<span>"),
                Event::End(Tag::Paragraph) => markup.push_str("</span>\n"),

                Event::Start(Tag::CodeBlock(_)) => markup.push_str("<tt>"),
                Event::End(Tag::CodeBlock(_)) => markup.push_str("</tt>"),

                Event::Start(Tag::Emphasis) => markup.push_str("<i>"),
                Event::End(Tag::Emphasis) => markup.push_str("</i>"),

                Event::Start(Tag::Strong) => markup.push_str("<b>"),
                Event::End(Tag::Strong) => markup.push_str("</b>"),

                Event::Start(Tag::Item) => markup.push_str("· "),
                Event::End(Tag::Item) | Event::SoftBreak => markup.push('\n'),

                Event::Start(Tag::Link(_, link, title)) => {
                    write!(markup, "<a href='{link}' title='{title}'>").ok();
                }
                Event::End(Tag::Link(_, _, _)) => markup.push_str("</a>"),

                Event::Text(t) => markup.push_str(&t.replace('&', "&amp;")),

                _ => (),
            }
        }
        markup.push_str("</markup>");

        Some(markup)
    }
}
