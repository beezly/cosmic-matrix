use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use crate::message::Message;

pub struct LoginState {
    pub homeserver: String,
    pub username: String,
    pub password: String,
    pub password_visible: bool,
    pub error: Option<String>,
    pub loading: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            homeserver: "matrix.org".to_string(),
            username: String::new(),
            password: String::new(),
            password_visible: false,
            error: None,
            loading: false,
        }
    }
}

pub fn login_view(state: &LoginState) -> Element<'_, Message> {
    let spacing = cosmic::theme::spacing();

    let mut form = widget::column()
        .spacing(spacing.space_s)
        .max_width(400.0)
        .align_x(Alignment::Center);

    form = form.push(widget::text::title2("Cosmic Matrix"));
    form = form.push(widget::text::body("Sign in to your Matrix account"));
    form = form.push(widget::vertical_space().height(Length::Fixed(spacing.space_m as f32)));

    // Homeserver input
    form = form.push(widget::text::caption_heading("Homeserver"));
    form = form.push(
        widget::text_input::text_input("matrix.org", &state.homeserver)
            .on_input(Message::HomeserverChanged),
    );

    // Username input
    form = form.push(widget::text::caption_heading("Username"));
    form = form.push(
        widget::text_input::text_input("@user:matrix.org", &state.username)
            .on_input(Message::UsernameChanged),
    );

    // Password input
    form = form.push(widget::text::caption_heading("Password"));
    form = form.push(
        widget::text_input::secure_input(
            "Password",
            &state.password,
            Some(Message::TogglePasswordVisibility),
            !state.password_visible,
        )
        .on_input(Message::PasswordChanged)
        .on_submit(|_| Message::LoginSubmit),
    );

    form = form.push(widget::vertical_space().height(Length::Fixed(spacing.space_s as f32)));

    // Error message
    if let Some(ref err) = state.error {
        form = form.push(widget::text::body(err.as_str()));
    }

    // Login button
    if state.loading {
        form = form.push(widget::button::suggested("Signing in...").width(Length::Fill));
    } else {
        let can_submit = !state.homeserver.is_empty()
            && !state.username.is_empty()
            && !state.password.is_empty();

        let mut btn = widget::button::suggested("Sign In").width(Length::Fill);
        if can_submit {
            btn = btn.on_press(Message::LoginSubmit);
        }
        form = form.push(btn);
    }

    widget::container(form)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}
