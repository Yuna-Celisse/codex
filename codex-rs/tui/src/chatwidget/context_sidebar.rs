//! Responsive context sidebar shared by inline and full-screen chat surfaces.
//!
//! Reserve room only on wide terminals. The breakpoint applies to the terminal,
//! not the already reduced chat area, so wrapping and cursor geometry agree.

use super::*;
use codex_protocol::num_format::format_with_separators;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::widgets::Wrap;

pub(crate) const CONTEXT_SIDEBAR_MIN_WIDTH: u16 = 120;
const SIDEBAR_WIDTH: u16 = 32;
const MIN_HEIGHT: u16 = 15;

pub(crate) fn split_context_area(area: Rect, enabled: bool) -> (Rect, Option<Rect>) {
    if !enabled || area.width < CONTEXT_SIDEBAR_MIN_WIDTH {
        return (area, None);
    }
    let chat = Rect::new(area.x, area.y, area.width - SIDEBAR_WIDTH, area.height);
    let sidebar = Rect::new(chat.right(), area.y, SIDEBAR_WIDTH, area.height);
    (chat, Some(sidebar))
}

impl ChatWidget {
    pub(crate) fn context_sidebar_enabled(&self) -> bool {
        self.bottom_pane
            .selected_index_for_present_view(crate::app::AGENTS_OVERVIEW_VIEW_ID)
            .is_none()
    }

    pub(crate) fn context_chat_width(&self, width: u16) -> u16 {
        split_context_area(Rect::new(0, 0, width, 0), self.context_sidebar_enabled())
            .0
            .width
    }

    pub(crate) fn render_context_sidebar(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        Widget::render(block, area, buf);
        Widget::render(
            Paragraph::new(self.context_sidebar_lines()).wrap(Wrap { trim: false }),
            inner,
            buf,
        );
    }

    fn context_sidebar_lines(&self) -> Vec<Line<'static>> {
        let usage = (!self.token_usage_pending)
            .then_some(self.token_info.as_ref())
            .flatten();
        let last = usage.map(|info| &info.last_token_usage);
        let window = self.status_line_context_window_size();
        let number = |value: Option<i64>| {
            value
                .map(format_with_separators)
                .unwrap_or_else(|| "—".to_string())
        };
        let mut lines = vec![
            Line::from("Context".bold()),
            Line::default(),
            Line::from(format!("{} tokens", number(last.map(|u| u.total_tokens)))),
        ];
        if let (Some(last), Some(window)) = (last, window.filter(|window| *window > 0)) {
            let percent = (last.total_tokens.max(0) as f64 / window as f64) * 100.0;
            lines.push(Line::from(format!(
                "{percent:.1}% of {}",
                number(Some(window))
            )));
        } else {
            lines.push(Line::from(format!("Window: {}", number(window))).dim());
        }
        lines.extend([
            Line::default(),
            Line::from(format!("Input: {}", number(last.map(|u| u.input_tokens)))),
            Line::from(format!(
                "Cached: {}",
                number(last.map(|u| u.cached_input_tokens))
            )),
            Line::from(format!("Output: {}", number(last.map(|u| u.output_tokens)))),
            Line::from(format!(
                "Reasoning: {}",
                number(last.map(|u| u.reasoning_output_tokens))
            )),
            Line::from(format!(
                "Session total: {}",
                number(usage.map(|u| u.total_token_usage.total_tokens))
            ))
            .dim(),
            Line::default(),
            Line::from("Directory".bold()),
            Line::from(self.config.cwd.to_string_lossy().into_owned()).dim(),
        ]);
        // This fork only ever speaks to an external server; showing which one (and its
        // version) is the visible proof that the embedded path is not in use.
        if let Some(connection) = self.remote_connection.as_ref() {
            lines.extend([
                Line::default(),
                Line::from("Backend".bold()),
                Line::from(format!(
                    "{} {}",
                    if connection.is_local_daemon {
                        "daemon"
                    } else {
                        "remote"
                    },
                    connection.version
                ))
                .dim(),
            ]);
        }
        self.context_sidebar_plan_lines(&mut lines);
        lines
    }

    /// opencode-style checklist of the live `update_plan` progress, below Directory so a
    /// long plan clips itself instead of pushing the static sections out of view.
    fn context_sidebar_plan_lines(&self, lines: &mut Vec<Line<'static>>) {
        let Some(plan) = self
            .transcript
            .last_plan
            .as_deref()
            .filter(|plan| !plan.is_empty())
        else {
            return;
        };
        let completed = plan
            .iter()
            .filter(|item| matches!(item.status, UpdatePlanItemStatus::Completed))
            .count();
        lines.extend([
            Line::default(),
            Line::from("Plan".bold()),
            Line::from(format!("{completed}/{} done", plan.len())).dim(),
        ]);
        for item in plan {
            let style = match item.status {
                UpdatePlanItemStatus::Completed => Style::default().dim(),
                UpdatePlanItemStatus::InProgress => Style::default()
                    .fg(crate::style::accent_color())
                    .bold(),
                UpdatePlanItemStatus::Pending => Style::default(),
            };
            let icon = match item.status {
                UpdatePlanItemStatus::Completed => "✓ ",
                UpdatePlanItemStatus::InProgress => "● ",
                UpdatePlanItemStatus::Pending => "○ ",
            };
            lines.push(Line::from(vec![
                ratatui::text::Span::styled(icon, style),
                ratatui::text::Span::styled(
                    item.step.split_whitespace().collect::<Vec<_>>().join(" "),
                    style,
                ),
            ]));
        }
    }

    pub(super) fn with_context_sidebar<'a>(
        &'a self,
        child: RenderableItem<'a>,
    ) -> RenderableItem<'a> {
        RenderableItem::Owned(Box::new(ContextSidebarRenderable { chat: self, child }))
    }
}

