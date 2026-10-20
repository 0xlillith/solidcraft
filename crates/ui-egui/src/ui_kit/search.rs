//! Command-search composition: a taller editor and two-line MenuItem variant.
//! Domain ranking and command dispatch stay with the caller.
use crate::{icons, theme::Tokens};
use egui::{FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

#[derive(Clone, Copy)]
pub(crate) enum SearchBadge {
    Kind,
    Valid,
    Invalid,
}

pub(crate) struct CommandSearch;

impl CommandSearch {
    pub(crate) fn width() -> f32 {
        let t = Tokens::get();
        t.form_card_width * 2.0 - t.space_8
    }

    pub(crate) fn field(ui: &mut Ui, value: &mut String, invalid: bool, focus: bool, limit: usize) -> Response {
        let t = Tokens::get();
        let height = t.control_height + t.space_12;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let focused = focus || ui.memory(|m| m.has_focus(egui::Id::new("sc_sbox_text")));
        let ink = if invalid {
            t.form_error
        } else if focused {
            t.form_primary
        } else {
            t.control_border
        };
        if focused {
            ui.painter().rect_filled(r.expand(t.space_3), t.radius_xl, if invalid { t.form_error_fill } else { t.accent_soft });
        }
        ui.painter().rect(r, t.radius_xl, t.control_fill, Stroke::new(1.0, ink), egui::StrokeKind::Inside);
        icons::paint_icon(
            ui.painter(),
            Rect::from_center_size(r.left_center() + vec2(t.space_12 + t.space_8, 0.0), vec2(t.space_16, t.space_16)),
            "zoom",
            1.0,
        );
        let cap = if value.is_empty() { "S" } else { "Esc" };
        Self::key(ui, r.right_center() - vec2(t.space_10, 0.0), cap);
        let editor = Rect::from_min_max(r.min + vec2(t.space_24 + t.space_16, 0.0), r.max - vec2(t.space_24 * 2.0, 0.0));
        let mono = value.contains('{');
        let response = ui
            .scope_builder(egui::UiBuilder::new().max_rect(editor), |ui| {
                ui.add_sized(
                    editor.size(),
                    super::fields::text_input(egui::TextEdit::singleline(value))
                        .id(egui::Id::new("sc_sbox_text"))
                        .char_limit(limit)
                        .frame(egui::Frame::NONE)
                        .hint_text(crate::i18n::tr_label("Search commands"))
                        .font(if mono { FontId::monospace(t.font_label) } else { FontId::proportional(t.font_button) }),
                )
            })
            .inner;
        if focus {
            response.request_focus();
        }
        crate::scenario::publish_target("shortcut:search", r);
        crate::scenario::publish_target("palette:search", r);
        response
    }

    fn key(ui: &Ui, right: egui::Pos2, label: &str) -> f32 {
        let t = Tokens::get();
        let galley = ui.painter().layout_no_wrap(label.to_owned(), FontId::monospace(t.font_small), t.form_dim);
        let size = galley.size() + vec2(t.space_10, t.space_4);
        let r = Rect::from_min_size(pos2(right.x - size.x, right.y - size.y / 2.0), size);
        ui.painter().rect(r, t.radius_sm, t.dialog_footer, Stroke::new(1.0, t.dialog_rule), egui::StrokeKind::Inside);
        ui.painter().galley(r.min + vec2(t.space_10 / 2.0, t.space_2), galley, t.form_dim);
        size.x
    }

    pub(crate) fn heading(ui: &mut Ui, label: &str, count: usize) {
        super::Menu::heading(ui, &format!("{}  {count}", crate::i18n::tr(label)));
    }

    pub(crate) fn detail(ui: &mut Ui, text: &str) {
        let t = Tokens::get();
        egui::Frame::NONE.fill(t.control_fill).stroke(Stroke::new(1.0, t.dialog_rule)).corner_radius(t.control_radius).inner_margin(t.space_8).show(
            ui,
            |ui| {
                super::text(ui, text, super::Text::MonoNote("command_parameters"));
            },
        );
    }

    pub(crate) fn footer(ui: &mut Ui, command: bool) {
        let t = Tokens::get();
        ui.add_space(t.space_6);
        ui.painter().hline(ui.available_rect_before_wrap().x_range(), ui.cursor().top(), Stroke::new(1.0, t.dialog_rule));
        egui::Frame::NONE.fill(t.dialog_footer).inner_margin(t.space_6).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - t.space_12).max(0.0));
            super::text(ui, crate::i18n::tr(if command { "search.command_keys" } else { "search.keys" }), super::Text::Note("search_keys"));
        });
    }
}

