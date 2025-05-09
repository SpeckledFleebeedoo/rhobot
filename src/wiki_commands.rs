use log::error;
use parse_wiki_text::{Configuration, Node};
use poise::CreateReply;
use poise::serenity_prelude::{Colour, CreateEmbed};
use serde::Deserialize;
use std::fmt::Debug;
use std::{error, fmt, fmt::Write};

use crate::formatting_tools::DiscordFormat;
use crate::{Context, Error, SEPARATOR};

#[derive(Debug)]
pub enum WikiError {
    ReqwestError(reqwest::Error),
    NoSearchResults(String),
    SendMessageFailed(serenity::Error),
    UrlParseError(url::ParseError),
}

impl fmt::Display for WikiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReqwestError(error) => write!(f, "Error retrieving wiki page: {error}"),
            Self::NoSearchResults(prompt) => write!(f, "No search results found for `{prompt}`"),
            Self::SendMessageFailed(error) => write!(f, "Failed to send message: {error}"),
            Self::UrlParseError(error) => write!(f, "Failed to parse wiki url: {error}"),
        }
    }
}

impl error::Error for WikiError {}

impl From<serenity::Error> for WikiError {
    fn from(value: serenity::Error) -> Self {
        Self::SendMessageFailed(value)
    }
}

impl From<reqwest::Error> for WikiError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<url::ParseError> for WikiError {
    fn from(value: url::ParseError) -> Self {
        Self::UrlParseError(value)
    }
}

struct NodeWrap<'a> {
    n: &'a parse_wiki_text::Node<'a>,
}

impl fmt::Display for NodeWrap<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.n {
            Node::Bold { .. } => write!(f, "**"),
            Node::BoldItalic { .. } => write!(f, "***"),
            Node::ExternalLink { nodes, .. } => {
                let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap { n: node });
                    output
                });
                match node_str.split_once(' ') {
                    Some(s) => write!(f, "[{}]({})", s.1, s.0),
                    None => write!(f, "{node_str}"),
                }
            }
            Node::Heading { level, nodes, .. } => {
                let node_strs = nodes
                    .iter()
                    .map(|node| format!("{}", NodeWrap { n: node }))
                    .collect::<Vec<String>>();
                write!(
                    f,
                    "\n||HEADING||{} {}\n",
                    "#".repeat(*level as usize),
                    node_strs.join(" ")
                )
            }
            Node::HorizontalDivider { .. } => write!(f, "\n---\n"),
            Node::Italic { .. } => write!(f, "*"),
            Node::Link { target, text, .. } => {
                let node_str = text.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap { n: node });
                    output
                });
                let target_formatted = target.replace(' ', "_");
                write!(
                    f,
                    "[{node_str}](https://wiki.factorio.com/{target_formatted})"
                )
            }
            Node::OrderedList { items, .. } => {
                let node_str = items.iter().fold(String::new(), |mut node_output, item| {
                    let _ = write!(
                        node_output,
                        "\n0. {}",
                        item.nodes
                            .iter()
                            .fold(String::new(), |mut item_output, node| {
                                let _ = write!(item_output, "{}", NodeWrap { n: node });
                                item_output
                            })
                    );
                    node_output
                });
                write!(f, "{node_str}")
            }
            Node::ParagraphBreak { .. } => write!(f, "\n\n"),
            Node::Preformatted { nodes, .. } => {
                let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap { n: node });
                    output
                });
                writeln!(f, "```{node_str}```",)
            }
            Node::StartTag {
                name: std::borrow::Cow::Borrowed("code"),
                ..
            }
            | Node::EndTag {
                name: std::borrow::Cow::Borrowed("code"),
                ..
            } => {
                write!(f, "`")
            }
            Node::Tag { name, nodes, .. } => format_tag(name, nodes, f),
            Node::Text { value, .. } => write!(f, "{value}"),
            Node::UnorderedList { items, .. } => {
                let node_str = items.iter().fold(String::new(), |mut node_output, item| {
                    let _ = write!(
                        node_output,
                        "\n- {}",
                        item.nodes
                            .iter()
                            .fold(String::new(), |mut item_output, node| {
                                let _ = write!(item_output, "{}", NodeWrap { n: node });
                                item_output
                            })
                    );
                    node_output
                });
                write!(f, "{node_str}")
            }
            Node::Template {
                name, parameters, ..
            } => format_template(name, parameters, f),
            // Node::Parameter { default, end, name, start } => todo!(),
            // Node::Category { end, ordinal, start, target } => todo!(),
            // Node::CharacterEntity { character, end, start } => todo!(),
            // Node::Comment { end, start } => todo!(),
            // Node::DefinitionList { end, items, start } => todo!(),
            // Node::Image { end, start, target, text } => todo!(),
            // Node::MagicWord { end, start } => todo!(),
            // Node::Redirect { end, target, start } => todo!(),
            // Node::Table { attributes, captions, end, rows, start } => todo!(),
            _ => Ok(()),
        }
    }
}

