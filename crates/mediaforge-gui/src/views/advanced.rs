use egui;
use mediaforge_core::enums::{
    AudioCodec, Container, DeinterlaceMethod, FpsMode, MovFlag, PixelFormat, PresetSpeed, Profile,
    ScaleAlgorithm, VideoCodec,
};
use mediaforge_core::ffmpeg;

use crate::app::{AdvancedTab, MediaForgeApp};
use crate::i18n;

impl MediaForgeApp {
    pub fn render_advanced_mode(&mut self, ui: &mut egui::Ui) {
        // Tab bar
        ui.horizontal(|ui| {
            for tab in &[
                AdvancedTab::Video,
                AdvancedTab::Audio,
                AdvancedTab::Filters,
                AdvancedTab::Output,
                AdvancedTab::Metadata,
            ] {
                let label = i18n::t(self.lang, match tab {
                    AdvancedTab::Video => "tab-video",
                    AdvancedTab::Audio => "tab-audio",
                    AdvancedTab::Filters => "tab-filters",
                    AdvancedTab::Output => "tab-output",
                    AdvancedTab::Metadata => "tab-metadata",
                });
                ui.selectable_value(&mut self.advanced_tab, *tab, label);
            }
        });

        ui.separator();

        // Scrollable content for advanced mode
        egui::ScrollArea::vertical()
            .id_salt("advanced_scroll")
            .show(ui, |ui| {
                match self.advanced_tab {
                    AdvancedTab::Video => self.render_advanced_video(ui),
                    AdvancedTab::Audio => self.render_advanced_audio(ui),
                    AdvancedTab::Filters => self.render_advanced_filters(ui),
                    AdvancedTab::Output => self.render_advanced_output(ui),
                    AdvancedTab::Metadata => self.render_advanced_metadata(ui),
                }
            });
    }

