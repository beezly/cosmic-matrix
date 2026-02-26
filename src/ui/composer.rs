use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;
use crate::state::timeline::TimelineState;

pub fn composer_view<'a>(state: &'a TimelineState) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let input = widget::text_input::text_input("Send a message...", &state.composer)
        .on_input(Message::ComposerChanged)
        .on_submit(|_| Message::SendMessage);

    let mut send_btn = widget::button::suggested("Send");
    if !state.composer.trim().is_empty() && !state.sending {
        send_btn = send_btn.on_press(Message::SendMessage);
    }

    let attach_label = if state.attachment_sending {
        "…"
    } else {
        "📎"
    };
    let mut attach_btn = widget::button::text(attach_label);
    if !state.attachment_sending {
        attach_btn = attach_btn.on_press(Message::PickAttachment);
    }

    let mut col = widget::column().spacing(spacing.space_xxs);

    if let Some(ref ctx) = state.reply_to {
        let preview = if ctx.body_preview.is_empty() {
            String::new()
        } else {
            format!(": {}", ctx.body_preview)
        };
        col = col.push(
            widget::container(
                widget::row()
                    .push(widget::text::caption(
                        format!("↩ Replying to {}{}", ctx.sender_display, preview),
                    ))
                    .push(widget::horizontal_space())
                    .push(
                        widget::button::text("×")
                            .on_press(Message::CancelReply)
                            .padding([0, spacing.space_xxs]),
                    )
                    .align_y(Alignment::Center),
            )
            .padding([spacing.space_xxs, spacing.space_xs])
            .width(Length::Fill),
        );
    }

    col = col.push(
        widget::row()
            .push(attach_btn)
            .push(input)
            .push(send_btn)
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center),
    );

    widget::container(col)
        .padding(spacing.space_xs)
        .width(Length::Fill)
        .into()
}
