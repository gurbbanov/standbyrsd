use crate::l10n::L10n;
use crate::message::Message;
use crate::widgets::{ClearCache, WidgetId};
use crate::{SF_PRO_DISPLAY_BLACK, SF_PRO_DISPLAY_BOLD};
use chrono::*;
use iced::widget::canvas::{Cache, Path};
use iced::widget::{canvas, text};
use iced::{Element, Length, Pixels, Point, Rectangle, Renderer, Theme, alignment, color, mouse};
use std::cell::Cell;

pub struct CalendarWidget {
    pub id: WidgetId,
    pub style: CalendarStyle,
}

impl CalendarWidget {
    pub fn new_with_id(id: WidgetId, style: CalendarStyle) -> Self {
        Self {
            id: id,
            style: style,
        }
    }

    pub fn view<'a>(&'a self, l10n: &'a L10n, time: &'a DateTime<Utc>) -> Element<'a, Message> {
        self.style.view(l10n, time)
    }
}

impl ClearCache for CalendarWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

pub enum CalendarStyle {
    MonthHalf(MonthCalendarHalf),
    DateHalf(DateCalendarHalf),
}

impl CalendarStyle {
    fn view<'a>(&'a self, l10n: &'a L10n, time: &'a DateTime<Utc>) -> Element<'a, Message> {
        match self {
            CalendarStyle::MonthHalf(c) => c.view(l10n, time),
            CalendarStyle::DateHalf(c) => c.view(l10n, time),
        }
    }
}

impl ClearCache for CalendarStyle {
    fn clear_cache(&self) {
        match self {
            CalendarStyle::MonthHalf(c) => c.cache.clear(),
            CalendarStyle::DateHalf(c) => c.cache.clear(),
        }
    }
}

#[derive(Default)]
pub struct MonthCalendarHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl MonthCalendarHalf {
    fn view<'a>(&'a self, l10n: &'a L10n, time: &'a DateTime<Utc>) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, l10n, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a MonthCalendarHalf, &'a L10n, &'a DateTime<Utc>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, l10n, now) = self;
        let palette = theme.palette();

        let layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let w = frame.width() * 0.95;
            let h = frame.height();

            let first_day_of_month = weekday_to_number(
                &NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                    .unwrap()
                    .weekday(),
            );

            let last_day_of_month = NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap()
                .day() as usize;

            let num_rows =
                ((first_day_of_month - 1 + last_day_of_month) as f32 / 7.0).ceil() as usize;

            let columns = 7usize;

            let cell_w_by_width = w / columns as f32;
            let total_rows = (num_rows + 2) as f32;
            let cell_w_by_height = h / total_rows;
            let cell_w = cell_w_by_width.min(cell_w_by_height);

            let cell_h = cell_w;
            let font_size = cell_w * 0.38;
            let month_font_size = cell_w * 0.6;

            let grid_w = cell_w * columns as f32;
            let total_h = month_font_size + cell_h * (1.0 + num_rows as f32);
            let offset_x = (w - grid_w) * 0.5;
            let offset_y = (h - total_h) * 0.5;

            frame.fill_text(canvas::Text {
                content: format!("   {}", l10n.get(&format!("month-{}", now.month())))
                    .to_uppercase(),
                position: Point::new(offset_x, offset_y + month_font_size * 0.5),
                size: month_font_size.into(),
                color: color!(255, 0, 0),
                font: SF_PRO_DISPLAY_BLACK,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });

            let weekdays: Vec<String> = (1..=7)
                .map(|i| l10n.get(&format!("weekday-short-{}", i)))
                .collect();

            for (col, label) in weekdays.iter().enumerate() {
                let x = offset_x + col as f32 * cell_w + cell_w * 0.5;
                let y = offset_y + month_font_size + cell_h * 0.5;
                let is_weekend = col >= 5;
                frame.fill_text(canvas::Text {
                    content: label.to_string(),
                    position: Point::new(x, y),
                    size: font_size.into(),
                    color: if is_weekend {
                        palette.danger
                    } else {
                        palette.text
                    },
                    font: SF_PRO_DISPLAY_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }

            let mut slot = first_day_of_month - 1;

            for day in 1..=last_day_of_month {
                let col = slot % 7;
                let row = slot / 7;

                let x = offset_x + col as f32 * cell_w + cell_w * 0.5;
                let y = offset_y + month_font_size + cell_h + row as f32 * cell_h + cell_h * 0.5;

                let is_today = day == now.day() as usize;
                let is_weekend = col >= 5;

                if is_today {
                    let r = cell_w * 0.5;
                    frame.fill(&Path::circle(Point::new(x, y), r), color!(255, 0, 0));
                }

                frame.fill_text(canvas::Text {
                    content: day.to_string(),
                    position: Point::new(x, y),
                    size: font_size.into(),
                    color: if is_today {
                        palette.success
                    } else if is_weekend {
                        palette.danger
                    } else {
                        palette.text
                    },
                    font: SF_PRO_DISPLAY_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });

                slot += 1;
            }
        });

        vec![layer]
    }
}

#[derive(Default)]
pub struct DateCalendarHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl DateCalendarHalf {
    fn view<'a>(&'a self, l10n: &'a L10n, time: &'a DateTime<Utc>) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, l10n, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a DateCalendarHalf, &'a L10n, &'a DateTime<Utc>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, l10n, time) = self;
        let palette = theme.palette();
        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            frame.with_save(|frame| {
                let size = frame.width().min(frame.height());
                let center = Point::new(frame.width() / 2.0, frame.height() / 2.0);

                frame.fill_text(canvas::Text {
                    content: l10n.get(&format!("weekday-{}", time.weekday().number_from_monday())),
                    size: Pixels(size * 0.2),
                    position: Point::new(center.x - size * 0.02, center.y - size * 0.23),
                    color: color!(255, 0, 0),
                    align_y: alignment::Vertical::Bottom,
                    align_x: text::Alignment::Right,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: l10n.get(&format!("month-{}-short", time.month())),
                    size: Pixels(size * 0.2),
                    position: Point::new(center.x + size * 0.02, center.y - size * 0.23),
                    color: palette.danger,
                    align_y: alignment::Vertical::Bottom,
                    align_x: text::Alignment::Left,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: format!("{}", time.day()),
                    size: Pixels(size * 0.8),
                    position: Point::new(center.x, center.y + size * 0.11),
                    color: palette.text,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Center,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });
            });
        });
        vec![dynamic_layer]
    }
}

pub fn weekday_to_number(weekday: &Weekday) -> usize {
    match weekday {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        _ => 7,
    }
}
