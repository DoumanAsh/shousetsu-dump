use scraper::html::Html;
use scraper::selector::Selector;
use serde_ignored_type::IgnoredAny;

use core::fmt;
use std::borrow::Cow;

use super::{Title, Id, IdBuffer, Chapter, Line};
use crate::http;

impl<'a> Title<'a> {
    pub fn new_kakuyomu(mut title: &'a str) -> Self {
        const AUTHOR_END: char = '）';
        const AUTHOR_START: char = '（';

        title = title.trim();
        if let Some(stripped) = title.strip_suffix(" - カクヨム") {
            title = stripped;
        }
        let author = match title.rfind(AUTHOR_END) {
            Some(idx) => {
                let mut author = &title[..idx];
                //Make sure we have opening bracket
                match author.rfind(AUTHOR_START) {
                    Some(start_idx) => {
                        //TODO consider authors that use brackets inside their name
                        //     it is dangerous to bluntly look for next brackets as title also can have brackets inside
                        author = &author[start_idx + AUTHOR_START.len_utf8()..];
                        title = &title[..start_idx];

                        Some(author)
                    },
                    None => None
                }
            },
            None => None,
        };

        Self {
            name: title,
            author,
        }
    }
}

#[derive(Debug, serde_derive::Deserialize)]
struct ScriptState {
    props: Props,
}

#[allow(non_snake_case)]
#[derive(Debug, serde_derive::Deserialize)]
struct Props {
    pageProps: PageProps,
}

#[allow(non_snake_case)]
#[derive(Debug, serde_derive::Deserialize)]
struct PageProps {
    __APOLLO_STATE__: ApolloState
}

#[derive(Debug)]
struct ApolloState {
    chapters: Vec<Chapter>,
}

struct ApolloStateVisitor;

impl<'de> serde::de::Visitor<'de> for ApolloStateVisitor {
    type Value = ApolloState;
    #[inline(always)]
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("Expected __APOLLO_STATE__ to contain JSON")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        const SUFFIX: &str = "Episode:";

        let mut chapters = Vec::new();
        while let Some(entry) = map.next_key::<Cow<'de, str>>()? {
            if let Some(chapter) = entry.strip_prefix(SUFFIX) {
                chapters.push(Chapter {
                    id: match IdBuffer::from_str_checked(chapter) {
                        Ok(id) => id,
                        Err(_) => return Err(serde::de::Error::custom(format!("{chapter}: not a valid chapter id")))
                    }
                })
            }
            let _ = map.next_value::<IgnoredAny>();
        }

        Ok(ApolloState {
            chapters
        })
    }
}

impl<'de> serde::Deserialize<'de> for ApolloState {
    #[inline(always)]
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ApolloStateVisitor)
    }
}

#[derive(Debug)]
pub struct Index<'a> {
    pub title: Option<&'a str>,
    pub chapters: Vec<Chapter>
}

pub struct NovelInfo {
    id: Id,
    inner: Html,
    selectors: ChapterSelector,
}

impl NovelInfo {
    pub fn new(id: Id, html: &str) -> Self {
        Self {
            id,
            inner: Html::parse_document(html),
            selectors: ChapterSelector::new(),
        }
    }

    pub fn get_index(&self) -> Option<Result<Index<'_>, serde_json::Error>> {
        let title = Selector::parse("title").unwrap();
        let title = self.inner.select(&title).next().and_then(|title| {
            title.text().next()
        });

        let selector = Selector::parse("script").unwrap();
        for elem in self.inner.select(&selector) {
            let element = elem.value();
            match (element.attr("type"), element.attr("id")) {
                (Some("application/json"), Some("__NEXT_DATA__")) => {
                    if let Some(json) = elem.text().next() {
                        return Some(serde_json::from_str::<ScriptState>(json).map(|result| Index {
                            title,
                            chapters: result.props.pageProps.__APOLLO_STATE__.chapters
                        }));
                    } else {
                        continue;
                    }
                },
                _ => (),
            }
        }
        None
    }
}

