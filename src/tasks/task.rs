#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Task {
    inner: todo_txt::task::Extended,
    pub id: usize,
}

impl Task {
    pub fn new() -> Self {
        Self {
            inner: todo_txt::task::Extended::default(),
            id: 0,
        }
    }

    pub fn markup_subject(&self) -> String {
        let mut subject = Self::markup_escape(&self.subject);

        let regex = regex::Regex::new(r"(?P<url>[\w]+://[^\s]+)").unwrap();
        subject = regex
            .replace_all(&subject, |caps: &regex::Captures<'_>| {
                format!(
                    "<a href=\"{url}\">{url}</a>",
                    url = caps[1].replace('&', "&amp;")
                )
            })
            .into_owned();

        let regex = regex::Regex::new(r"(?P<space>^|[\s])(?P<tag>[\+@][\w\-\\]+)").unwrap();
        subject = regex
            .replace_all(&subject, "$space<b>$tag</b>")
            .into_owned();

        subject
    }

    fn markup_escape(text: &str) -> String {
        gtk::glib::markup_escape_text(text).as_str().to_string()
    }
}

impl todo_txt::Task for Task {}

impl AsRef<todo_txt::task::Simple> for Task {
    fn as_ref(&self) -> &todo_txt::task::Simple {
        &self.inner
    }
}

impl std::str::FromStr for Task {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = todo_txt::task::Extended::from_str(s)?;

        Ok(Self { inner, id: 0 })
    }
}

impl From<String> for Task {
    fn from(value: String) -> Self {
        let inner = todo_txt::task::Extended::from(value);

        Self { inner, id: 0 }
    }
}

impl std::ops::Deref for Task {
    type Target = todo_txt::task::Extended;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for Task {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::ops::Deref;

        f.write_str(format!("{}", self.deref()).as_str())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tasks::task::*;

    #[test]
    fn markup_escape() {
        let mut task = Task::new();
        task.subject = "P&T keep focus on long term +HoWE".to_string();

        assert_eq!(
            task.markup_subject(),
            "P&amp;T keep focus on long term <b>+HoWE</b>"
        );
    }
}
