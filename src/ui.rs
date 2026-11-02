use std::io;

use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::app::{Action, App, Mode};
use crate::project;

// Tokyo Night, the same colors as the tmux theme.
const BG: Color = Color::Rgb(0x1a, 0x1b, 0x26);
const BG_HIGHLIGHT: Color = Color::Rgb(0x29, 0x2e, 0x42);
const FG: Color = Color::Rgb(0xa9, 0xb1, 0xd6);
const GRAY: Color = Color::Rgb(0x56, 0x5f, 0x89);
const BLUE: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const CYAN: Color = Color::Rgb(0x7d, 0xcf, 0xff);
const GREEN: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
const YELLOW: Color = Color::Rgb(0xe0, 0xaf, 0x68);
const PURPLE: Color = Color::Rgb(0xbb, 0x9a, 0xf7);
const RED: Color = Color::Rgb(0xff, 0x75, 0x7f);

/// Shows the todos full screen (or in the tmux popup) until the user closes it.
pub fn show(mut app: App) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    // Always give the terminal back, even if `run` failed.
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    // Remembers the scroll position between frames.
    let mut list_state = ListState::default();
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app, &mut list_state))?;
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &App, list_state: &mut ListState) {
    frame.render_widget(Block::new().style(Style::new().bg(BG).fg(FG)), frame.area());

    let input_height = match app.mode {
        Mode::Add(_) | Mode::Edit(_) => 3,
        _ => 0,
    };
    let footer = footer(app);
    let footer_height = u16::try_from(footer.height()).unwrap_or(1);
    let [list_area, input_area, footer_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(input_height),
        Constraint::Length(footer_height),
    ])
    .areas(frame.area());

    draw_list(frame, app, list_state, list_area);
    match &app.mode {
        Mode::Add(text) => draw_input(frame, " New todo ", text, input_area),
        Mode::Edit(text) => draw_input(frame, " Edit todo ", text, input_area),
        _ => {}
    }
    frame.render_widget(footer, footer_area);
}

fn draw_list(frame: &mut Frame, app: &App, list_state: &mut ListState, area: Rect) {
    let progress = format!(" {}/{} done ", app.done_count(), app.todos().len());
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(BLUE)
        .title(title(app))
        .title(Line::from(progress).fg(GRAY).right_aligned());

    if app.todos().is_empty() {
        let mut hint = vec![
            Span::raw(" No todos here yet. Press "),
            Span::raw("a").fg(CYAN).bold(),
            Span::raw(" to add one"),
        ];
        if app.project.is_some() {
            hint.extend([
                Span::raw(", "),
                Span::raw("tab").fg(CYAN).bold(),
                Span::raw(" to switch list"),
            ]);
        }
        hint.push(Span::raw("."));
        let hint = Paragraph::new(Line::from(hint).fg(GRAY)).block(block);
        frame.render_widget(hint, area);
        return;
    }

    let items = app.todos().iter().map(|todo| {
        let line = if todo.done {
            Line::from(vec![
                Span::raw("✓ ").fg(GREEN),
                Span::raw(&todo.text).fg(GRAY).crossed_out(),
            ])
        } else {
            Line::from(vec![
                Span::raw("○ ").fg(YELLOW),
                Span::raw(&todo.text).fg(FG),
            ])
        };
        ListItem::new(line)
    });
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().bg(BG_HIGHLIGHT).fg(BLUE).bold())
        .highlight_symbol("▌")
        .highlight_spacing(HighlightSpacing::Always);

    list_state.select(Some(app.selected));
    frame.render_stateful_widget(list, area, list_state);
}

fn draw_input(frame: &mut Frame, title: &str, text: &str, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(YELLOW)
        .title(Line::from(title).fg(YELLOW).bold());
    let inner = block.inner(area);

    // When the text is wider than the box, scroll so its end stays visible.
    let text_width = u16::try_from(Span::raw(text).width()).unwrap_or(u16::MAX);
    let scroll = (text_width + 1).saturating_sub(inner.width);

    let input = Paragraph::new(text).fg(FG).scroll((0, scroll)).block(block);
    frame.render_widget(input, area);
    frame.set_cursor_position(Position::new(inner.x + text_width - scroll, inner.y));
}

/// ` popdo │ global `: the list on screen is bright, the other one is gray.
fn title(app: &App) -> Line<'static> {
    let Some(project) = &app.project else {
        return Line::from(" Todo ").fg(PURPLE).bold();
    };
    let tab = |name: String, active: bool| {
        if active {
            Span::raw(name).fg(PURPLE).bold()
        } else {
            Span::raw(name).fg(GRAY)
        }
    };
    Line::from(vec![
        Span::raw(" "),
        tab(project::list_name(Some(project)), !app.show_global),
        Span::raw(" │ ").fg(GRAY),
        tab("global".to_string(), app.show_global),
        Span::raw(" "),
    ])
}

fn footer(app: &App) -> Text<'static> {
    if let Some(error) = &app.error {
        return Line::from(format!(" {error}")).fg(RED).bold().into();
    }
    match &app.mode {
        Mode::Normal => {
            let mut moves = vec![("j/k", "move"), ("J/K", "reorder")];
            if app.project.is_some() {
                moves.push(("tab", "switch list"));
            }
            moves.push(("q", "close"));
            Text::from(vec![
                keys_help(&[
                    ("a", "add"),
                    ("e", "edit"),
                    ("space", "done"),
                    ("d", "delete"),
                    ("D", "clear done"),
                ]),
                keys_help(&moves),
            ])
        }
        Mode::Add(_) | Mode::Edit(_) => keys_help(&[("enter", "save"), ("esc", "cancel")]).into(),
        Mode::Confirm(action) => {
            let question = match action {
                Action::DeleteSelected => {
                    let text = app.selected_todo().map_or("", |todo| todo.text.as_str());
                    format!(" Delete \"{text}\"? ")
                }
                Action::ClearDone => match app.done_count() {
                    1 => " Delete the done todo? ".to_string(),
                    n => format!(" Delete the {n} done todos? "),
                },
            };
            Line::from(vec![
                Span::raw(question).fg(RED).bold(),
                Span::raw("y").fg(CYAN).bold(),
                Span::raw(" yes  ").fg(GRAY),
                Span::raw("any key").fg(CYAN).bold(),
                Span::raw(" no").fg(GRAY),
            ])
            .into()
        }
    }
}

/// Builds a line like ` a add  e edit`, with keys in cyan.
fn keys_help(keys: &[(&'static str, &'static str)]) -> Line<'static> {
    keys.iter()
        .flat_map(|&(key, label)| {
            [
                Span::raw(" "),
                Span::raw(key).fg(CYAN).bold(),
                Span::raw(format!(" {label} ")).fg(GRAY),
            ]
        })
        .collect()
}
