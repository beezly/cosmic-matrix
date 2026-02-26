use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;

pub fn room_header_view<'a>(
    room_name: &'a str,
    is_encrypted: bool,
    topic: Option<&'a str>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let mut row = widget::row()
        .spacing(spacing.space_xs)
        .align_y(Alignment::Center);

    row = row.push(widget::text::title4(room_name.to_string()));

    if is_encrypted {
        row = row.push(widget::text::caption("Encrypted"));
    }

    let mut col = widget::column().spacing(2);
    col = col.push(row);

    if let Some(topic) = topic {
        if !topic.is_empty() {
            col = col.push(widget::text::caption(topic.to_string()));
        }
    }

    widget::container(col)
        .padding([spacing.space_xxs, spacing.space_s])
        .width(Length::Fill)
        .into()
}
