use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub struct Channel {
    pub url: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarChannel {
    url: String,
    name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Video {
    pub url: String,
    pub title: String,
    pub channel: Rc<Channel>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarVideo {
    url: String,
    title: String,
    channel: u64,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Watched {
    pub video: Rc<Video>,
    pub when: chrono::DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarWatched {
    video: u64,
    when: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct Models {
    watches: Vec<Rc<Watched>>,
    channels: Vec<Rc<Channel>>,
    videos: Vec<Rc<Video>>,
}

impl Models {
    pub fn new() -> Models {
        Models {
            watches: Vec::new(),
            channels: Vec::new(),
            videos: Vec::new(),
        }
    }

    pub fn count_videos(&self, where_video: WhereVideo) -> u64 {
        self.videos.iter().fold(0, |acc, video| {
            if where_video.matches(video.clone()) {
                return acc + 1;
            } else {
                return acc;
            }
        })
    }

    pub fn insert_watched(
        &mut self,
        when: chrono::DateTime<Utc>,
        video: WhereVideo,
    ) -> Rc<Watched> {
        let video = self.find_video(video).unwrap().clone();
        let watched = Rc::new(Watched { video, when });
        self.watches.push(watched.clone());

        return watched;
    }

    pub fn insert_channel(&mut self, url: String, name: String) -> Rc<Channel> {
        let channel = Rc::new(Channel { url, name });
        self.channels.push(channel.clone());

        channel
    }

    pub fn insert_video(&mut self, url: String, title: String, channel: WhereChannel) -> Rc<Video> {
        let video = Rc::new(Video {
            url,
            title,
            channel: self.find_channel(channel).unwrap().clone(),
        });

        self.videos.push(video.clone());

        video
    }

    pub fn find_watched(&self, where_watched: WhereWatched) -> Option<Rc<Watched>> {
        self.watches
            .iter()
            .find(|watched| {
                return where_watched.matches((*watched).clone());
            })
            .map(|watched| watched.clone())
    }

    pub fn find_channel(&self, where_channel: WhereChannel) -> Option<Rc<Channel>> {
        self.channels
            .iter()
            .find(|channel| {
                return where_channel.matches((*channel).clone());
            })
            .map(|channel| channel.clone())
    }

    pub fn find_video(&self, where_video: WhereVideo<'_>) -> Option<Rc<Video>> {
        self.videos
            .iter()
            .find(|video| where_video.matches((*video).clone()))
            .map(|video| video.clone())
    }

    pub fn find_or_create_channel(&mut self, url: &String, name: &String) -> Rc<Channel> {
        if let Some(channel) = self.find_channel(WhereChannel::Structure {
            url: Some(&url),
            name: Some(&name),
        }) {
            return channel;
        }

        self.insert_channel(url.clone(), name.clone())
    }

    pub fn find_or_create_video(
        &mut self,
        url: String,
        title: String,
        channel: Rc<Channel>,
    ) -> Rc<Video> {
        if let Some(video) = self.find_video(WhereVideo::Structure {
            url: Some(url.clone()),
            title: Some(title.clone()),
            channel: Some(WhereChannel::Reference(channel.clone())),
        }) {
            return video.clone();
        }

        let channel = self.find_or_create_channel(&channel.url, &channel.name);
        self.insert_video(url, title, WhereChannel::Reference(channel))
    }

    pub fn index_of_watched(&self, watched: Rc<Watched>) -> u64 {
        self.watches
            .iter()
            .position(|w| w == &watched)
            .expect("watched not found") as u64
    }

    pub fn index_of_channel(&self, channel: Rc<Channel>) -> u64 {
        self.channels
            .iter()
            .position(|c| c == &channel)
            .expect("channel not found") as u64
    }

    pub fn index_of_video(&self, video: Rc<Video>) -> u64 {
        self.videos
            .iter()
            .position(|v| v == &video)
            .expect("video not found") as u64
    }

    pub fn to_string(&self) -> String {
        let scalar_models = ScalarModels {
            watches: self
                .watches
                .iter()
                .map(|watched| ScalarWatched {
                    video: self.index_of_video(watched.video.clone()),
                    when: watched.when,
                })
                .collect(),
            channels: self
                .channels
                .iter()
                .map(|channel| ScalarChannel {
                    url: channel.url.clone(),
                    name: channel.name.clone(),
                })
                .collect(),
            videos: self
                .videos
                .iter()
                .map(|video| ScalarVideo {
                    url: video.url.clone(),
                    title: video.title.clone(),
                    channel: self.index_of_channel(video.channel.clone()),
                })
                .collect(),
        };

        serde_json::to_string(&scalar_models).unwrap()
    }

    pub fn from_str(s: String) -> Self {
        let scalar_models: ScalarModels = serde_json::from_str(&s).unwrap();

        let mut models = Models {
            watches: Vec::new(),
            channels: Vec::new(),
            videos: Vec::new(),
        };

        for channel in scalar_models.channels {
            let channel = Channel {
                url: channel.url,
                name: channel.name,
            };
            models.channels.push(Rc::new(channel));
        }

        for video in scalar_models.videos {
            let channel = &models.channels[video.channel as usize];
            let video = Video {
                url: video.url,
                title: video.title,
                channel: channel.clone(),
            };
            models.videos.push(Rc::new(video));
        }

        for watched in scalar_models.watches {
            let video = &models.videos[watched.video as usize];
            let watched = Watched {
                video: video.clone(),
                when: watched.when,
            };
            models.watches.push(Rc::new(watched));
        }

        models
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarModels {
    watches: Vec<ScalarWatched>,
    channels: Vec<ScalarChannel>,
    videos: Vec<ScalarVideo>,
}

pub enum WhereChannel<'a> {
    Structure {
        url: Option<&'a String>,
        name: Option<&'a String>,
    },
    Reference(Rc<Channel>),
}

impl WhereChannel<'_> {
    fn matches(&self, channel: Rc<Channel>) -> bool {
        match self {
            WhereChannel::Structure { url, name } => {
                if let Some(url) = url {
                    if &channel.url != *url {
                        return false;
                    }
                }

                if let Some(name) = name {
                    if &channel.name != *name {
                        return false;
                    }
                }

                return true;
            }
            WhereChannel::Reference(reference) => reference == &channel,
        }
    }
}

pub enum WhereVideo<'a> {
    Structure {
        url: Option<String>,
        title: Option<String>,
        channel: Option<WhereChannel<'a>>,
    },
    Reference(Rc<Video>),
    Any,
}

impl WhereVideo<'_> {
    fn matches(&self, video: Rc<Video>) -> bool {
        match self {
            WhereVideo::Structure {
                url,
                title,
                channel,
            } => {
                if let Some(url) = url {
                    if video.url != *url {
                        return false;
                    }
                }
                if let Some(title) = title {
                    if video.title != *title {
                        return false;
                    }
                }
                if let Some(channel) = channel {
                    if !channel.matches(video.channel.clone()) {
                        return false;
                    }
                }

                return true;
            }
            WhereVideo::Reference(reference) => {
                return reference == &video;
            }
            WhereVideo::Any => {
                return true;
            }
        }
    }
}

enum WhereWatched<'a> {
    Structure {
        video: Option<WhereVideo<'a>>,
        when: Option<chrono::DateTime<Utc>>,
    },
    Reference(Rc<Watched>),
}

impl WhereWatched<'_> {
    fn matches(&self, watched: Rc<Watched>) -> bool {
        match self {
            WhereWatched::Structure { video, when } => {
                if let Some(video) = video {
                    if !video.matches(watched.video.clone()) {
                        return false;
                    }
                }
                if let Some(when) = when {
                    if watched.when != *when {
                        return false;
                    }
                }

                return true;
            }
            WhereWatched::Reference(reference) => {
                return reference == &watched;
            }
        }
    }
}
