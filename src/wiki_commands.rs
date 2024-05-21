use parse_wiki_text::{Node, Configuration};
use poise::serenity_prelude::{CreateEmbed, Colour};
use poise::CreateReply;
use crate::{custom_errors::CustomError, Context, Error};
use std::{fmt, fmt::Write};
use serde::Deserialize;

struct NodeWrap<'a>{n: &'a parse_wiki_text::Node<'a>}

impl fmt::Display for NodeWrap<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.n {
            Node::Bold { .. } => write!(f, "**"),
            Node::BoldItalic { .. } => write!(f, "***"),
            Node::ExternalLink { nodes, .. } => {
                let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap{n: node});
                    output
                });
                match node_str.split_once(' ') {
                    Some(s) => write!(f, "[{}]({})", s.1, s.0),
                    None => write!(f, "{node_str}"),
                }
            },
            Node::Heading { level, nodes, .. } => {
                let node_strs = nodes.iter().map(|node| format!("{}", NodeWrap{n: node})).collect::<Vec<String>>();
                write!(f, "\n||HEADING||{} {}\n", "#".repeat(*level as usize), node_strs.join(" "))
            },
            Node::HorizontalDivider { .. } => write!(f, "\n---\n"),
            Node::Italic { .. } => write!(f, "*"),
            Node::Link { target, text, .. } => {
                let node_str = text.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap{n: node});
                    output
                });
                let target_formatted = target.replace(' ', "_");
                write!(f, "[{node_str}](https://wiki.factorio.com/{target_formatted})")
            },
            Node::OrderedList { items, .. } => {
                let node_str = items.iter().fold(String::new(), |mut node_output, item| {
                    let _ = write!(node_output, "\n0. {}", item.nodes.iter().fold(String::new(), |mut item_output, node| {
                        let _ = write!(item_output, "{}", NodeWrap{n: node});
                        item_output
                    }));
                    node_output
                });
                write!(f, "{node_str}")
            },
            Node::ParagraphBreak { .. } => write!(f, "\n\n"),
            Node::Preformatted { nodes, .. } => {
                let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                    let _ = write!(output, "{}", NodeWrap{n: node});
                    output
                });
                writeln!(f, "```{node_str}```",)
            },
            Node::StartTag { name: std::borrow::Cow::Borrowed("code"), .. } | 
            Node::EndTag { name: std::borrow::Cow::Borrowed("code"), ..} => {
                write!(f, "`")
            },
            Node::Tag { name, nodes, .. } => {
                match name.as_ref() {
                    "syntaxhighlight" => {
                        let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                            let _ = write!(output, "{}", NodeWrap{n: node});
                            output
                        });
                        writeln!(f, "```lua\n{node_str}```",)
                    },
                    "nowiki" => {
                        let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                            let _ = write!(output, "{}", NodeWrap{n: node});
                            output
                        });
                        write!(f, "{node_str}")
                    },
                    _ => {
                        let node_str = nodes.iter().fold(String::new(), |mut output, node| {
                            let _ = write!(output, "{}", NodeWrap{n: node});
                            output
                        });
                        write!(f, "TAG {name}: {node_str}")
                    },
                }
            },
            Node::Text { value, .. } => write!(f, "{value}"),
            Node::UnorderedList { items, .. } => {
                let node_str = items.iter().fold(String::new(), |mut node_output, item| {
                    let _ = write!(node_output, "\n- {}", item.nodes.iter().fold(String::new(), |mut item_output, node| {
                        let _ = write!(item_output, "{}", NodeWrap{n: node});
                        item_output
                    }));
                    node_output
                });
                write!(f, "{node_str}")
            },
            // Node::Parameter { default, end, name, start } => todo!(),
            // Node::Category { end, ordinal, start, target } => todo!(),
            // Node::CharacterEntity { character, end, start } => todo!(),
            // Node::Comment { end, start } => todo!(),
            // Node::DefinitionList { end, items, start } => todo!(),
            // Node::Image { end, start, target, text } => todo!(),
            // Node::MagicWord { end, start } => todo!(),
            // Node::Redirect { end, target, start } => todo!(),
            // Node::Table { attributes, captions, end, rows, start } => todo!(),
            // Node::Template { end, name, parameters, start } => todo!(),
            _ => write!(f, "")
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