impl fmt::Debug for NovelInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fmt = fmt.debug_struct("Novel Info");
        fmt.field("Id", &self.id.id())
           .field("Url", &self.id.url());

        if let Ok(title) = super::NovelInfo::title(self) {
            fmt.field("Title", &title.name);
            if let Some(author) = title.author {
                fmt.field("Author", &author);
            }
        }
        if let Ok(chapters) = super::NovelInfo::chapters(self) {
           fmt.field("Number of Chapter", &chapters.len());
        }

        fmt.finish()
    }

}

struct ChapterContent<'a> {
    html: scraper::Html,
    selectors: &'a ChapterSelector,
}

impl super::ChapterContent for ChapterContent<'_> {
    fn title(&self) -> super::Result<String> {
        match self.html.select(&self.selectors.title).next() {
            Some(title) => Ok(title.inner_html()),
            None => Err(super::Error::invalid_chapter_content("Cannot find .widget-episodeTitle with title".into())),
        }
    }

    fn lines(&self) -> super::Result<impl Iterator<Item = Line> + '_> {
        let body = match self.html.select(&self.selectors.body).next() {
            Some(body) => body,
            None => return Err(super::Error::invalid_chapter_content("Cannot find .widget-episodeBody.js-episode-body with text".into())),
        };

        Ok(body.select(&self.selectors.line).map(Line::new_kakuyomu))
    }
}

impl super::NovelInfo for NovelInfo {
    type ChapterIter = std::vec::IntoIter<Chapter>;

    #[inline(always)]
    fn id(&self) -> &Id {
        &self.id
    }

    fn title(&self) -> super::Result<Title<'_>> {
        let title = Selector::parse("title").unwrap();
        match self.inner.select(&title).next().and_then(|title| title.text().next()) {
            Some(title) => Ok(Title::new_kakuyomu(title)),
            None => Err(super::Error::MissingTitle),
        }
    }

    fn chapters(&self) -> super::Result<Self::ChapterIter> {
        let script = Selector::parse("script").unwrap();

        for elem in self.inner.select(&script) {
            let element = elem.value();
            match (element.attr("type"), element.attr("id")) {
                (Some("application/json"), Some("__NEXT_DATA__")) => {
                    if let Some(json) = elem.text().next() {
                        return serde_json::from_str::<ScriptState>(json).map(|result| result.props.pageProps.__APOLLO_STATE__.chapters.into_iter()).map_err(super::Error::InvalidKakuyomuChapters);
                    } else {
                        continue;
                    }
                },
                _ => continue,
            }
        }

        Err(super::Error::MissingChapters)
    }

    fn extract_chapter_content<'a>(&'a self, body: &'a str) -> impl super::ChapterContent + 'a {
        ChapterContent {
            html: scraper::Html::parse_fragment(body),
            selectors: &self.selectors,
        }
    }
}

pub struct ChapterSelector {
    body: Selector,
    line: Selector,
    title: Selector,
}

impl ChapterSelector {
    pub fn new() -> Self {
        Self {
            body: Selector::parse(".widget-episodeBody.js-episode-body").unwrap(),
            line: Selector::parse("p").unwrap(),
            title: Selector::parse(".widget-episodeTitle").unwrap()
        }
    }
}

impl Line {
    #[inline(always)]
    fn new_kakuyomu(line: scraper::ElementRef<'_>) -> Self {
        match line.attr("class") {
            Some("blank") => Self::Break,
            _ => Self::Paragraph(line.inner_html()),
        }
    }
}

pub fn fetch_novel_info(id: Id, http: &http::Client) -> super::Result<NovelInfo> {
    let url = id.url().to_string();
    let html: String = match http.get(&url) {
        Ok(resp) => resp,
        Err(http::Error::StatusFailed(404)) => return Err(super::Error::NotFound),
        Err(error) => return Err(super::Error::Transient(error)),
    };

    Ok(NovelInfo::new(id, &html))
}
