//
// now_playing_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
// 单独播放页（参照 AerMusic-Refactor 设计）
//
use async_channel::Sender;
use glib::{
    ParamSpec, Value,
};
use gtk::{
    CompositeTemplate, glib, prelude::*, subclass::prelude::*,
    Button, Label, Scale, TextView,
};

use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::cell::{Cell, RefCell};

use crate::application::{Action, PlayModeAction};
use crate::model::ImageDownloadImpl;
use crate::path::CACHE;

glib::wrapper! {
    pub struct NowPlayingPage(ObjectSubclass<imp::NowPlayingPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl NowPlayingPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sender(&self, sender_: Sender<Action>) {
        let sender = sender_.clone();
        self.imp().sender.set(sender).unwrap();
    }

    pub fn set_current_song(&self, si: &SongInfo) {
        let imp = self.imp();
        imp.song_title.set_label(&si.name);
        imp.song_artist.set_label(&si.singer);
        imp.song_album.set_label(&si.album);
        imp.lyrics_text_view.buffer().set_text("");
        imp.current_lyrics.borrow_mut().clear();

        // 加载专辑封面（始终通过 set_from_net，它会优先使用缓存，没有则下载）
        let cover_image = &imp.cover_image;
        let sender = imp.sender.get().unwrap().clone();
        let mut path_cover = CACHE.clone();
        path_cover.push(format!("{}-songlist.jpg", si.album_id));
        cover_image.set_from_net(
            si.pic_url.to_owned(),
            path_cover.to_owned(),
            (320, 320),
            &sender,
        );

        // 同时设置模糊背景
        let cover_blur_bg = &imp.cover_blur_bg;
        cover_blur_bg.set_from_net(
            si.pic_url.to_owned(),
            path_cover.to_owned(),
            (320, 320),
            &sender,
        );
    }

    pub fn update_lyrics(&self, lyrics: Vec<(u64, String)>) {
        let imp = self.imp();
        let buffer = imp.lyrics_text_view.buffer();
        buffer.set_text(
            &lyrics
                .iter()
                .map(|(_, x)| x.to_owned())
                .collect::<Vec<_>>()
                .join(""),
        );
        let mut current_lyrics = imp.current_lyrics.borrow_mut();
        *current_lyrics = lyrics;
    }

    pub fn update_lyrics_highlight(&self, time: u64) {
        let imp = self.imp();
        let lyrics_text_view = imp.lyrics_text_view.get();
        let lyrics = imp.current_lyrics.borrow().clone();
        let playing_indexes = get_playing_indexes(lyrics, time);
        if playing_indexes.is_none() {
            return;
        }
        let (start, end) = playing_indexes.unwrap();
        self.set_lyrics_highlight(start as i32, end as i32);
        if let Some(mark) = self.find_mark(start as i32) {
            lyrics_text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
        }
    }

    fn set_lyrics_highlight(&self, line_start: i32, line_end: i32) {
        let imp = self.imp();
        let highlight_text_tag = imp.highlight_text_tag.get().unwrap();
        let uttered_text_tag = imp.uttered_text_tag.get().unwrap();
        let buffer = imp.lyrics_text_view.buffer();
        buffer.remove_tag(highlight_text_tag, &buffer.start_iter(), &buffer.end_iter());
        buffer.remove_tag(uttered_text_tag, &buffer.start_iter(), &buffer.end_iter());
        for i in 0..line_start {
            if let Some(start) = buffer.iter_at_line(i) {
                let mut end = start.clone();
                if !start.ends_line() { end.forward_to_line_end(); }
                buffer.apply_tag(uttered_text_tag, &start, &end);
            }
        }
        for i in line_start..=line_end {
            if let Some(start) = buffer.iter_at_line(i) {
                let mut end = start.clone();
                if !start.ends_line() { end.forward_to_line_end(); }
                buffer.apply_tag(highlight_text_tag, &start, &end);
            }
        }
    }

    fn find_mark(&self, line: i32) -> Option<gtk::TextMark> {
        let imp = self.imp();
        let buffer = imp.lyrics_text_view.buffer();
        let iter = buffer.iter_at_line(line)?;
        Some(buffer.create_mark(None, &iter, true))
    }

    pub fn update_progress(&self, position: u64, duration: u64) {
        let imp = self.imp();
        let adjustment = imp.seek_scale.adjustment();
        if duration > 0 {
            adjustment.set_upper(duration as f64);
            adjustment.set_value(position as f64);
        }
        imp.current_time_label.set_label(&format_time(position));
        imp.duration_label.set_label(&format_time(duration));
    }

    pub fn update_play_pause_state(&self, playing: bool) {
        let imp = self.imp();
        if playing {
            imp.play_pause_button.set_icon_name("media-playback-pause-symbolic");
        } else {
            imp.play_pause_button.set_icon_name("media-playback-start-symbolic");
        }
    }

    pub fn update_like_state(&self, liked: bool) {
        let imp = self.imp();
        if liked {
            imp.like_button.add_css_class("playing-like");
        } else {
            imp.like_button.remove_css_class("playing-like");
        }
    }

    pub fn bind_current_song(&self, window: &crate::window::NeteaseCloudMusicGtk4Window) {
        let imp = self.imp();
        if !imp.subscribed.get() {
            imp.subscribed.set(true);
            let page = self.downgrade();
            window.connect_local(
                "current-song-changed",
                false,
                move |_args| {
                    if let Some(page) = page.upgrade() {
                        let _ = page;
                    }
                    None
                },
            );
        }
        let _ = imp.window.set(window.clone());
    }
}

// 获取当前歌词行索引
fn get_playing_indexes(lyrics: Vec<(u64, String)>, time: u64) -> Option<(usize, usize)> {
    let mut idx: Option<(usize, usize)> = None;
    for (i, (t, _)) in lyrics.iter().enumerate() {
        if *t <= time {
            idx = Some((i, i));
        } else {
            break;
        }
    }
    idx
}

// 格式化时间（毫秒 -> M:SS）
fn format_time(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

mod imp {

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/now-playing-page.ui")]
    pub struct NowPlayingPage {
        #[template_child]
        pub back_button: TemplateChild<Button>,
        #[template_child]
        pub page_title: TemplateChild<Label>,
        #[template_child]
        pub playlist_toggle_button: TemplateChild<Button>,
        #[template_child]
        pub cover_image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub cover_blur_bg: TemplateChild<gtk::Picture>,
        #[template_child]
        pub song_title: TemplateChild<Label>,
        #[template_child]
        pub song_artist: TemplateChild<Label>,
        #[template_child]
        pub song_album: TemplateChild<Label>,
        #[template_child]
        pub lyrics_text_view: TemplateChild<TextView>,
        #[template_child]
        pub seek_scale: TemplateChild<Scale>,
        #[template_child]
        pub current_time_label: TemplateChild<Label>,
        #[template_child]
        pub duration_label: TemplateChild<Label>,
        #[template_child]
        pub play_pause_button: TemplateChild<Button>,
        #[template_child]
        pub prev_button: TemplateChild<Button>,
        #[template_child]
        pub next_button: TemplateChild<Button>,
        #[template_child]
        pub shuffle_button: TemplateChild<Button>,
        #[template_child]
        pub repeat_button: TemplateChild<Button>,
        #[template_child]
        pub like_button: TemplateChild<Button>,
        #[template_child]
        pub play_mode_button: TemplateChild<Button>,
        #[template_child]
        pub volume_scale: TemplateChild<Scale>,
        #[template_child]
        pub volume_button: TemplateChild<Button>,

        pub sender: OnceCell<Sender<Action>>,
        pub window: OnceCell<crate::window::NeteaseCloudMusicGtk4Window>,
        pub subscribed: Cell<bool>,
        pub highlight_text_tag: OnceCell<gtk::TextTag>,
        pub uttered_text_tag: OnceCell<gtk::TextTag>,
        pub current_lyrics: RefCell<Vec<(u64, String)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NowPlayingPage {
        const NAME: &'static str = "NowPlayingPage";
        type Type = super::NowPlayingPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NowPlayingPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            let buffer = self.lyrics_text_view.buffer();
            let highlight_tag = gtk::TextTag::new(None);
            buffer.tag_table().add(&highlight_tag);
            self.highlight_text_tag.set(highlight_tag).unwrap();
            let uttered_tag = gtk::TextTag::new(None);
            buffer.tag_table().add(&uttered_tag);
            self.uttered_text_tag.set(uttered_tag).unwrap();
            obj.apply_lyrics_colors();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| vec![]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, _value: &Value, _pspec: &ParamSpec) {
            unimplemented!()
        }

        fn property(&self, _id: usize, _pspec: &ParamSpec) -> Value {
            unimplemented!()
        }
    }

    impl WidgetImpl for NowPlayingPage {}
    impl BoxImpl for NowPlayingPage {}

    #[gtk::template_callbacks]
    impl NowPlayingPage {
        #[template_callback]
        fn back_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PageBack);
            }
        }

        #[template_callback]
        fn playlist_toggle_clicked(&self) {
            // 由窗口层处理打开歌单/歌词页面
        }

        #[template_callback]
        fn prev_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayPreviousSong);
            }
        }

        #[template_callback]
        fn play_pause_button_clicked(&self, _button: Button) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::TogglePlayPause);
            }
        }

        #[template_callback]
        fn next_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayNextSong);
            }
        }

        #[template_callback]
        fn shuffle_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayMode(PlayModeAction::ToggleShuffle));
            }
        }

        #[template_callback]
        fn repeat_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayMode(PlayModeAction::ToggleRepeat));
            }
        }

        #[template_callback]
        fn like_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayMode(PlayModeAction::ToggleLike));
            }
        }

        #[template_callback]
        fn play_mode_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayMode(PlayModeAction::CycleMode));
            }
        }

        #[template_callback]
        fn volume_value_changed(&self, scale: Scale) {
            let vol = scale.value();
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PersistVolume(vol));
            }
            let icon = if vol <= 0.0 {
                "audio-volume-muted-symbolic"
            } else if vol < 0.33 {
                "audio-volume-low-symbolic"
            } else if vol < 0.66 {
                "audio-volume-medium-symbolic"
            } else {
                "audio-volume-high-symbolic"
            };
            self.volume_button.set_icon_name(icon);
        }

        #[template_callback]
        fn volume_button_clicked(&self) {
            if let Some(sender) = self.sender.get() {
                let _ = sender.send_blocking(Action::PlayMode(PlayModeAction::ToggleMute));
            }
        }

        #[template_callback]
        fn seek_scale_change_value(
            &self,
            _scale: Scale,
            _scroll: gtk::ScrollType,
            value: f64,
        ) -> glib::Propagation {
            if let Some(sender) = self.sender.get() {
                let sec = (value / 1_000_000.0) as u64; // 微秒转秒
                let _ = sender.send_blocking(Action::ScaleSeekUpdate(sec));
            }
            glib::Propagation::Proceed
        }
    }
}

impl NowPlayingPage {
    #[allow(deprecated)]
    fn apply_lyrics_colors(&self) {
        let imp = self.imp();
        let ctx = imp.lyrics_text_view.style_context();
        if let Some(mut fg) = ctx.lookup_color("theme_fg_color") {
            fg.set_alpha(0.45);
            imp.uttered_text_tag.get().unwrap()
                .set_property("foreground-rgba", &fg);
        } else {
            imp.uttered_text_tag.get().unwrap()
                .set_property("foreground", "#999999");
        }
        imp.highlight_text_tag.get().unwrap()
            .set_property("foreground", "#d9a441");
        imp.highlight_text_tag.get().unwrap()
            .set_property("weight", 700);
    }
}