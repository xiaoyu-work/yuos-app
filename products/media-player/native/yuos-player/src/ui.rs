// SPDX-License-Identifier: GPL-3.0-only

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit};
use cosmic::iced::widget::{pin, stack};
use cosmic::iced::{Alignment, ContentFit, Length, Point, Size};
use cosmic::widget::{self, Slider};
use cosmic::{Element, font};

use crate::{App, DropdownKind, JUMP_BACKWARD_ICON, JUMP_FORWARD_ICON, Message, RepeatState};
use crate::{ProjectNode, Video, fl, style};

fn icon(name: &'static str) -> widget::icon::Handle {
    widget::icon::from_name(name).size(16).handle()
}

fn icon_button(name: &'static str, message: Message) -> widget::button::Button<'static, Message> {
    glyph_button(icon(name), message)
}

fn glyph_button(
    handle: widget::icon::Handle,
    message: Message,
) -> widget::button::Button<'static, Message> {
    widget::button::custom(widget::icon(handle).size(16))
        .class(style::control(false))
        .padding(6)
        .on_press(message)
}

fn open_button(
    label: String,
    icon_name: &'static str,
    primary: bool,
    message: Message,
) -> Element<'static, Message> {
    widget::button::custom(
        widget::row::with_children(vec![
            widget::icon(icon(icon_name)).into(),
            widget::text(label).size(13).into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .class(if primary {
        style::control(true)
    } else {
        style::outline_button()
    })
    .padding([6, 11])
    .on_press(message)
    .into()
}

fn time(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    let (hours, minutes, seconds) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn divider() -> Element<'static, Message> {
    widget::container(widget::space::vertical())
        .width(Length::Fill)
        .height(1)
        .class(style::divider())
        .into()
}

impl App {
    pub(super) fn media_title(&self) -> String {
        self.video_opt
            .as_ref()
            .and(self.flags.url_opt.as_ref())
            .map(|url| {
                url.to_file_path()
                    .ok()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .or_else(|| url.host_str().map(str::to_owned))
                    .unwrap_or_else(|| fl!("xdg-name"))
            })
            .unwrap_or_else(|| fl!("xdg-name"))
    }

    pub(super) fn folders(&self) -> Option<Element<'_, cosmic::Action<Message>>> {
        if self.fullscreen || !self.core.nav_bar_active() {
            return None;
        }

        let model = &self.nav_model;
        let mut rows = widget::column::with_capacity(model.iter().count()).spacing(2);
        for id in model
            .iter()
            .filter(|id| model.data::<ProjectNode>(*id).is_some())
            .chain(
                model
                    .iter()
                    .filter(|id| model.data::<ProjectNode>(*id).is_none()),
            )
        {
            let Some(name) = model.text(id) else { continue };
            let active = model.is_active(id);
            let (disclosure, glyph) = match model.data::<ProjectNode>(id) {
                Some(ProjectNode::Folder { open, .. }) => (Some(*open), "folder-symbolic"),
                Some(ProjectNode::File { path, .. }) => (
                    None,
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| {
                            [
                                "aac", "aiff", "alac", "flac", "m4a", "mp3", "oga", "ogg", "opus",
                                "wav",
                            ]
                            .iter()
                            .any(|audio| ext.eq_ignore_ascii_case(audio))
                        })
                    {
                        "audio-x-generic-symbolic"
                    } else {
                        "video-x-generic-symbolic"
                    },
                ),
                None => (None, "folder-open-symbolic"),
            };
            let mut row = widget::row::with_capacity(3)
                .align_y(Alignment::Center)
                .spacing(6)
                .push(widget::space::horizontal().width(12 * model.indent(id).unwrap_or(0).min(8)));
            if let Some(open) = disclosure {
                row = row.push(
                    widget::icon::from_name(if open {
                        "go-down-symbolic"
                    } else {
                        "go-next-symbolic"
                    })
                    .size(12),
                );
            }
            row = row.push(widget::icon(icon(glyph))).push(
                widget::text(name)
                    .size(13)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
            );
            rows = rows.push(
                widget::button::custom(row)
                    .class(style::control(active))
                    .width(Length::Fill)
                    .height(26)
                    .padding([3, 6])
                    .on_press(cosmic::Action::Cosmic(cosmic::app::Action::NavBar(id))),
            );
        }

        Some(
            widget::container(widget::column::with_children(vec![
                widget::container(
                    widget::text(fl!("media-folders"))
                        .size(12)
                        .font(font::semibold()),
                )
                .class(style::muted())
                .padding([4, 6])
                .into(),
                widget::scrollable(rows).height(Length::Fill).into(),
            ]))
            .padding(8)
            .width(if self.core.is_condensed() {
                Length::Fill
            } else {
                Length::Fixed(228.0)
            })
            .height(Length::Fill)
            .class(style::sidebar())
            .into(),
        )
    }

    pub(super) fn player_view<'a>(
        &'a self,
        video: &'a Video,
        player: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let stage: Element<_> = if video.has_video() {
            widget::container(player)
                .center(Length::Fill)
                .class(style::video_stage())
                .into()
        } else {
            // Keep the renderer widget attached so audio-only streams still dispatch
            // timeline and end-of-stream events; opaque now-playing content covers it.
            stack([
                widget::space::horizontal()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
                player,
                widget::container(widget::responsive(|size| self.audio_details(size)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .class(style::surface())
                    .into(),
            ])
            .into()
        };
        let stage = widget::mouse_area(stage)
            .on_press(Message::PlayPause)
            .on_double_press(Message::Fullscreen);
        if self.fullscreen {
            let mut layers: Vec<Element<_>> = vec![stage.into()];
            if self.controls {
                layers.push(
                    widget::column::with_children(vec![
                        widget::space::vertical().into(),
                        self.transport(video),
                    ])
                    .into(),
                );
            }
            return stack(layers).into();
        }
        widget::column::with_children(vec![
            divider(),
            stage.into(),
            self.transport(video),
            self.status_bar(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn empty_view(&self) -> Element<'_, Message> {
        let body = widget::container(
            widget::column::with_children(vec![
                widget::icon::from_name("video-x-generic-symbolic")
                    .size(48)
                    .into(),
                widget::text(fl!("no-video-or-audio-file-open"))
                    .size(13)
                    .into(),
                widget::row::with_children(vec![
                    open_button(
                        fl!("open-file"),
                        "document-open-symbolic",
                        true,
                        Message::FileOpen,
                    ),
                    open_button(
                        fl!("open-folder"),
                        "folder-open-symbolic",
                        false,
                        Message::FolderOpen,
                    ),
                ])
                .spacing(8)
                .into(),
            ])
            .spacing(12)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .class(style::muted());
        widget::column::with_children(vec![divider(), body.into()])
            .height(Length::Fill)
            .into()
    }

    fn audio_details(&self, size: Size) -> Element<'_, Message> {
        let meta = &self.mpris_meta;
        let title = if meta.title.is_empty() {
            self.media_title()
        } else {
            meta.title.clone()
        };
        let artist = if meta.artists.is_empty() {
            fl!("unknown-author")
        } else {
            meta.artists.join(", ")
        };
        let mut details = widget::column::with_children(vec![
            widget::text(title).size(15).font(font::semibold()).into(),
            widget::container(widget::text(artist).size(13))
                .class(style::muted())
                .into(),
        ])
        .spacing(6);
        if !meta.album.is_empty() {
            details =
                details.push(widget::text(fl!("album", album = meta.album.as_str())).size(13));
        }
        if let Some(year) = meta.album_year_opt {
            details = details.push(widget::text(fl!("release-year", year = year)).size(13));
        }
        let side = 248.0_f32
            .min(size.width * 0.42)
            .min((size.height - 16.0).max(0.0));
        let mut layout = widget::row::with_capacity(2)
            .align_y(Alignment::Center)
            .spacing(24);
        if side >= 44.0 {
            let art: Element<_> = if let Some(file) = &self.album_art_opt {
                widget::image(widget::image::Handle::from_path(file.path()))
                    .width(side)
                    .height(side)
                    .content_fit(ContentFit::Contain)
                    .into()
            } else {
                widget::icon::from_name("audio-x-generic-symbolic")
                    .size((side * 0.38) as u16)
                    .into()
            };
            layout = layout.push(widget::container(art).center(side).class(style::artwork()));
        }
        widget::container(layout.push(details))
            .padding([4, 16])
            .center(Length::Fill)
            .into()
    }

    fn timeline(&self) -> Element<'_, Message> {
        let duration = self.duration;
        let position = self.position.clamp(0.0, duration);
        let seek = Slider::new(0.0..=duration, position, Message::Seek)
            .height(18)
            .step(0.1)
            .on_release(Message::SeekRelease)
            .class(style::scrubber());
        let rail: Element<_> = if let Some((a, b)) = self.ab_repeat.filter(|_| duration > 0.0) {
            let fractions =
                [a, b].map(|point| point.map(|value| (value / duration).clamp(0.0, 1.0) as f32));
            let marks = widget::responsive(move |size| {
                let x = |fraction: f32| 6.0 + (size.width - 12.0).max(0.0) * fraction;
                let mut layer: Vec<Element<_>> = vec![
                    widget::space::horizontal()
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into(),
                ];
                if let [Some(a), Some(b)] = fractions {
                    let start = x(a.min(b));
                    layer.push(
                        pin(widget::container(widget::space::horizontal())
                            .width(x(a.max(b)) - start)
                            .height(8)
                            .class(style::loop_range()))
                        .x(start)
                        .y(14)
                        .into(),
                    );
                }
                for (label, fraction) in ["A", "B"].into_iter().zip(fractions) {
                    if let Some(fraction) = fraction {
                        layer.push(
                            pin(widget::text(label).size(10))
                                .x(x(fraction) - 3.0)
                                .into(),
                        );
                        layer.push(
                            pin(widget::container(widget::space::vertical())
                                .width(1)
                                .height(9)
                                .class(style::loop_marker()))
                            .x(x(fraction))
                            .y(14)
                            .into(),
                        );
                    }
                }
                widget::container(stack(layer))
                    .class(style::accent_text())
                    .into()
            });
            stack([
                widget::column::with_children(vec![
                    widget::space::vertical().height(9).into(),
                    seek.into(),
                ])
                .into(),
                marks.into(),
            ])
            .into()
        } else if duration > 0.0 {
            seek.into()
        } else {
            widget::container(
                widget::container(widget::space::horizontal())
                    .width(Length::Fill)
                    .height(4)
                    .class(style::divider()),
            )
            .height(18)
            .align_y(Alignment::Center)
            .into()
        };
        widget::row::with_children(vec![
            widget::text(time(self.position)).size(11).into(),
            rail,
            widget::text(format!("-{}", time(duration - self.position)))
                .size(11)
                .into(),
        ])
        .spacing(10)
        .align_y(Alignment::Center)
        .height(27)
        .into()
    }

    fn transport(&self, video: &Video) -> Element<'_, Message> {
        let repeat = self.flags.config_state.player_state.repeat == RepeatState::Track;
        let repeat_message = Message::RepeatToggled(if repeat {
            RepeatState::Disabled
        } else {
            RepeatState::Track
        });
        let repeat_icon: &[u8] = if repeat {
            include_bytes!("../res/icons/hicolor/16x16/apps/player-repeat-track-symbolic.svg")
        } else {
            include_bytes!("../res/icons/hicolor/16x16/apps/player-repeat-symbolic.svg")
        };
        let repeat_button = widget::tooltip(
            glyph_button(
                widget::icon::from_svg_bytes(repeat_icon).symbolic(true),
                repeat_message,
            )
            .class(style::control(repeat)),
            widget::text(if repeat {
                fl!("repeat-track")
            } else {
                fl!("repeat-disabled")
            })
            .size(13),
            widget::tooltip::Position::Top,
        );
        let skip = |forward| {
            let (handle, message) = if video.has_video() {
                (
                    widget::icon::from_svg_bytes(if forward {
                        JUMP_FORWARD_ICON
                    } else {
                        JUMP_BACKWARD_ICON
                    })
                    .symbolic(true),
                    Message::SeekRelative(if forward { 10.0 } else { -10.0 }),
                )
            } else if forward {
                (icon("media-skip-forward-symbolic"), Message::PlayNext)
            } else {
                (icon("media-skip-backward-symbolic"), Message::PlayPrev)
            };
            widget::button::icon(handle)
                .class(style::control(false))
                .padding(6)
                .on_press(message)
        };
        let play = icon_button(
            if video.paused() {
                "media-playback-start-symbolic"
            } else {
                "media-playback-pause-symbolic"
            },
            Message::PlayPause,
        )
        .class(style::play_button())
        .padding(10);
        let volume = video.volume();
        let volume_icon = if video.muted() {
            "audio-volume-muted-symbolic"
        } else if volume >= 2.0 / 3.0 {
            "audio-volume-high-symbolic"
        } else if volume >= 1.0 / 3.0 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-low-symbolic"
        };
        let volume_slider = || {
            Slider::new(0.0..=1.0, volume, Message::AudioVolume)
                .step(0.01)
                .width(84)
                .class(style::scrubber())
        };
        let tracks = self.popup(
            icon_button(
                "media-view-subtitles-symbolic",
                Message::DropdownToggle(DropdownKind::Subtitle),
            )
            .class(style::control(
                self.dropdown_opt == Some(DropdownKind::Subtitle),
            )),
            DropdownKind::Subtitle,
            || self.track_choices(),
        );
        let mut right = widget::row::with_children(vec![
            tracks,
            icon_button("view-fullscreen-symbolic", Message::Fullscreen).into(),
        ])
        .spacing(3)
        .align_y(Alignment::Center);
        if self.core.is_condensed() {
            right = right.push(self.popup(
                icon_button(volume_icon, Message::DropdownToggle(DropdownKind::Audio)),
                DropdownKind::Audio,
                || {
                    widget::row::with_children(vec![
                        icon_button(volume_icon, Message::AudioToggle).into(),
                        volume_slider().into(),
                    ])
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .into()
                },
            ));
        } else {
            right = right
                .push(icon_button(volume_icon, Message::AudioToggle))
                .push(volume_slider());
        }
        let buttons = widget::row::with_children(vec![
            widget::container(repeat_button).width(Length::Fill).into(),
            widget::row::with_children(vec![skip(false).into(), play.into(), skip(true).into()])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            widget::container(right)
                .width(Length::Fill)
                .align_x(Alignment::End)
                .into(),
        ])
        .align_y(Alignment::Center);
        widget::container(widget::column::with_children(vec![
            divider(),
            widget::container(
                widget::column::with_children(vec![self.timeline(), buttons.into()]).spacing(1),
            )
            .padding([0, 12, 7, 12])
            .into(),
        ]))
        .class(style::surface())
        .into()
    }

    fn popup<'a>(
        &self,
        button: widget::button::Button<'a, Message>,
        kind: DropdownKind,
        content: impl FnOnce() -> Element<'a, Message>,
    ) -> Element<'a, Message> {
        if self.dropdown_opt != Some(kind) {
            return button.into();
        }
        widget::popover(button)
            .position(widget::popover::Position::Point(Point::new(-190.0, 26.0)))
            .on_close(Message::DropdownClose)
            .popup(
                widget::container(content())
                    .padding(8)
                    .class(style::popover()),
            )
            .into()
    }

    fn track_choices(&self) -> Element<'_, Message> {
        let mut choices =
            widget::column::with_capacity(self.audio_codes.len() + self.text_codes.len() + 2)
                .spacing(2);
        for (title, is_audio) in [(fl!("audio"), true), (fl!("subtitles"), false)] {
            let count = if is_audio {
                self.audio_codes.len()
            } else {
                self.text_codes.len()
            };
            if count == 0 {
                continue;
            }
            choices = choices.push(
                widget::container(widget::text(title).size(12).font(font::semibold()))
                    .padding([4, 6])
                    .class(style::muted()),
            );
            for index in 0..count {
                let (name, selected, message) = if is_audio {
                    (
                        &self.audio_codes[index],
                        usize::try_from(self.current_audio).ok() == Some(index),
                        Message::AudioCode(index),
                    )
                } else {
                    let track = &self.text_codes[index];
                    (
                        &track.name,
                        track.id == self.current_text,
                        Message::TextCode(index),
                    )
                };
                let marker: Element<_> = if selected {
                    widget::icon::from_name("object-select-symbolic")
                        .size(14)
                        .into()
                } else {
                    widget::space::horizontal().width(14).into()
                };
                choices = choices.push(
                    widget::button::custom(
                        widget::row::with_children(vec![
                            marker,
                            widget::text(name).size(13).into(),
                        ])
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .class(style::control(selected))
                    .width(Length::Fill)
                    .padding([4, 6])
                    .on_press(message),
                );
            }
        }
        widget::container(widget::scrollable(choices))
            .width(220)
            .max_height(240)
            .into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let path = self
            .flags
            .url_opt
            .as_ref()
            .and_then(|url| url.to_file_path().ok())
            .map(|path| {
                let home = std::env::home_dir();
                let (prefix, path) = home
                    .as_ref()
                    .and_then(|home| path.strip_prefix(home).ok())
                    .map_or((None, path.as_path()), |relative| (Some("~"), relative));
                prefix
                    .into_iter()
                    .map(str::to_owned)
                    .chain(
                        path.components()
                            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
                    )
                    .collect::<Vec<_>>()
                    .join("  ›  ")
            })
            .unwrap_or_else(|| self.media_title());
        let mut row = widget::row::with_children(vec![
            widget::icon::from_name("folder-symbolic").size(12).into(),
            widget::text(path)
                .size(11)
                .width(Length::Fill)
                .ellipsize(Ellipsize::Middle(EllipsizeHeightLimit::Lines(1)))
                .into(),
        ])
        .spacing(6)
        .align_y(Alignment::Center);
        let status = match self.ab_repeat {
            Some((Some(a), Some(b))) => {
                Some(format!("{} {} – {}", fl!("ab-repeat"), time(a), time(b)))
            }
            Some((Some(a), None)) => Some(format!("{} A {}", fl!("ab-repeat"), time(a))),
            _ if self.flags.config_state.player_state.repeat == RepeatState::Track => {
                Some(fl!("repeat-track"))
            }
            _ => None,
        };
        if let Some(status) = status {
            row = row.push(widget::text(status).size(11));
        }
        widget::column::with_children(vec![
            divider(),
            widget::container(row)
                .padding([6, 12])
                .class(style::muted())
                .into(),
        ])
        .into()
    }
}