fn format_template(
    name: &[Node<'_>],
    parameters: &[parse_wiki_text::Parameter<'_>],
    f: &mut fmt::Formatter<'_>,
) -> Result<(), fmt::Error> {
    match name.first() {
        Some(Node::Text {
            value: "imagelink" | "Imagelink",
            ..
        }) => {
            let Some(par) = parameters.first() else {
                return Ok(());
            };
            let Some(Node::Text { value, .. }) = par.value.first() else {
                return Ok(());
            };
            // Assumes imagelinks never have a custom caption.
            write!(
                f,
                "[{value}](https://wiki.factorio.com/{})",
                value.replace(' ', "_")
            )
        }
        Some(Node::Text {
            value: "About/Space age",
            ..
        }) => {
            writeln!(
                f,
                r"_[Space Age](https://wiki.factorio.com/Space\_Age) expansion exclusive feature._"
            )
        }
        Some(Node::Text { value, .. }) if value.contains("DISPLAYTITLE") => {
            write!(
                f,
                "DISPLAYTITLE: {}",
                value.strip_prefix("DISPLAYTITLE:").unwrap()
            )
        }
        _ => Ok(()),
    }
}

fn format_tag(
    name: &str,
    nodes: &[Node<'_>],
    f: &mut fmt::Formatter<'_>,
) -> Result<(), fmt::Error> {
    match name {
        "syntaxhighlight" => {
            let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                let _ = write!(output, "{}", NodeWrap { n: node });
                output
            });
            writeln!(f, "```lua\n{node_str}```",)
        }
        "nowiki" => {
            let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                let _ = write!(output, "{}", NodeWrap { n: node });
                output
            });
            write!(f, "{node_str}")
        }
        _ => {
            let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                let _ = write!(output, "{}", NodeWrap { n: node });
                output
            });
            write!(f, "TAG {name}: {node_str}")
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct PageResponse {
    parse: Parse,
}

#[derive(Deserialize, Debug, Clone)]
struct Parse {
    title: String,
    wikitext: String,
}