struct ContextSidebarRenderable<'a> {
    chat: &'a ChatWidget,
    child: RenderableItem<'a>,
}

impl Renderable for ContextSidebarRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let (chat, sidebar) = split_context_area(area, self.chat.context_sidebar_enabled());
        self.child.render(chat, buf);
        if let Some(sidebar) = sidebar {
            self.chat.render_context_sidebar(sidebar, buf);
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        let height = self
            .child
            .desired_height(self.chat.context_chat_width(width));
        if self.chat.context_chat_width(width) < width {
            height.max(MIN_HEIGHT)
        } else {
            height
        }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.child
            .cursor_pos(split_context_area(area, self.chat.context_sidebar_enabled()).0)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.child
            .cursor_style(split_context_area(area, self.chat.context_sidebar_enabled()).0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatwidget::tests::make_chatwidget_manual_with_sender;

    #[test]
    fn sidebar_breakpoint_preserves_area_and_chat_width() {
        let narrow = Rect::new(3, 5, 119, 24);
        assert_eq!(split_context_area(narrow, true), (narrow, None));
        let wide = Rect::new(3, 5, 120, 24);
        let (chat, sidebar) = split_context_area(wide, true);
        let sidebar = sidebar.unwrap();
        assert_eq!(chat.width, 88);
        assert_eq!(sidebar.width, 32);
        assert_eq!(chat.right(), sidebar.x);
        assert_eq!(sidebar.right(), wide.right());
        assert_eq!(split_context_area(wide, false), (wide, None));
    }

    #[tokio::test]
    async fn resize_hides_sidebar_without_losing_draft_and_usage_is_not_cumulative() {
        let (mut chat, _, _, _) = make_chatwidget_manual_with_sender().await;
        chat.handle_paste("draft survives resizing".to_string());
        let usage = TokenUsageInfo {
            total_token_usage: TokenUsage {
                total_tokens: 900_000,
                ..TokenUsage::default()
            },
            last_token_usage: TokenUsage {
                total_tokens: 24_000,
                ..TokenUsage::default()
            },
            model_context_window: Some(100_000),
        };
        chat.set_token_info(Some(usage));
        for width in [120, 119, 160, 80] {
            let area = Rect::new(0, 0, width, 24);
            let mut buffer = Buffer::empty(area);
            chat.as_renderable().render(area, &mut buffer);
            let text = buffer
                .content
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>();
            assert!(text.contains("draft survives resizing"));
            assert_eq!(text.contains("Context"), width >= CONTEXT_SIDEBAR_MIN_WIDTH);
            if width >= CONTEXT_SIDEBAR_MIN_WIDTH {
                assert!(text.contains("24,000 tokens"));
                assert!(text.contains("24.0% of 100,000"));
                assert!(text.contains("Session total: 900,000"));
            }
            assert!(chat.as_renderable().cursor_pos(area).is_some());
        }
    }

    #[tokio::test]
    async fn backend_line_names_the_connected_server() {
        let (mut chat, _, _, _) = make_chatwidget_manual_with_sender().await;
        chat.remote_connection = Some(crate::status::remote_connection::RemoteConnectionStatus {
            address: "unix:///tmp/app-server-control.sock".to_string(),
            version: "v0.162.1".to_string(),
            is_local_daemon: true,
        });
        let area = Rect::new(0, 0, 140, 30);
        let mut buffer = Buffer::empty(area);
        chat.as_renderable().render(area, &mut buffer);
        let text: String = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Backend"), "{text}");
        assert!(text.contains("daemon v0.162.1"), "{text}");
    }

    #[tokio::test]
    async fn plan_checklist_appears_only_after_a_plan_update() {
        let (mut chat, _, _, _) = make_chatwidget_manual_with_sender().await;
        let area = Rect::new(0, 0, 140, 30);
        let render = |chat: &mut ChatWidget| {
            let mut buffer = Buffer::empty(area);
            chat.as_renderable().render(area, &mut buffer);
            buffer
                .content
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
        };
        assert!(!render(&mut chat).contains("done"));
        chat.on_plan_update(UpdatePlanArgs {
            explanation: None,
            plan: vec![
                UpdatePlanItemArg {
                    step: "Wire the protocol".to_string(),
                    status: UpdatePlanItemStatus::Completed,
                },
                UpdatePlanItemArg {
                    step: "Render   the checklist".to_string(),
                    status: UpdatePlanItemStatus::InProgress,
                },
                UpdatePlanItemArg {
                    step: "Ship it".to_string(),
                    status: UpdatePlanItemStatus::Pending,
                },
            ],
        });
        let text = render(&mut chat);
        assert!(text.contains("Plan"), "{text}");
        assert!(text.contains("1/3 done"), "{text}");
        assert!(text.contains("✓ Wire the protocol"), "{text}");
        assert!(text.contains("● Render the checklist"), "{text}");
        assert!(text.contains("○ Ship it"), "{text}");
    }
}
