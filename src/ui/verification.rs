use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::{Message, VerificationInfo, VerificationPhase};

pub fn verification_panel<'a>(info: &'a VerificationInfo) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let content: Element<'a, Message> = match &info.phase {
        VerificationPhase::WaitingForAccept => widget::column()
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .push(widget::text::title3("Waiting for other device…"))
            .push(widget::text::body(format!(
                "Verify with {}",
                info.other_user_id
            )))
            .push(
                widget::button::text("Cancel")
                    .on_press(Message::CancelVerification)
                    .class(cosmic::theme::Button::Destructive),
            )
            .into(),

        VerificationPhase::SasStarted => widget::column()
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .push(widget::text::title3("Exchanging keys…"))
            .push(widget::text::body("Please wait"))
            .push(
                widget::button::text("Cancel")
                    .on_press(Message::CancelVerification)
                    .class(cosmic::theme::Button::Destructive),
            )
            .into(),

        VerificationPhase::ShowingEmoji(emojis) => {
            let mut emoji_row = widget::row()
                .spacing(spacing.space_s)
                .align_y(Alignment::Center);

            for (symbol, description) in emojis {
                let cell = widget::column()
                    .spacing(4)
                    .align_x(Alignment::Center)
                    .width(Length::Fixed(80.0))
                    .push(widget::text::title2(symbol.clone()))
                    .push(widget::text::caption(description.clone()));
                emoji_row = emoji_row.push(cell);
            }

            widget::column()
                .spacing(spacing.space_m)
                .align_x(Alignment::Center)
                .push(widget::text::title3("Compare emoji"))
                .push(widget::text::body(
                    "Do both devices show the same emoji?",
                ))
                .push(emoji_row)
                .push(
                    widget::row()
                        .spacing(spacing.space_s)
                        .push(
                            widget::button::text("They Match")
                                .on_press(Message::VerificationConfirm)
                                .class(cosmic::theme::Button::Suggested),
                        )
                        .push(
                            widget::button::text("No Match")
                                .on_press(Message::VerificationMismatch)
                                .class(cosmic::theme::Button::Destructive),
                        )
                        .push(
                            widget::button::text("Cancel")
                                .on_press(Message::CancelVerification),
                        ),
                )
                .into()
        }

        VerificationPhase::Confirming => widget::column()
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .push(widget::text::title3("Confirming…"))
            .push(widget::text::body("Please wait while verification completes"))
            .into(),

        VerificationPhase::Done => widget::column()
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .push(widget::text::title3("Verification complete"))
            .push(widget::text::body("Your identity has been verified."))
            .into(),

        VerificationPhase::Cancelled(reason) => widget::column()
            .spacing(spacing.space_m)
            .align_x(Alignment::Center)
            .push(widget::text::title3("Verification cancelled"))
            .push(widget::text::body(format!("Reason: {reason}")))
            .into(),
    };

    widget::container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .padding(spacing.space_l)
        .into()
}

pub fn incoming_verification_banner<'a>(sender: &'a str) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    widget::container(
        widget::row()
            .spacing(spacing.space_s)
            .align_y(Alignment::Center)
            .push(widget::text::body(format!(
                "Device from {sender} wants to verify"
            )))
            .push(widget::horizontal_space())
            .push(
                widget::button::text("Accept")
                    .on_press(Message::AcceptVerification)
                    .class(cosmic::theme::Button::Suggested),
            )
            .push(
                widget::button::text("Ignore")
                    .on_press(Message::IgnoreVerification),
            ),
    )
    .padding([spacing.space_xxs, spacing.space_s])
    .width(Length::Fill)
    .into()
}
