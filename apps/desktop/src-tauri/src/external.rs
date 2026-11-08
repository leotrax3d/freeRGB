//! User-triggered fixed external destinations; never executes a shell command.
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Destination {
    Email,
    Youtube,
    Publisher,
    Issue { title: String, body: String },
}
pub fn url(destination: Destination) -> Result<String, String> {
    Ok(match destination {
        Destination::Email => "mailto:freergbteam@gmail.com".into(),
        Destination::Youtube => "https://www.youtube.com/channel/UCh-B-9nUgKeCNJm3ESrpR8w".into(),
        Destination::Publisher => "https://github.com/leotrax3d/freeRGB".into(),
        Destination::Issue { title, body } => {
            if title.is_empty() || title.chars().count() > 160 || body.len() > 6000 {
                return Err("Report is too long to open safely; shorten it before sharing".into());
            }
            let mut url = url::Url::parse("https://github.com/leotrax3d/freeRGB/issues/new")
                .map_err(|e| e.to_string())?;
            url.query_pairs_mut()
                .append_pair("title", &title)
                .append_pair("body", &body);
            let url = String::from(url);
            if url.len() > 16000 {
                return Err("Report is too long for a browser link; copy it instead".into());
            }
            url
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_links_are_fixed_and_issue_content_is_encoded() {
        assert_eq!(
            url(Destination::Email).unwrap(),
            "mailto:freergbteam@gmail.com"
        );
        let value = url(Destination::Issue {
            title: "test&redirect=evil#x".into(),
            body: "USB device\n<script>".into(),
        })
        .unwrap();
        let parsed = url::Url::parse(&value).unwrap();
        assert_eq!(parsed.host_str(), Some("github.com"));
        assert_eq!(parsed.path(), "/leotrax3d/freeRGB/issues/new");
        assert!(parsed.fragment().is_none());
        assert_eq!(parsed.query_pairs().count(), 2);
        assert!(url(Destination::Issue {
            title: "test".into(),
            body: "x".repeat(6001)
        })
        .is_err());
    }
}
