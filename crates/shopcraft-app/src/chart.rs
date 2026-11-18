//! Sales chart for one shop. Bars when the span is short, an area when it is long.

use craft_biz::Money;
use craft_biz_ui::app::Frame;
use craft_biz_ui::theme::{self, Tone};
use craft_biz_ui::widgets::{self, Kpi};
use egui::{
    epaint::Vertex, Align2, Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Sense, Stroke, Ui,
    Vec2,
};
use shopcraft_core::sales::SalesPoint;

pub fn show(ui: &mut Ui, f: &Frame, points: &[SalesPoint], caption: &str) {
    ui.heading(f.t("Umsatz", "Sales"));
    ui.label(caption);
    ui.add_space(8.0);
    if points.is_empty() {
        widgets::empty_state(
            ui,
            f.t(
                "Noch kein Umsatz in diesem Buch",
                "No sales in this book yet",
            ),
            f.t(
                "Demo laden, dann verkaufen oder bestellen. Diese Seite liest nur diesen Laden.",
                "Seed the demo, then sell or place an order. This page reads only this shop.",
            ),
        );
        return;
    }
    let total: i64 = points.iter().map(|point| point.cents).sum();
    let orders: i64 = points.iter().map(|point| point.orders).sum();
    let days_with_sales = points.iter().filter(|point| point.orders > 0).count();
    let best = points.iter().max_by_key(|point| point.cents);
    widgets::kpi_row(
        ui,
        &[
            Kpi::new(f.t("Summe", "Total"), Money::from_cents(total).to_string()),
            Kpi::new(f.t("Vorgänge", "Orders"), orders.to_string()),
            Kpi::new(
                f.t("Tage mit Umsatz", "Days with sales"),
                days_with_sales.to_string(),
            ),
            Kpi::new(
                f.t("Bester Tag", "Best day"),
                best.map(|point| Money::from_cents(point.cents).to_string())
                    .unwrap_or_else(|| "0".into()),
            ),
        ],
    );
    ui.add_space(8.0);
    let height = (ui.available_height() - 12.0).clamp(260.0, 440.0);
    plot(ui, f, points, height);
}