// All language codes to account for future wiki expansion.
const LANG_CODES: [&str; 284] = [
    "/aa", "/ab", "/ae", "/af", "/ak", "/am", "/an", "/ar", "/ar-ae", "/ar-bh", "/ar-dz", "/ar-eg",
    "/ar-iq", "/ar-jo", "/ar-kw", "/ar-lb", "/ar-ly", "/ar-ma", "/ar-om", "/ar-qa", "/ar-sa",
    "/ar-sy", "/ar-tn", "/ar-ye", "/as", "/av", "/ay", "/az", "/ba", "/be", "/bg", "/bh", "/bi",
    "/bm", "/bn", "/bo", "/br", "/bs", "/ca", "/ce", "/ch", "/co", "/cr", "/cs", "/cu", "/cv",
    "/cy", "/da", "/de", "/de-at", "/de-ch", "/de-de", "/de-li", "/de-lu", "/div", "/dv", "/dz",
    "/ee", "/el", "/en", "/en-au", "/en-bz", "/en-ca", "/en-cb", "/en-gb", "/en-ie", "/en-jm",
    "/en-nz", "/en-ph", "/en-tt", "/en-us", "/en-za", "/en-zw", "/eo", "/es", "/es-ar", "/es-bo",
    "/es-cl", "/es-co", "/es-cr", "/es-do", "/es-ec", "/es-es", "/es-gt", "/es-hn", "/es-mx",
    "/es-ni", "/es-pa", "/es-pe", "/es-pr", "/es-py", "/es-sv", "/es-us", "/es-uy", "/es-ve",
    "/et", "/eu", "/fa", "/ff", "/fi", "/fj", "/fo", "/fr", "/fr-be", "/fr-ca", "/fr-ch", "/fr-fr",
    "/fr-lu", "/fr-mc", "/fy", "/ga", "/gd", "/gl", "/gn", "/gu", "/gv", "/ha", "/he", "/hi",
    "/ho", "/hr", "/hr-ba", "/hr-hr", "/ht", "/hu", "/hy", "/hz", "/ia", "/id", "/ie", "/ig",
    "/ii", "/ik", "/in", "/io", "/is", "/it", "/it-ch", "/it-it", "/iu", "/iw", "/ja", "/ji",
    "/jv", "/jw", "/ka", "/kg", "/ki", "/kj", "/kk", "/kl", "/km", "/kn", "/ko", "/kok", "/kr",
    "/ks", "/ku", "/kv", "/kw", "/ky", "/kz", "/la", "/lb", "/lg", "/li", "/ln", "/lo", "/ls",
    "/lt", "/lu", "/lv", "/mg", "/mh", "/mi", "/mk", "/ml", "/mn", "/mo", "/mr", "/ms", "/ms-bn",
    "/ms-my", "/mt", "/my", "/na", "/nb", "/nd", "/ne", "/ng", "/nl", "/nl-be", "/nl-nl", "/nn",
    "/no", "/nr", "/ns", "/nv", "/ny", "/oc", "/oj", "/om", "/or", "/os", "/pa", "/pi", "/pl",
    "/ps", "/pt", "/pt-br", "/pt-pt", "/qu", "/qu-bo", "/qu-ec", "/qu-pe", "/rm", "/rn", "/ro",
    "/ru", "/rw", "/sa", "/sb", "/sc", "/sd", "/se", "/se-fi", "/se-no", "/se-se", "/sg", "/sh",
    "/si", "/sk", "/sl", "/sm", "/sn", "/so", "/sq", "/sr", "/sr-ba", "/sr-sp", "/ss", "/st",
    "/su", "/sv", "/sv-fi", "/sv-se", "/sw", "/sx", "/syr", "/ta", "/te", "/tg", "/th", "/ti",
    "/tk", "/tl", "/tn", "/to", "/tr", "/ts", "/tt", "/tw", "/ty", "/ug", "/uk", "/ur", "/us",
    "/uz", "/ve", "/vi", "/vo", "/wa", "/wo", "/xh", "/yi", "/yo", "/za", "/zh", "/zh-cn",
    "/zh-hk", "/zh-mo", "/zh-sg", "/zh-tw", "/zu",
];

async fn get_mediawiki_page(name: &str) -> Result<Parse, WikiError> {
    let url = reqwest::Url::parse_with_params(
        "https://wiki.factorio.com/api.php?",
        &[
            ("action", "parse"),
            ("format", "json"),
            ("page", name),
            ("redirects", "1"),
            ("prop", "wikitext"),
            ("formatversion", "2"),
        ],
    )?;
    let response = reqwest::get(url).await?;
    let page: PageResponse = response.json().await?;
    Ok(page.parse)
}

#[derive(Deserialize, Debug)]
struct WikiData {
    _search: String,
    titles: Vec<String>,
    _descriptions: Vec<String>,
    _urls: Vec<String>,
}

pub async fn opensearch_mediawiki(name: &str) -> Result<Vec<String>, WikiError> {
    let url = reqwest::Url::parse_with_params(
        "https://wiki.factorio.com/api.php",
        &[
            ("action", "opensearch"),
            ("format", "json"),
            ("search", name),
            ("namespace", "0|3000"),
            ("limit", "100"),
            ("formatversion", "2"),
        ],
    )?;
    let response = reqwest::get(url).await?;
    let json: WikiData = response.json().await?;
    if json.titles.is_empty() {
        return Ok(vec![]);
    };

    let mut output = Vec::new();

    for name in json.titles {
        if LANG_CODES.iter().any(|&langcode| name.ends_with(langcode)) {
            continue;
        };
        output.push(name);
    }
    Ok(output)
}