async fn get_mediawiki_page(name: &str) -> Result<Parse, Error> {
    let url = reqwest::Url::parse_with_params("https://wiki.factorio.com/api.php?", &[
            ("action", "parse"),
            ("format", "json"),
            ("page", name),
            ("redirects", "1"),
            ("prop", "wikitext"),
            ("formatversion", "2"),
            ])?;
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

pub async fn opensearch_mediawiki(name: &str) -> Result<Vec<String>, Error> {
    let url = reqwest::Url::parse_with_params("https://wiki.factorio.com/api.php", &[
        ("action", "opensearch"),
        ("format", "json"),
        ("search", name),
        ("namespace", "0|3000"),
        ("limit", "100"),
        ("formatversion", "2")
    ])?;
    let response = reqwest::get(url).await?;
    let json: WikiData = response.json().await?;
    if json.titles.is_empty() {
        return Ok(vec![]);
    };
    // All language codes to account for future wiki expansion. 
    let langcodes = ["/aa", "/ab", "/ae", "/af", "/ak", "/am", "/an", "/ar", "/ar-ae", "/ar-bh", 
    "/ar-dz", "/ar-eg", "/ar-iq", "/ar-jo", "/ar-kw", "/ar-lb", "/ar-ly", "/ar-ma", "/ar-om", "/ar-qa", "/ar-sa", 
    "/ar-sy", "/ar-tn", "/ar-ye", "/as", "/av", "/ay", "/az", "/ba", "/be", "/bg", "/bh", "/bi", "/bm", "/bn", 
    "/bo", "/br", "/bs", "/ca", "/ce", "/ch", "/co", "/cr", "/cs", "/cu", "/cv", "/cy", "/da", "/de", "/de-at", 
    "/de-ch", "/de-de", "/de-li", "/de-lu", "/div", "/dv", "/dz", "/ee", "/el", "/en", "/en-au", "/en-bz", "/en-ca", 
    "/en-cb", "/en-gb", "/en-ie", "/en-jm", "/en-nz", "/en-ph", "/en-tt", "/en-us", "/en-za", "/en-zw", "/eo", 
    "/es", "/es-ar", "/es-bo", "/es-cl", "/es-co", "/es-cr", "/es-do", "/es-ec", "/es-es", "/es-gt", "/es-hn", 
    "/es-mx", "/es-ni", "/es-pa", "/es-pe", "/es-pr", "/es-py", "/es-sv", "/es-us", "/es-uy", "/es-ve", "/et", 
    "/eu", "/fa", "/ff", "/fi", "/fj", "/fo", "/fr", "/fr-be", "/fr-ca", "/fr-ch", "/fr-fr", "/fr-lu", "/fr-mc", 
    "/fy", "/ga", "/gd", "/gl", "/gn", "/gu", "/gv", "/ha", "/he", "/hi", "/ho", "/hr", "/hr-ba", "/hr-hr", "/ht", 
    "/hu", "/hy", "/hz", "/ia", "/id", "/ie", "/ig", "/ii", "/ik", "/in", "/io", "/is", "/it", "/it-ch", "/it-it", 
    "/iu", "/iw", "/ja", "/ji", "/jv", "/jw", "/ka", "/kg", "/ki", "/kj", "/kk", "/kl", "/km", "/kn", "/ko", "/kok", 
    "/kr", "/ks", "/ku", "/kv", "/kw", "/ky", "/kz", "/la", "/lb", "/lg", "/li", "/ln", "/lo", "/ls", "/lt", "/lu", 
    "/lv", "/mg", "/mh", "/mi", "/mk", "/ml", "/mn", "/mo", "/mr", "/ms", "/ms-bn", "/ms-my", "/mt", "/my", "/na", 
    "/nb", "/nd", "/ne", "/ng", "/nl", "/nl-be", "/nl-nl", "/nn", "/no", "/nr", "/ns", "/nv", "/ny", "/oc", "/oj", 
    "/om", "/or", "/os", "/pa", "/pi", "/pl", "/ps", "/pt", "/pt-br", "/pt-pt", "/qu", "/qu-bo", "/qu-ec", "/qu-pe", 
    "/rm", "/rn", "/ro", "/ru", "/rw", "/sa", "/sb", "/sc", "/sd", "/se", "/se-fi", "/se-no", "/se-se", "/sg", "/sh", 
    "/si", "/sk", "/sl", "/sm", "/sn", "/so", "/sq", "/sr", "/sr-ba", "/sr-sp", "/ss", "/st", "/su", "/sv", "/sv-fi", 
    "/sv-se", "/sw", "/sx", "/syr", "/ta", "/te", "/tg", "/th", "/ti", "/tk", "/tl", "/tn", "/to", "/tr", "/ts", 
    "/tt", "/tw", "/ty", "/ug", "/uk", "/ur", "/us", "/uz", "/ve", "/vi", "/vo", "/wa", "/wo", "/xh", "/yi", "/yo", 
    "/za", "/zh", "/zh-cn", "/zh-hk", "/zh-mo", "/zh-sg", "/zh-tw", "/zu"];

    let mut output = Vec::new();

    for name in json.titles {
        if langcodes.iter().any(|&langcode| name.ends_with(langcode)) {
            continue
        };
        output.push(name);
    };
    Ok(output)
}

/// Link a wiki page
#[poise::command(prefix_command, slash_command, track_edits)]
pub async fn wiki(
    ctx: Context<'_>,
    #[description = "Wiki page name"]
    #[autocomplete = "autocomplete_wiki"]
    #[rest]
    name: String,
) -> Result<(), Error> {
    let search_result: String = match ctx {
        poise::Context::Application(_) => name,
        poise::Context::Prefix(_) => {
            let results = opensearch_mediawiki(&name).await?;
            let Some(res) = results.first() else {
                return Err(Box::new(CustomError::new("Wiki search returned no results")))
            };
            res.to_owned()
        },
    };
    
    let embed = get_wiki_page(&search_result).await?;
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())

}