fn plot(ui: &mut Ui, f: &Frame, points: &[SalesPoint], height: f32) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let tokens = theme::current(ui.ctx());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(12), tokens.surface);
    painter.rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, tokens.border),
        egui::StrokeKind::Inside,
    );

    let plot = rect.shrink2(Vec2::new(72.0, 16.0));
    let plot = Rect::from_min_max(
        Pos2::new(plot.left(), plot.top() + 8.0),
        Pos2::new(plot.right() - 8.0, plot.bottom() - 28.0),
    );
    painter.rect_filled(plot, CornerRadius::same(8), tokens.sunken);

    let min_c = points
        .iter()
        .map(|point| point.cents)
        .min()
        .unwrap_or(0)
        .min(0);
    let max_c = points
        .iter()
        .map(|point| point.cents)
        .max()
        .unwrap_or(0)
        .max(0)
        .max(min_c + 1);
    let span = (max_c - min_c) as f32;
    let y_of = |cents: i64| {
        let t = (cents - min_c) as f32 / span;
        plot.bottom() - t * plot.height()
    };
    let zero_y = y_of(0);

    for step in 0..=4 {
        let cents = min_c + (max_c - min_c) * i64::from(step) / 4;
        let y = y_of(cents);
        painter.hline(
            plot.x_range(),
            y,
            Stroke::new(1.0, tokens.border.gamma_multiply(0.65)),
        );
        painter.text(
            Pos2::new(rect.left() + 10.0, y),
            Align2::LEFT_CENTER,
            Money::from_cents(cents).to_string(),
            FontId::proportional(11.0),
            tokens.text_weak,
        );
    }
    painter.hline(
        plot.x_range(),
        zero_y,
        Stroke::new(1.2, tokens.border_strong),
    );

    let line = theme::tone_solid(Tone::Teal, tokens.dark);
    let fill = Color32::from_rgba_unmultiplied(line.r(), line.g(), line.b(), 56);
    let danger = theme::tone_solid(Tone::Danger, tokens.dark);
    let n = points.len();
    let x_of = |index: usize| {
        if n == 1 {
            plot.center().x
        } else {
            plot.left() + plot.width() * index as f32 / (n - 1) as f32
        }
    };

    let hover = response.hover_pos().and_then(|pos| {
        if !plot.contains(pos) || n == 0 {
            return None;
        }
        let t = ((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0);
        let index = if n == 1 {
            0
        } else {
            (t * (n - 1) as f32).round() as usize
        };
        points.get(index).map(|point| (index, point))
    });

    if n >= 24 {
        let mut mesh = Mesh::default();
        for index in 0..n.saturating_sub(1) {
            let x0 = x_of(index);
            let x1 = x_of(index + 1);
            let y0 = y_of(points[index].cents.max(0));
            let y1 = y_of(points[index + 1].cents.max(0));
            let base = mesh.vertices.len() as u32;
            for (pos, color) in [
                (Pos2::new(x0, zero_y), Color32::TRANSPARENT),
                (Pos2::new(x0, y0), fill),
                (Pos2::new(x1, y1), fill),
                (Pos2::new(x1, zero_y), Color32::TRANSPARENT),
            ] {
                mesh.vertices.push(Vertex {
                    pos,
                    uv: Pos2::ZERO,
                    color,
                });
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        painter.add(egui::Shape::mesh(mesh));
        let pts: Vec<Pos2> = points
            .iter()
            .enumerate()
            .map(|(index, point)| Pos2::new(x_of(index), y_of(point.cents)))
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(2.4, line)));
    } else {
        let slot = plot.width() / n as f32;
        let bar_w = (slot * 0.62).clamp(4.0, 36.0);
        for (index, point) in points.iter().enumerate() {
            let x = if n == 1 {
                plot.center().x
            } else {
                plot.left() + slot * (index as f32 + 0.5)
            };
            let y = y_of(point.cents);
            let color = if point.cents < 0 { danger } else { line };
            let hovered = hover.is_some_and(|(hit, _)| hit == index);
            let color = if hovered {
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 255)
            } else if point.cents == 0 {
                tokens.border
            } else {
                color
            };
            let top = y.min(zero_y);
            let bottom = y.max(zero_y).max(top + 1.0);
            let bar = Rect::from_min_max(
                Pos2::new(x - bar_w * 0.5, top),
                Pos2::new(x + bar_w * 0.5, bottom),
            );
            painter.rect_filled(bar, CornerRadius::same(4), color);
        }
    }

    let label_every = if n <= 8 { 1 } else { (n / 6).max(1) };
    for (index, point) in points.iter().enumerate() {
        if index % label_every != 0 && index + 1 != n {
            continue;
        }
        painter.text(
            Pos2::new(x_of(index), plot.bottom() + 14.0),
            Align2::CENTER_TOP,
            short_day(&point.day),
            FontId::proportional(11.0),
            tokens.text_weak,
        );
    }

    if let Some((index, point)) = hover {
        let x = x_of(index);
        painter.vline(x, plot.y_range(), Stroke::new(1.0, tokens.border_strong));
        painter.circle_filled(Pos2::new(x, y_of(point.cents)), 4.0, line);
        let text = format!(
            "{}\n{}\n{} {}",
            point.day,
            Money::from_cents(point.cents),
            point.orders,
            f.t("Vorgänge", "orders")
        );
        let galley = painter.layout_no_wrap(text, FontId::proportional(13.0), tokens.text);
        let mut tip = Rect::from_min_size(
            Pos2::new(x + 10.0, plot.top() + 8.0),
            galley.size() + Vec2::splat(16.0),
        );
        if tip.right() > rect.right() - 8.0 {
            tip = tip.translate(Vec2::new(rect.right() - 8.0 - tip.right(), 0.0));
        }
        painter.rect_filled(tip, CornerRadius::same(8), tokens.surface);
        painter.rect_stroke(
            tip,
            CornerRadius::same(8),
            Stroke::new(1.0, tokens.border),
            egui::StrokeKind::Inside,
        );
        painter.galley(tip.left_top() + Vec2::splat(8.0), galley, tokens.text);
    }
}

fn short_day(day: &str) -> String {
    if day.len() >= 10 {
        day[5..10].to_string()
    } else {
        day.to_string()
    }
}
