use chrono::{FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

trait Model {
    type Id: Eq + Hash + Clone;

    fn id(&self) -> &Self::Id;
}

#[derive(Debug, PartialEq, Clone)]
pub struct Channel {
    pub url: String,
    pub name: String,
}

impl Model for Channel {
    type Id = String;

    fn id(&self) -> &String {
        &self.url
    }
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

impl Model for Video {
    type Id = String;

    fn id(&self) -> &String {
        &self.url
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarVideo {
    url: String,
    title: String,
    channel: <Channel as Model>::Id,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Watched {
    pub video: Rc<Video>,
    pub when: chrono::DateTime<FixedOffset>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarWatched {
    video: <Video as Model>::Id,
    when: chrono::DateTime<FixedOffset>,
}

#[derive(Debug)]
pub struct Models {
    watches: Vec<Watched>,
    channels: HashMap<<Channel as Model>::Id, Rc<Channel>>,
    videos: HashMap<<Video as Model>::Id, Rc<Video>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ScalarModels {
    watches: Vec<ScalarWatched>,
    channels: Vec<ScalarChannel>,
    videos: Vec<ScalarVideo>,
}

impl Models {
    pub fn new() -> Models {
        Models {
            watches: Vec::new(),
            channels: HashMap::new(),
            videos: HashMap::new(),
        }
    }

    pub fn count_videos(&self, where_video: WhereVideo) -> u64 {
        match where_video {
            WhereVideo::Structure(matcher) => {
                return self
                    .videos
                    .iter()
                    .filter(|(_, video)| matcher.eq(*video))
                    .count() as u64;
            }
            WhereVideo::Reference(video) => {
                if self.videos.contains_key(video.id()) {
                    return 1;
                } else {
                    return 0;
                }
            }
            WhereVideo::Any => {
                return self.videos.len() as u64;
            }
        }
    }

    pub fn count_watches(&self, where_watch: WhereWatched) -> u64 {
        match where_watch {
            WhereWatched::Structure(matcher) => {
                return self
                    .watches
                    .iter()
                    .filter(|watched| matcher.eq(watched))
                    .count() as u64;
            }
            WhereWatched::Reference(_) => {
                todo!();
            }
            WhereWatched::Any => {
                return self.watches.len() as u64;
            }
        }
    }

    pub fn count_watched_by_video(&self) -> HashMap<String, (usize, Rc<Video>)> {
        let mut counts = HashMap::new();

        for watched in self.watches.iter() {
            let count = counts
                .entry(watched.video.id().clone())
                .or_insert((0, watched.video.clone()));

            (*count).0 += 1;
        }

        counts
    }

    pub fn insert_watched(
        &mut self,
        when: chrono::DateTime<FixedOffset>,
        video: WhereVideo,
    ) -> Watched {
        let video = self.find_video(video).unwrap().clone();
        let watched = Watched { video, when };
        self.watches.push(watched.clone());

        return watched;
    }

    pub fn insert_channel(&mut self, url: String, name: String) -> Rc<Channel> {
        let channel = Rc::new(Channel { url, name });
        self.channels.insert(channel.id().clone(), channel.clone());

        channel
    }

    pub fn insert_video(&mut self, url: String, title: String, channel: WhereChannel) -> Rc<Video> {
        let video = Rc::new(Video {
            url,
            title,
            channel: self.find_channel(channel).unwrap().clone(),
        });

        self.videos.insert(video.id().clone(), video.clone());

        video
    }

    pub fn find_channel(&self, where_channel: WhereChannel) -> Option<Rc<Channel>> {
        match where_channel {
            WhereChannel::Structure(matcher) => {
                if let Some(url) = matcher.url {
                    return self.channels.get(url).map(|channel| channel.clone());
                }

                return self
                    .channels
                    .iter()
                    .find(|(_, channel)| matcher.matches(*channel))
                    .map(|(_, channel)| channel.clone());
            }
            WhereChannel::Reference(channel) => Some(channel),
            WhereChannel::Any => self.channels.values().next().map(|channel| channel.clone()),
        }
    }

    pub fn find_video(&self, where_video: WhereVideo<'_>) -> Option<Rc<Video>> {
        match where_video {
            WhereVideo::Structure(matcher) => {
                if let Some(url) = matcher.url {
                    return self.videos.get(url).map(|video| video.clone());
                }

                return self
                    .videos
                    .iter()
                    .find(|(_, video)| matcher.eq(*video))
                    .map(|(_, video)| video.clone());
            }
            WhereVideo::Reference(video) => Some(video),
            WhereVideo::Any => self.videos.values().next().map(|video| video.clone()),
        }
    }

    pub fn find_or_create_channel(&mut self, url: &String, name: &String) -> Rc<Channel> {
        if let Some(channel) = self.find_channel(WhereChannel::Structure(ChannelMatcher {
            url: Some(&url),
            name: Some(&name),
        })) {
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
        if let Some(video) = self.videos.get(&url) {
            return video.clone();
        }

        let channel = self.find_or_create_channel(&channel.url, &channel.name);
        self.insert_video(url, title, WhereChannel::Reference(channel))
    }

    pub fn to_string(&self) -> String {
        let scalar_models = ScalarModels {
            watches: self
                .watches
                .iter()
                .map(|watched| ScalarWatched {
                    video: watched.video.id().clone(),
                    when: watched.when,
                })
                .collect(),
            channels: self
                .channels
                .iter()
                .map(|(_, channel)| ScalarChannel {
                    url: channel.url.clone(),
                    name: channel.name.clone(),
                })
                .collect(),
            videos: self
                .videos
                .iter()
                .map(|(_, video)| ScalarVideo {
                    url: video.url.clone(),
                    title: video.title.clone(),
                    channel: video.channel.id().clone(),
                })
                .collect(),
        };

        serde_json::to_string(&scalar_models).unwrap()
    }

    pub fn from_str(s: String) -> serde_json::Result<Models> {
        let scalar_models: ScalarModels = serde_json::from_str(&s)?;

        let mut models = Models {
            watches: Vec::new(),
            channels: HashMap::new(),
            videos: HashMap::new(),
        };

        for channel in scalar_models.channels {
            let channel = Channel {
                url: channel.url,
                name: channel.name,
            };
            models
                .channels
                .insert(channel.id().clone(), Rc::new(channel));
        }

        for video in scalar_models.videos {
            let channel = &models.channels.get(&video.channel).unwrap();
            let video = Video {
                url: video.url,
                title: video.title,
                channel: (*channel).clone(),
            };
            models.videos.insert(video.id().clone(), Rc::new(video));
        }

        for watched in scalar_models.watches {
            let video = &models.videos.get(&watched.video).unwrap();
            let watched = Watched {
                video: (*video).clone(),
                when: watched.when,
            };
            models.watches.push(watched);
        }

        Ok(models)
    }
}

pub struct ChannelMatcher<'a> {
    url: Option<&'a String>,
    name: Option<&'a String>,
}

impl ChannelMatcher<'_> {
    fn matches(&self, channel: &Rc<Channel>) -> bool {
        if let Some(url) = self.url {
            // If matching on primary key (url), other fields can be skipped
            return channel.url == *url;
        }

        if let Some(name) = self.name {
            if channel.name != *name {
                return false;
            }
        }

        return true;
    }
}

pub enum WhereChannel<'a> {
    Structure(ChannelMatcher<'a>),
    Reference(Rc<Channel>),
    Any,
}

impl WhereChannel<'_> {
    fn matches(&self, channel: Rc<Channel>) -> bool {
        match self {
            WhereChannel::Structure(matcher) => matcher.matches(&channel),
            WhereChannel::Reference(reference) => reference == &channel,
            WhereChannel::Any => true,
        }
    }
}

pub struct VideoMatcher<'a> {
    url: Option<&'a String>,
    title: Option<&'a String>,
    channel: Option<WhereChannel<'a>>,
}

impl PartialEq<Rc<Video>> for VideoMatcher<'_> {
    fn eq(&self, video: &Rc<Video>) -> bool {
        if let Some(url) = self.url {
            if &video.url != url {
                return false;
            }
        }

        if let Some(title) = self.title {
            if &video.title != title {
                return false;
            }
        }

        if let Some(channel) = &self.channel {
            if !channel.matches(video.channel.clone()) {
                return false;
            }
        }

        return true;
    }
}

pub enum WhereVideo<'a> {
    #[allow(dead_code)]
    Structure(VideoMatcher<'a>),
    Reference(Rc<Video>),
    Any,
}

impl WhereVideo<'_> {
    fn matches(&self, video: Rc<Video>) -> bool {
        match self {
            WhereVideo::Structure(VideoMatcher {
                url,
                title,
                channel,
            }) => {
                if let Some(url) = url {
                    // If matching on primary key (url), other fields can be skipped
                    return &video.url == *url;
                }
                if let Some(title) = title {
                    if &video.title != *title {
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

pub struct WatchedMatcher<'a> {
    video: Option<WhereVideo<'a>>,
    when: Option<chrono::DateTime<Utc>>,
}

impl PartialEq<Watched> for WatchedMatcher<'_> {
    fn eq(&self, watched: &Watched) -> bool {
        if let Some(video) = &self.video {
            if !video.matches(watched.video.clone()) {
                return false;
            }
        }

        if let Some(when) = &self.when {
            if watched.when != *when {
                return false;
            }
        }

        return true;
    }
}

#[allow(dead_code)]
pub enum WhereWatched<'a> {
    Structure(WatchedMatcher<'a>),
    Reference(Rc<Watched>),
    Any,
}

impl WhereWatched<'_> {
    #[allow(dead_code)]
    fn matches(&self, watched: Watched) -> bool {
        match self {
            WhereWatched::Structure(WatchedMatcher { video, when }) => {
                if let Some(video) = video {
                    if !video.matches(watched.video) {
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
                return **reference == watched;
            }
            WhereWatched::Any => {
                return true;
            }
        }
    }
}