pub async fn get_wiki_page(search_result: &str) -> Result<CreateEmbed, Error> {
    let article = get_mediawiki_page(search_result).await?;

    // Parser configuration for wiki.factorio.com
    let configuration = Configuration::new(&parse_wiki_text::ConfigurationSource { 
        category_namespaces : &["category"],
        extension_tags : &["charinsert", "gallery", "imagemap", "indicator", "info", "langconvert", 
                "nowiki", "pre", "section", "seo", "smwdoc", "source", "syntaxhighlight", "tabber"], 
        file_namespaces : &["file", "image"], 
        link_trail : "abcdefghijklmnopqrstuvwxyz", 
        magic_words : &["expectunusedcategory", "forcetoc", "hiddencat", "index", "newsectionlink", 
                "nocc", "nocontentconvert", "noeditsection", "nofactbox", "nogallery", "noindex", 
                "nonewsectionlink", "notc", "notitleconvert", "notoc", "showfactbox", "staticredirect", "toc"], 
        protocols : &["//", "bitcoin:", "ftp://", "ftps://", "geo:", "git://", "gopher://", "http://", 
                "https://", "irc://", "ircs://", "magnet:", "mailto:", "mms://", "news:", "nntp://", 
                "redis://", "sftp://", "sip:", "sips:", "sms:", "ssh://", "svn://", "tel:", "telnet://", 
                "urn:", "worldwind://", "xmpp:"], 
        redirect_magic_words : &["redirect"],
    });
    let parsed = configuration.parse(&article.wikitext);

    let parsed_text = parsed.nodes.iter().fold(String::new(), |mut output, n| {
        let _ = write!(output, "{}", NodeWrap{n});
        output
    });
    let sections = parsed_text.split("||HEADING||").collect::<Vec<&str>>();
    let mut formatted_text: String;
    match sections.len() {
        0 => formatted_text = String::new(),
        1 => formatted_text = sections[0].to_owned(),
        _ => {
            if sections[0].len() < 100 {
                formatted_text = sections[0].to_owned() + sections[1];
            } else {
                formatted_text = sections[0].to_owned();
            };
        },
    };
    formatted_text.truncate(2048);
    let embed = CreateEmbed::new()
        .title(&article.title)
        .url(format!("https://wiki.factorio.com/{}", &article.title.replace(' ', "_")))
        .description(&formatted_text)
        .color(Colour::ORANGE);
    Ok(embed)
}

async fn autocomplete_wiki<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String> {
    if partial.is_empty() {
        return vec!["Main Page".to_owned()]
    }
    match opensearch_mediawiki(partial).await {
        Ok(r) => r,
        Err(e) => {
            println!("Error searching wiki: {e}");
            vec![]
        }
    }
}