/// Link a wiki page. Can also be used inline with [[wiki search]].
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn wiki(
    ctx: Context<'_>,
    #[description = "Wiki page name"]
    #[autocomplete = "autocomplete_wiki"]
    #[rest]
    name: String,
) -> Result<(), Error> {
    let mut command = name.split(SEPARATOR).next().unwrap_or(&name).trim();
    if command.is_empty() {
        command = "Main Page";
    };

    let search_result: String = match ctx {
        poise::Context::Application(_) => command.to_owned(),
        poise::Context::Prefix(_) => {
            let results = opensearch_mediawiki(command).await?;
            let res = results
                .first()
                .ok_or_else(|| WikiError::NoSearchResults(command.to_owned()))?;
            res.to_owned()
        }
    };

    let embed = get_wiki_page(&search_result).await?;
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

fn get_factorio_wiki_parser_config() -> Configuration {
    // Parser configuration for wiki.factorio.com
    Configuration::new(&parse_wiki_text::ConfigurationSource {
        category_namespaces: &["category"],
        extension_tags: &[
            "charinsert",
            "gallery",
            "imagemap",
            "indicator",
            "info",
            "langconvert",
            "nowiki",
            "pre",
            "section",
            "seo",
            "smwdoc",
            "source",
            "syntaxhighlight",
            "tabber",
        ],
        file_namespaces: &["file", "image"],
        link_trail: "abcdefghijklmnopqrstuvwxyz",
        magic_words: &[
            "expectunusedcategory",
            "forcetoc",
            "hiddencat",
            "index",
            "newsectionlink",
            "nocc",
            "nocontentconvert",
            "noeditsection",
            "nofactbox",
            "nogallery",
            "noindex",
            "nonewsectionlink",
            "notc",
            "notitleconvert",
            "notoc",
            "showfactbox",
            "staticredirect",
            "toc",
        ],
        protocols: &[
            "//",
            "bitcoin:",
            "ftp://",
            "ftps://",
            "geo:",
            "git://",
            "gopher://",
            "http://",
            "https://",
            "irc://",
            "ircs://",
            "magnet:",
            "mailto:",
            "mms://",
            "news:",
            "nntp://",
            "redis://",
            "sftp://",
            "sip:",
            "sips:",
            "sms:",
            "ssh://",
            "svn://",
            "tel:",
            "telnet://",
            "urn:",
            "worldwind://",
            "xmpp:",
        ],
        redirect_magic_words: &["redirect"],
    })
}

pub async fn get_wiki_page(search_result: &str) -> Result<CreateEmbed, WikiError> {
    let article = get_mediawiki_page(search_result).await?;

    let parsed_text = get_factorio_wiki_parser_config()
        .parse(&article.wikitext)
        .nodes
        .iter()
        .fold(String::new(), |mut output, n| {
            let _ = write!(output, "{}", NodeWrap { n });
            output
        });

    let sections = parsed_text.split("||HEADING||").collect::<Vec<&str>>();

    let formatted_text = match sections.len() {
        0 => String::new(),
        1 => sections[0].to_owned(),
        _ => {
            if sections[0].len() < 100 {
                sections[0].to_owned() + sections[1]
            } else {
                sections[0].to_owned()
            }
        }
    };
    let embed = CreateEmbed::new()
        .title(article.title.truncate_for_embed(256))
        .url(format!(
            "https://wiki.factorio.com/{}",
            &article.title.replace(' ', "_")
        ))
        .description(formatted_text.truncate_for_embed(2048))
        .color(Colour::ORANGE);
    Ok(embed)
}

async fn autocomplete_wiki(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    if partial.is_empty() {
        return vec!["Main Page".to_owned()];
    }
    match opensearch_mediawiki(partial).await {
        Ok(r) => r,
        Err(e) => {
            error!("Error searching wiki: {e}");
            vec![]
        }
    }
}
