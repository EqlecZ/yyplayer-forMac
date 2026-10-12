use super::AppController;
use std::time::Instant;

impl AppController {
    pub(super) fn lyric_control(&mut self, action: &str, value: &str) -> bool {
        let Some(field) = action.strip_prefix("lyrics-style-") else {
            return false;
        };
        let mut next = self.settings.lyrics.clone();
        match field {
            "reset" => next = Default::default(),
            "album-color" | "bilingual" => {
                let enabled = match value {
                    "yes" => true,
                    "no" => false,
                    _ => return true,
                };
                if field == "album-color" {
                    next.album_color = enabled;
                } else {
                    next.bilingual = enabled;
                }
            }
            _ => {
                let Ok(number) = value.parse::<u16>() else {
                    return true;
                };
                match field {
                    "line-gap" => next.line_gap = number,
                    "inactive-size" => next.inactive_size = number,
                    "inactive-opacity" => {
                        let Ok(number) = u8::try_from(number) else {
                            return true;
                        };
                        next.inactive_opacity = number;
                    }
                    "inactive-weight" => next.inactive_weight = number,
                    "active-size" => next.active_size = number,
                    "active-weight" => next.active_weight = number,
                    "secondary-weight" => next.secondary_weight = number,
                    "bilingual-gap" => next.bilingual_gap = number,
                    _ => return false,
                }
            }
        }
        if let Err(error) = next.validate() {
            self.status = error;
            return true;
        }
        if next != self.settings.lyrics {
            self.settings.lyrics = next;
            self.settings_revision += 1;
            self.settings_dirty = Some(Instant::now());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use player_ui::UiAction;
    #[test]
    fn style_edits_are_independent_validated_and_only_dirty_on_change() {
        let mut c =
            AppController::new(player_mpv::MpvEngine::default(), player_platform::current());
        let defaults = c.settings.lyrics.clone();
        for (field, value) in [
            ("line-gap", "80"),
            ("inactive-size", "42"),
            ("inactive-opacity", "30"),
            ("inactive-weight", "300"),
            ("active-size", "56"),
            ("active-weight", "800"),
            ("secondary-weight", "500"),
            ("bilingual-gap", "18"),
            ("album-color", "no"),
            ("bilingual", "no"),
        ] {
            c.dispatch(UiAction::Control(
                format!("lyrics-style-{field}"),
                value.into(),
            ));
        }
        assert_eq!(c.settings.lyrics.line_gap, 80);
        assert_eq!(c.settings.lyrics.secondary_weight, 500);
        assert_eq!(c.settings.lyrics.bilingual_gap, 18);
        assert_eq!(c.settings.lyrics.inactive_opacity, 30);
        assert!(!c.settings.lyrics.album_color);
        let valid = c.settings.lyrics.clone();
        let revision = c.settings_revision;
        for (field, value) in [
            ("line-gap", "NaN"),
            ("line-gap", "-1"),
            ("active-weight", "550"),
            ("bilingual-gap", "41"),
            ("inactive-opacity", "999"),
            ("album-color", "maybe"),
            ("line-gap", "80"),
        ] {
            c.dispatch(UiAction::Control(
                format!("lyrics-style-{field}"),
                value.into(),
            ));
            assert_eq!(c.settings.lyrics, valid);
            assert_eq!(c.settings_revision, revision);
        }
        c.dispatch(UiAction::Control(
            "lyrics-style-reset".into(),
            String::new(),
        ));
        assert_eq!(c.settings.lyrics, defaults);
    }
}