    fn render_advanced_video(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Codec & Quality")
            .default_open(true)
            .show(ui, |ui| {
                // Video codec
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-video-codec"));
                    egui::ComboBox::from_id_salt("adv_vcodec")
                        .selected_text(self.params.video_codec.label())
                        .show_ui(ui, |ui| {
                            for c in VideoCodec::ALL {
                                ui.selectable_value(&mut self.params.video_codec, *c, c.label());
                            }
                        });
                });

                // CRF
                ui.horizontal(|ui| {
                    ui.label("CRF");
                    let mut crf = self.params.crf.unwrap_or(23);
                    if ui.add(egui::Slider::new(&mut crf, 0..=51)).changed() {
                        self.params.crf = Some(crf);
                    }
                    if ui.button("Auto").clicked() {
                        self.params.crf = None;
                    }
                });

                // Video bitrate
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-video-bitrate"));
                    let mut br = self.params.video_bitrate.unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut br).range(0..=100000).speed(100))
                        .changed()
                    {
                        self.params.video_bitrate = if br > 0 { Some(br) } else { None };
                    }
                });

                // Max bitrate + bufsize
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-max-bitrate"));
                    let mut maxbr = self.params.max_bitrate.unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut maxbr).range(0..=100000).speed(100))
                        .changed()
                    {
                        self.params.max_bitrate = if maxbr > 0 { Some(maxbr) } else { None };
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-bufsize"));
                    let mut buf = self.params.bufsize.unwrap_or(0);
                    if ui
                        .add(egui::DragValue::new(&mut buf).range(0..=100000).speed(100))
                        .changed()
                    {
                        self.params.bufsize = if buf > 0 { Some(buf) } else { None };
                    }
                });

                // Preset speed
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-preset-speed"));
                    egui::ComboBox::from_id_salt("adv_preset_speed")
                        .selected_text(self.params.preset.ffmpeg_name())
                        .show_ui(ui, |ui| {
                            for p in PresetSpeed::ALL {
                                ui.selectable_value(&mut self.params.preset, *p, p.ffmpeg_name());
                            }
                        });
                });

                // Profile
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-profile"));
                    let profile_str = self
                        .params
                        .profile
                        .map(|p| p.ffmpeg_name().to_string())
                        .unwrap_or_else(|| "auto".to_string());
                    egui::ComboBox::from_id_salt("adv_profile")
                        .selected_text(profile_str)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(false, "auto").clicked() {
                                self.params.profile = None;
                            }
                            for p in Profile::ALL {
                                if ui
                                    .selectable_label(self.params.profile == Some(*p), p.ffmpeg_name())
                                    .clicked()
                                {
                                    self.params.profile = Some(*p);
                                }
                            }
                        });
                });
            });

        egui::CollapsingHeader::new("Resolution & Framerate")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-resolution"));
                    ui.add(
                        egui::DragValue::new(&mut self.params.width)
                            .range(64..=7680)
                            .speed(16),
                    );
                    ui.label("×");
                    ui.add(
                        egui::DragValue::new(&mut self.params.height)
                            .range(64..=4320)
                            .speed(16),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-scale-algo"));
                    egui::ComboBox::from_id_salt("adv_scale_algo")
                        .selected_text(match self.params.scale_algorithm {
                            ScaleAlgorithm::Bilinear => "Bilinear",
                            ScaleAlgorithm::Bicubic => "Bicubic",
                            ScaleAlgorithm::Lanczos => "Lanczos",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.params.scale_algorithm,
                                ScaleAlgorithm::Bilinear,
                                "Bilinear",
                            );
                            ui.selectable_value(
                                &mut self.params.scale_algorithm,
                                ScaleAlgorithm::Bicubic,
                                "Bicubic",
                            );
                            ui.selectable_value(
                                &mut self.params.scale_algorithm,
                                ScaleAlgorithm::Lanczos,
                                "Lanczos",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-fps"));
                    match &mut self.params.fps {
                        FpsMode::SameAsSource => {
                            ui.label("Same as source");
                            if ui.button("Custom").clicked() {
                                self.params.fps = FpsMode::Fixed(30);
                            }
                        }
                        FpsMode::Fixed(fps) => {
                            ui.add(egui::DragValue::new(fps).range(1..=120));
                            if ui.button("Auto").clicked() {
                                self.params.fps = FpsMode::SameAsSource;
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-deinterlace"));
                    let mut deint = self.params.deinterlace.is_some();
                    if ui.checkbox(&mut deint, "Enabled").changed() {
                        self.params.deinterlace = if deint {
                            Some(DeinterlaceMethod::Yadif)
                        } else {
                            None
                        };
                    }
                    if deint {
                        egui::ComboBox::from_id_salt("adv_deint_method")
                            .selected_text(
                                self.params
                                    .deinterlace
                                    .map(|d| d.ffmpeg_name())
                                    .unwrap_or("yadif"),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.params.deinterlace,
                                    Some(DeinterlaceMethod::Yadif),
                                    "yadif",
                                );
                                ui.selectable_value(
                                    &mut self.params.deinterlace,
                                    Some(DeinterlaceMethod::Bwdif),
                                    "bwdif",
                                );
                            });
                    }
                });
            });

        egui::CollapsingHeader::new("Pixel Format & Threads")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-pixel-format"));
                    egui::ComboBox::from_id_salt("adv_pix_fmt")
                        .selected_text(self.params.pixel_format.ffmpeg_name())
                        .show_ui(ui, |ui| {
                            for pf in PixelFormat::ALL {
                                ui.selectable_value(
                                    &mut self.params.pixel_format,
                                    *pf,
                                    pf.ffmpeg_name(),
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-threads"));
                    ui.add(
                        egui::DragValue::new(&mut self.params.threads)
                            .range(0..=32)
                            .speed(1),
                    );
                });
            });
    }

    fn render_advanced_audio(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Audio Codec")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-audio-codec"));
                    egui::ComboBox::from_id_salt("adv_acodec")
                        .selected_text(self.params.audio_codec.label())
                        .show_ui(ui, |ui| {
                            for c in AudioCodec::ALL {
                                ui.selectable_value(&mut self.params.audio_codec, *c, c.label());
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-audio-quality"));
                    ui.add(
                        egui::Slider::new(&mut self.params.audio_bitrate, 32..=320).text("kbps"),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-audio-channels"));
                    ui.add(
                        egui::DragValue::new(&mut self.params.audio_channels)
                            .range(1..=8)
                            .speed(1),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-sample-rate"));
                    egui::ComboBox::from_id_salt("adv_sample_rate")
                        .selected_text(self.params.sample_rate.to_string())
                        .show_ui(ui, |ui| {
                            for rate in &[8000u32, 11025, 16000, 22050, 44100, 48000, 96000] {
                                ui.selectable_value(
                                    &mut self.params.sample_rate,
                                    *rate,
                                    rate.to_string(),
                                );
                            }
                        });
                });
            });
    }

    fn render_advanced_filters(&mut self, ui: &mut egui::Ui) {
        // Video filters — current list
        ui.label(i18n::t(self.lang, "label-video-filters"));
        ui.add_space(4.0);

        let vfilters = self.params.video_filters.clone();
        let mut to_remove: Option<usize> = None;
        for (i, filter) in vfilters.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(filter.label());
                if ui.button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(i) = to_remove {
            self.params.video_filters.remove(i);
        }

        // Add common filters
        ui.horizontal(|ui| {
            ui.menu_button("+ Add Video Filter", |ui| {
                if ui.button("Flip Horizontal").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::HFlip);
                    ui.close();
                }
                if ui.button("Flip Vertical").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::VFlip);
                    ui.close();
                }
                if ui.button("Rotate 90°").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::Rotate(90));
                    ui.close();
                }
                if ui.button("Rotate 180°").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::Rotate(180));
                    ui.close();
                }
                if ui.button("Denoise (hqdn3d)").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::Denoise);
                    ui.close();
                }
                if ui.button("Grayscale").clicked() {
                    self.params
                        .video_filters
                        .push(mediaforge_core::enums::VideoFilter::Grayscale);
                    ui.close();
                }
            });
        });

        ui.add_space(12.0);
        ui.separator();

        // Audio filters
        ui.label(i18n::t(self.lang, "label-audio-filters"));
        ui.add_space(4.0);

        let afilters = self.params.audio_filters.clone();
        let mut to_remove_a: Option<usize> = None;
        for (i, filter) in afilters.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(filter.label());
                if ui.button("✕").clicked() {
                    to_remove_a = Some(i);
                }
            });
        }
        if let Some(i) = to_remove_a {
            self.params.audio_filters.remove(i);
        }

        ui.horizontal(|ui| {
            ui.menu_button("+ Add Audio Filter", |ui| {
                if ui.button("Volume 2x").clicked() {
                    self.params
                        .audio_filters
                        .push(mediaforge_core::enums::AudioFilter::Volume(2.0));
                    ui.close();
                }
                if ui.button("Volume 0.5x").clicked() {
                    self.params
                        .audio_filters
                        .push(mediaforge_core::enums::AudioFilter::Volume(0.5));
                    ui.close();
                }
                if ui.button("Loudnorm (EBU R128)").clicked() {
                    self.params
                        .audio_filters
                        .push(mediaforge_core::enums::AudioFilter::Loudnorm);
                    ui.close();
                }
            });
        });

        ui.add_space(12.0);
        ui.separator();

        // Extra args
        ui.label(i18n::t(self.lang, "label-extra-args"));
        let mut extra = self.params.extra_args.join(" ");
        ui.text_edit_singleline(&mut extra);
        self.params.extra_args = extra.split_whitespace().map(|s| s.to_string()).collect();
    }

    fn render_advanced_output(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Container & Flags")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-container"));
                    egui::ComboBox::from_id_salt("adv_container")
                        .selected_text(self.params.container.label())
                        .show_ui(ui, |ui| {
                            for c in Container::ALL {
                                ui.selectable_value(&mut self.params.container, *c, c.label());
                            }
                        });
                });

                ui.label(i18n::t(self.lang, "label-movflags"));
                let mut faststart = self.params.movflags.contains(&MovFlag::FastStart);
                if ui
                    .checkbox(&mut faststart, i18n::t(self.lang, "label-faststart"))
                    .changed()
                {
                    if faststart {
                        if !self.params.movflags.contains(&MovFlag::FastStart) {
                            self.params.movflags.push(MovFlag::FastStart);
                        }
                    } else {
                        self.params.movflags.retain(|f| *f != MovFlag::FastStart);
                    }
                }
                let mut frag = self.params.movflags.contains(&MovFlag::FragKeyframe);
                if ui
                    .checkbox(&mut frag, i18n::t(self.lang, "label-frag-keyframe"))
                    .changed()
                {
                    if frag {
                        if !self.params.movflags.contains(&MovFlag::FragKeyframe) {
                            self.params.movflags.push(MovFlag::FragKeyframe);
                        }
                    } else {
                        self.params.movflags.retain(|f| *f != MovFlag::FragKeyframe);
                    }
                }
            });

        egui::CollapsingHeader::new("Trim")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-trim-start"));
                    let mut ss = self.params.trim_start.clone().unwrap_or_default();
                    ui.text_edit_singleline(&mut ss);
                    self.params.trim_start = if ss.is_empty() { None } else { Some(ss) };
                    ui.label("(e.g. 00:01:30 or 90)");
                });
                ui.horizontal(|ui| {
                    ui.label(i18n::t(self.lang, "label-trim-end"));
                    let mut to = self.params.trim_end.clone().unwrap_or_default();
                    ui.text_edit_singleline(&mut to);
                    self.params.trim_end = if to.is_empty() { None } else { Some(to) };
                    ui.label("(e.g. 00:05:00 or 300)");
                });
            });

        egui::CollapsingHeader::new(i18n::t(self.lang, "label-command-preview"))
            .default_open(false)
            .show(ui, |ui| {
                let dummy_input = std::path::Path::new("input.mp4");
                let dummy_output = std::path::Path::new("output.mp4");
                let cmd = ffmpeg::command_to_string_with_ffmpeg(
                    &self.params,
                    dummy_input,
                    dummy_output,
                    self.ffmpeg_path.as_deref(),
                );
                ui.add_sized(
                    [ui.available_width(), 60.0],
                    egui::TextEdit::multiline(&mut cmd.as_str())
                        .font(egui::TextStyle::Monospace)
                        .interactive(false),
                );
            });
    }

    fn render_advanced_metadata(&mut self, ui: &mut egui::Ui) {
        ui.label("Metadata key-value pairs passed to ffmpeg -metadata:");

        let mut to_remove: Option<String> = None;
        let entries: Vec<(String, String)> = self
            .params
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (key, value) in &entries {
            ui.horizontal(|ui| {
                ui.label(format!("{key}: {value}"));
                if ui.button("✕").clicked() {
                    to_remove = Some(key.clone());
                }
            });
        }
        if let Some(key) = to_remove {
            self.params.metadata.remove(&key);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(i18n::t(self.lang, "label-metadata-key"));
            ui.text_edit_singleline(&mut self.new_meta_key);
            ui.label(i18n::t(self.lang, "label-metadata-value"));
            ui.text_edit_singleline(&mut self.new_meta_val);

            if ui
                .button(i18n::t(self.lang, "label-add-metadata"))
                .clicked()
                && !self.new_meta_key.is_empty()
            {
                self.params
                    .metadata
                    .insert(self.new_meta_key.clone(), self.new_meta_val.clone());
                self.new_meta_key.clear();
                self.new_meta_val.clear();
            }
        });
    }
}
