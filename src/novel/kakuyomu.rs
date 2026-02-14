use scraper::html::Html;
use scraper::selector::Selector;
use serde_ignored_type::IgnoredAny;

use core::fmt::{self, Write};
use std::borrow::Cow;

use super::{Title, Id, IdBuffer};
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
                    Some(mut start_idx) => {
                        //If author used brackets in his name, then we need to account for that
                        //So we count number of round brackets
                        let mut sub_end_count = 0usize;
                        let mut author_sub = author;
                        while let Some(nested_idx) = author_sub.rfind(AUTHOR_END) {
                            sub_end_count = sub_end_count.saturating_add(1);
                            author_sub = &author_sub[..nested_idx];
                        }

                        //if there is nested closing brackets, then skip equal number of opening brackets
                        while sub_end_count > 0 {
                            if let Some(new_idx) = author[..start_idx - AUTHOR_START.len_utf8()].rfind(AUTHOR_START) {
                                sub_end_count -= 1;
                                start_idx = new_idx;
                            } else {
                                break;
                            }
                        }
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

#[derive(Debug)]
#[repr(transparent)]
pub struct Chapter {
    id: IdBuffer,
}

impl super::Chapter for Chapter {
    fn preapre_url(&self, id: &Id, out: &mut String) {
        let url = id.url();
        let id = self.id.as_str();
        //writing string cannot fail (aside from OOM)
        let _ = write!(out, "{url}/episodes/{id}");
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
}

impl NovelInfo {
    pub fn new(id: Id, html: &str) -> Self {
        Self {
            id,
            inner: Html::parse_document(html)
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

        fmt.finish()
    }

}

impl super::NovelInfo for NovelInfo {
    type Chapter = Chapter;
    type ChapterIter = std::vec::IntoIter<Self::Chapter>;

    #[inline(always)]
    fn id(&self) -> &Id {
        &self.id
    }

    fn title(&self) -> super::Result<Title<'_>> {
        let title = Selector::parse("title").unwrap();
        match self.inner.select(&title).next().and_then(|title| title.text().next()) {
            Some(title) => Ok(Title::new_kakuyomu(title)),
            None => Err(super::Error::MissingIndex),
        }
    }

    fn chapters(&mut self) -> super::Result<Self::ChapterIter> {
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
