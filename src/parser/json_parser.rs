use std::{error::Error, io::Read};

use serde::Deserialize;

use crate::model::{Models, WhereVideo};

const DEFAULT_CHANNEL: &str = "(hidden)";

#[derive(Deserialize, Debug, PartialEq)]
struct DataRow {
    header: String,
    title: String,
    #[serde(rename = "titleUrl", default)]
    title_url: String,
    #[serde(default)]
    subtitles: Vec<Subtitles>,
    time: String,
    products: Vec<String>,
    #[serde(rename = "activityControls")]
    activity_controls: Vec<String>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Subtitles {
    name: String,
    #[serde(default)]
    url: String,
}

pub fn parse<R: Read>(reader: R) -> Result<Models, Box<dyn Error>> {
    let rows = parse_data_rows(reader)?;
    let mut models = Models::new();

    for row in rows {
        if row.title == "Visited YouTube Music" {
            // Skip rows that are not videos
            continue;
        }

        // TODO maybe model functions should take a &str to avoid allocating here
        let default_channel = DEFAULT_CHANNEL.to_owned();

        let channel_subtitle = row.subtitles.get(0);
        let channel_name = match channel_subtitle {
            Some(s) => &s.name,
            None => &default_channel,
        };
        let channel_url = match channel_subtitle {
            Some(s) => &s.url,
            None => &default_channel,
        };
        let date = chrono::DateTime::parse_from_rfc3339(row.time.as_str())?;

        let title = if row.title.starts_with("Watched ") {
            &row.title[8..]
        } else {
            &row.title
        };

        let channel = models.find_or_create_channel(&channel_url, &channel_name);
        let video = models.find_or_create_video(row.title_url, title.into(), channel);

        models.insert_watched(date, WhereVideo::Reference(video));
    }

    Ok(models)
}

fn parse_data_rows<R: Read>(reader: R) -> Result<Vec<DataRow>, serde_json::Error> {
    serde_json::from_reader(reader).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let input = "[]";
        let actual = parse_data_rows(input.as_bytes()).unwrap();
        let expected: Vec<DataRow> = vec![];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_one() {
        let input = r#"
            [{
                "header": "YouTube",
                "title": "Watched An Addictive Alternative To DAWs",
                "titleUrl": "https://www.youtube.com/watch?v\u003drtTWtzWav8I",
                "subtitles": [{
                    "name": "Benn Jordan",
                    "url": "https://www.youtube.com/channel/UCshObcm-nLhbu8MY50EZ5Ng"
                }],
                "time": "2023-06-04T04:07:59.107Z",
                "products": ["YouTube"],
                "activityControls": ["YouTube watch history"]
            }]
        "#;

        let actual = parse_data_rows(input.as_bytes()).unwrap();
        let expected: Vec<DataRow> = vec![DataRow {
            header: "YouTube".into(),
            title: "Watched An Addictive Alternative To DAWs".into(),
            title_url: "https://www.youtube.com/watch?v=rtTWtzWav8I".into(),
            subtitles: vec![Subtitles {
                name: "Benn Jordan".into(),
                url: "https://www.youtube.com/channel/UCshObcm-nLhbu8MY50EZ5Ng".into(),
            }],
            time: "2023-06-04T04:07:59.107Z".into(),
            products: vec!["YouTube".into()],
            activity_controls: vec!["YouTube watch history".into()],
        }];

        assert_eq!(expected, actual);
    }
}
