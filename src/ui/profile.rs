use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;

pub fn profile_panel_view<'a>(
    own_user_id: &'a str,
    own_avatar: Option<&'a ImageHandle>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let mut col = widget::column()
        .spacing(spacing.space_m)
        .align_x(Alignment::Center);

    col = col.push(widget::text::heading("Profile"));

    // Avatar display
    let avatar_elem: Element<_> = if let Some(handle) = own_avatar {
        cosmic::iced::widget::image(handle.clone())
            .content_fit(ContentFit::Cover)
            .width(Length::Fixed(96.0))
            .height(Length::Fixed(96.0))
            .into()
    } else {
        let initial = own_user_id
            .strip_prefix('@')
            .and_then(|s| s.split(':').next())
            .and_then(|s| s.chars().next())
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        widget::container(widget::text::heading(initial))
            .width(Length::Fixed(96.0))
            .height(Length::Fixed(96.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    };

    col = col.push(widget::container(avatar_elem));

    col = col.push(widget::text::body(own_user_id.to_string()));

    // Buttons
    col = col.push(
        widget::button::text("Change avatar")
            .on_press(Message::PickAvatar),
    );
    col = col.push(
        widget::button::text("Clear avatar")
            .on_press(Message::ClearAvatar),
    );
    col = col.push(
        widget::button::text("Close")
            .on_press(Message::CloseProfilePanel),
    );

    widget::container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .class(cosmic::theme::Container::Background)
        .into()
}
