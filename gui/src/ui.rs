//! View layer — a halloy-style 3-pane layout (buffers · chat · nicks) rendered
//! in FiraCode.

use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Color, Element, Length};

use crate::app::{App, Message};
use crate::state::{Buffer, BufferKind, Line, LineKind};
use crate::theme;

pub fn view(app: &App) -> Element<'_, Message> {
    let body = row![sidebar(app), chat_pane(app)]
        .spacing(0)
        .height(Length::Fill);

    // Nick list only for channel buffers.
    let body = if app.buffers[app.active].kind == BufferKind::Channel {
        body.push(nick_pane(&app.buffers[app.active]))
    } else {
        body
    };

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BG.into()),
            ..container::Style::default()
        })
        .into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let mut items: Vec<Element<Message>> = Vec::with_capacity(app.buffers.len() + 1);
    items.push(
        container(text("REPARTEE").size(15).font(theme::bold()).color(theme::ACCENT))
            .padding([10, 12])
            .into(),
    );

    for (i, buf) in app.buffers.iter().enumerate() {
        let label = match buf.kind {
            BufferKind::Server => "● server".to_string(),
            BufferKind::Channel => buf.name.clone(),
            BufferKind::Query => format!("@ {}", buf.name),
        };
        let active = i == app.active;
        let btn = button(text(label).size(14).font(theme::font()).color(if active {
            theme::TEXT
        } else {
            theme::DIM
        }))
        .width(Length::Fill)
        .padding([5, 12])
        .on_press(Message::SelectBuffer(i))
        .style(move |_, _| button::Style {
            background: Some(if active { theme::BG_ACTIVE } else { Color::TRANSPARENT }.into()),
            text_color: if active { theme::TEXT } else { theme::DIM },
            border: iced::Border {
                radius: 4.0.into(),
                ..iced::Border::default()
            },
            ..button::Style::default()
        });
        items.push(btn.into());
    }

    container(column(items).spacing(1).width(Length::Fixed(200.0)))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BG_PANEL.into()),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn chat_pane(app: &App) -> Element<'_, Message> {
    let buf = &app.buffers[app.active];

    let header = container(
        text(if buf.topic.is_empty() {
            buf.name.clone()
        } else {
            format!("{}  —  {}", buf.name, buf.topic)
        })
        .size(13)
        .color(theme::DIM)
        .font(theme::font()),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::BG_PANEL.into()),
        border: iced::Border {
            color: theme::BORDER,
            width: 1.0,
            ..iced::Border::default()
        },
        ..container::Style::default()
    });

    let lines: Vec<Element<Message>> = buf.lines.iter().map(line_view).collect();
    let log = scrollable(column(lines).spacing(2).padding([8, 12]).width(Length::Fill))
        .id(app.log_id.clone())
        .height(Length::Fill)
        .width(Length::Fill);

    let input = text_input(
        if buf.kind == BufferKind::Server {
            "type /join #channel to start…"
        } else {
            "message…"
        },
        &app.input,
    )
    .on_input(Message::InputChanged)
    .on_submit(Message::Submit)
    .padding([9, 12])
    .size(14)
    .font(theme::font());

    column![header, log, container(input).padding(8)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn line_view(line: &Line) -> Element<'_, Message> {
    let ts = text(line.time.format("%H:%M").to_string())
        .size(12)
        .color(theme::DIM)
        .font(theme::font());

    let content: Element<Message> = match line.kind {
        LineKind::Message => {
            let nick = line.nick.clone().unwrap_or_default();
            let color = theme::nick_for(&nick);
            row![
                text(format!("{nick} "))
                    .size(14)
                    .color(color)
                    .font(theme::bold()),
                text(line.text.clone())
                    .size(14)
                    .color(theme::TEXT)
                    .font(theme::font())
                    .width(Length::Fill),
            ]
            .into()
        }
        LineKind::Action => {
            let nick = line.nick.clone().unwrap_or_default();
            let color = theme::nick_for(&nick);
            text(format!("* {nick} {}", line.text))
                .size(14)
                .color(color)
                .font(theme::font())
                .width(Length::Fill)
                .into()
        }
        LineKind::Notice => {
            let from = line.nick.clone().unwrap_or_else(|| "notice".to_string());
            text(format!("-{from}- {}", line.text))
                .size(14)
                .color(theme::ACCENT)
                .font(theme::font())
                .width(Length::Fill)
                .into()
        }
        LineKind::Event => text(line.text.clone())
            .size(13)
            .color(theme::DIM)
            .font(theme::font())
            .width(Length::Fill)
            .into(),
    };

    row![ts, Space::with_width(Length::Fixed(8.0)), content]
        .align_y(Alignment::Start)
        .width(Length::Fill)
        .into()
}

fn nick_pane(buf: &Buffer) -> Element<'_, Message> {
    let mut items: Vec<Element<Message>> = Vec::with_capacity(buf.users.len() + 1);
    items.push(
        container(
            text(format!("{} users", buf.users.len()))
                .size(12)
                .color(theme::DIM)
                .font(theme::bold()),
        )
        .padding([8, 10])
        .into(),
    );
    for u in &buf.users {
        let symbol = u.prefix.chars().next().unwrap_or(' ');
        let label = if symbol == ' ' {
            u.nick.clone()
        } else {
            format!("{symbol}{}", u.nick)
        };
        items.push(
            container(
                text(label)
                    .size(13)
                    .color(theme::nick_for(&u.nick))
                    .font(theme::font()),
            )
            .padding([1, 10])
            .into(),
        );
    }

    container(scrollable(column(items).spacing(1)).height(Length::Fill))
        .width(Length::Fixed(170.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::BG_PANEL.into()),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                ..iced::Border::default()
            },
            ..container::Style::default()
        })
        .into()
}