impl super::MenuItem<'_> {
    /// 38 pt search row: shared menu states, 26 pt tile, highlighted match, path and keycap.
    pub(crate) fn search_row(&self, ui: &mut Ui, query: &str, path: &str, badge: Option<SearchBadge>) -> Response {
        let t = Tokens::get();
        let (r, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.control_height + t.space_10),
            if self.enabled { Sense::click() } else { Sense::hover() },
        );
        let hot = self.enabled && (self.highlighted || response.hovered());
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(r));
        if hot {
            painter.rect_filled(r, t.control_radius, t.accent_soft);
        }
        let tile = Rect::from_center_size(
            pos2(r.left() + t.space_8 + (t.space_24 + t.space_2) / 2.0, r.center().y),
            vec2(t.space_24 + t.space_2, t.space_24 + t.space_2),
        );
        painter.rect_filled(tile, t.control_radius, if hot { t.accent_soft } else { t.dialog_footer });
        icons::paint_icon(&painter, tile.shrink(t.space_6), self.icon, if self.enabled { 1.0 } else { 0.4 });
        let right = r.right() - t.space_8;
        let key_width = if self.shortcut.is_empty() {
            0.0
        } else if let Some(badge) = badge {
            let ink = match badge {
                SearchBadge::Kind => t.form_dim,
                SearchBadge::Valid => t.form_success,
                SearchBadge::Invalid => t.form_error,
            };
            let galley = painter.layout_no_wrap(self.shortcut.to_owned(), crate::fonts::semibold(t.font_small), ink);
            let size = galley.size() + vec2(t.space_16, t.space_4);
            let rect = Rect::from_min_size(pos2(right - size.x, r.center().y - size.y / 2.0), size);
            painter.rect_filled(
                rect,
                t.radius_xl,
                match badge {
                    SearchBadge::Invalid => t.form_error_fill,
                    SearchBadge::Valid => t.form_success_fill,
                    SearchBadge::Kind => t.dialog_footer,
                },
            );
            painter.galley(rect.min + vec2(t.space_8, t.space_2), galley, ink);
            size.x + t.space_10
        } else {
            CommandSearch::key(ui, pos2(right, r.center().y), self.shortcut) + t.space_10
        };
        let left = tile.right() + t.space_10;
        let label = self.label;
        let normal = egui::TextFormat::simple(FontId::proportional(t.font_label), if self.enabled { t.form_ink } else { t.form_dim });
        let mut job = egui::text::LayoutJob::default();
        // Match by character boundaries, including case mappings that change UTF-8 byte lengths.
        let q = query.trim().to_lowercase();
        let hit = (!q.is_empty())
            .then(|| {
                label.char_indices().find_map(|(start, _)| {
                    let mut lowered = String::new();
                    for (offset, c) in label.get(start..)?.char_indices() {
                        lowered.extend(c.to_lowercase());
                        if lowered == q {
                            return Some((start, start + offset + c.len_utf8()));
                        }
                        if !q.starts_with(&lowered) {
                            break;
                        }
                    }
                    None
                })
            })
            .flatten();
        if let Some((start, end)) = hit {
            job.append(label.get(..start).unwrap_or_default(), 0.0, normal.clone());
            job.append(label.get(start..end).unwrap_or_default(), 0.0, egui::TextFormat::simple(crate::fonts::bold(t.font_label), t.selected_ink));
            job.append(label.get(end..).unwrap_or_default(), 0.0, normal);
        } else {
            job.append(label, 0.0, normal);
        }
        job.wrap.max_width = (right - key_width - left).max(0.0);
        job.wrap.max_rows = 1;
        let galley = painter.layout_job(job);
        painter.galley(pos2(left, r.top() + t.space_3), galley, t.form_ink);
        let path_rect = Rect::from_min_max(pos2(left, r.center().y), pos2((right - key_width).max(left), r.bottom() - t.space_2));
        if let Some((tab, group)) = path.split_once('›') {
            let tab = painter.layout_no_wrap(tab.trim().to_owned(), FontId::proportional(t.font_small), t.form_dim);
            let x = (left + tab.size().x + t.space_6).min(path_rect.right());
            painter.with_clip_rect(path_rect).galley(pos2(left, path_rect.center().y - tab.size().y / 2.0), tab, t.form_dim);
            icons::glyphs::breadcrumb(&painter.with_clip_rect(path_rect), pos2(x, path_rect.center().y), t.form_dim);
            super::text::paint_text(
                ui,
                Rect::from_min_max(pos2((x + t.space_6).min(path_rect.right()), path_rect.top()), path_rect.max),
                group.trim(),
                FontId::proportional(t.font_small),
                t.form_dim,
                false,
            );
        } else {
            super::text::paint_text(ui, path_rect, path, FontId::proportional(t.font_small), t.form_dim, false);
        }
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, self.enabled, label));
        crate::scenario::publish_target(&format!("menu:{}", self.id), r);
        response
    }
}
